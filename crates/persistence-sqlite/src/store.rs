use crate::migrations::run_migrations;
use async_trait::async_trait;
use custos_core_domain::{ContinuationPacket, DomainError, Span, SpanState, Task, TaskStatus};
use custos_task_kernel::TaskStore;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub struct SqliteTaskStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskStore {
    pub fn new_in_memory() -> Result<Self, DomainError> {
        let conn =
            Connection::open_in_memory().map_err(|e| DomainError::Validation(e.to_string()))?;
        run_migrations(&conn).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new(path: &str) -> Result<Self, DomainError> {
        let conn = Connection::open(path).map_err(|e| DomainError::Validation(e.to_string()))?;
        run_migrations(&conn).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl TaskStore for SqliteTaskStore {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, status, epoch, created_at, updated_at, metadata FROM tasks WHERE id = ?1"
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        let task = stmt.query_row(params![task_id], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "draft" => TaskStatus::Draft,
                "queued" => TaskStatus::Queued,
                "running" => TaskStatus::Running,
                "blocked" => TaskStatus::Blocked,
                "succeeded" => TaskStatus::Succeeded,
                "failed" => TaskStatus::Failed,
                _ => TaskStatus::Cancelled,
            };
            let meta_str: String = row.get(6)?;
            let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));

            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                status,
                epoch: row.get::<_, i64>(3)? as u64,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                metadata,
            })
        });

        match task {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
             title=excluded.title, status=excluded.status, epoch=excluded.epoch,
             updated_at=excluded.updated_at, metadata=excluded.metadata",
            params![
                task.id,
                task.title,
                task.status.to_string(),
                task.epoch as i64,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.metadata.to_string(),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, span_num, state, provider, model, input_digest, output_digest, started_at, ended_at FROM spans WHERE id = ?1"
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        let span = stmt.query_row(params![span_id], |row| {
            let state_str: String = row.get(3)?;
            let state = match state_str.as_str() {
                "completed" => SpanState::Completed,
                "failed" => SpanState::Failed,
                _ => SpanState::Started,
            };
            Ok(Span {
                id: row.get(0)?,
                task_id: row.get(1)?,
                span_num: row.get::<_, i64>(2)? as u32,
                state,
                provider: row.get(4)?,
                model: row.get(5)?,
                input_digest: row.get(6)?,
                output_digest: row.get(7)?,
                started_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                ended_at: row.get::<_, Option<String>>(9)?.map(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&chrono::Utc)
                }),
            })
        });

        match span {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    async fn save_span(&self, span: &Span) -> Result<(), DomainError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO spans (id, task_id, span_num, state, provider, model, input_digest, output_digest, started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
             state=excluded.state, output_digest=excluded.output_digest, ended_at=excluded.ended_at",
            params![
                span.id,
                span.task_id,
                span.span_num as i64,
                span.state.to_string(),
                span.provider,
                span.model,
                span.input_digest,
                span.output_digest,
                span.started_at.to_rfc3339(),
                span.ended_at.as_ref().map(|d| d.to_rfc3339()),
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, span_num, state, provider, model, input_digest, output_digest, started_at, ended_at FROM spans WHERE task_id = ?1 ORDER BY span_num ASC"
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], |row| {
                let state_str: String = row.get(3)?;
                let state = match state_str.as_str() {
                    "completed" => SpanState::Completed,
                    "failed" => SpanState::Failed,
                    _ => SpanState::Started,
                };
                Ok(Span {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    span_num: row.get::<_, i64>(2)? as u32,
                    state,
                    provider: row.get(4)?,
                    model: row.get(5)?,
                    input_digest: row.get(6)?,
                    output_digest: row.get(7)?,
                    started_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    ended_at: row.get::<_, Option<String>>(9)?.map(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&chrono::Utc)
                    }),
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut spans = Vec::new();
        for r in rows {
            spans.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(spans)
    }

    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO continuation_packets (task_id, from_span, to_span, provider, model, task_summary, current_state, integrity_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                packet.task_id,
                packet.from_span as i64,
                packet.to_span as i64,
                packet.provider,
                packet.model,
                packet.task_summary,
                packet.current_state.to_string(),
                packet.integrity_hash,
                packet.created_at.to_rfc3339(),
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_id, from_span, to_span, provider, model, task_summary, current_state, integrity_hash, created_at
             FROM continuation_packets WHERE task_id = ?1 ORDER BY to_span DESC LIMIT 1"
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        let packet = stmt.query_row(params![task_id], |row| {
            let state_str: String = row.get(6)?;
            let current_state = serde_json::from_str(&state_str).unwrap_or(serde_json::json!({}));
            Ok(ContinuationPacket {
                task_id: row.get(0)?,
                from_span: row.get::<_, i64>(1)? as u32,
                to_span: row.get::<_, i64>(2)? as u32,
                provider: row.get(3)?,
                model: row.get(4)?,
                task_summary: row.get(5)?,
                current_state,
                integrity_hash: row.get(7)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match packet {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }
}
