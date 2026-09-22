use crate::connection::DbConnection;
use custos_core_domain::{DomainError, Span, SpanState};
use rusqlite::params;

#[derive(Clone)]
pub struct SpanRepository {
    db: DbConnection,
}

impl SpanRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, span_num, state, provider, model, input_digest, output_digest, started_at, ended_at FROM spans WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let span = stmt.query_row(params![span_id], |row| {
            let state_str: String = row.get(3)?;
            let state = match state_str.as_str() {
                "completed" => SpanState::Completed,
                "failed" => SpanState::Failed,
                _ => SpanState::Started,
            };

            let started_at_str: String = row.get(8)?;
            let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let ended_at = row
                .get::<_, Option<String>>(9)?
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            Ok(Span {
                id: row.get(0)?,
                task_id: row.get(1)?,
                span_num: row.get::<_, i64>(2)? as u32,
                state,
                provider: row.get(4)?,
                model: row.get(5)?,
                input_digest: row.get(6)?,
                output_digest: row.get(7)?,
                started_at,
                ended_at,
            })
        });

        match span {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    pub fn save_span(&self, span: &Span) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
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
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, span_num, state, provider, model, input_digest, output_digest, started_at, ended_at FROM spans WHERE task_id = ?1 ORDER BY span_num ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], |row| {
                let state_str: String = row.get(3)?;
                let state = match state_str.as_str() {
                    "completed" => SpanState::Completed,
                    "failed" => SpanState::Failed,
                    _ => SpanState::Started,
                };

                let started_at_str: String = row.get(8)?;
                let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let ended_at = row
                    .get::<_, Option<String>>(9)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                Ok(Span {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    span_num: row.get::<_, i64>(2)? as u32,
                    state,
                    provider: row.get(4)?,
                    model: row.get(5)?,
                    input_digest: row.get(6)?,
                    output_digest: row.get(7)?,
                    started_at,
                    ended_at,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut spans = Vec::new();
        for r in rows {
            spans.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(spans)
    }
}
