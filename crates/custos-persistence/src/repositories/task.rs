use crate::connection::DbConnection;
use custos_core::kernel::{TaskEvent, TaskSnapshotImported};
use custos_domain::{DomainError, Task, TaskStatus};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone)]
pub struct TaskRepository {
    db: DbConnection,
}

impl TaskRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        let conn = self.db.lock()?;
        Self::read_task(&conn, task_id)
    }

    fn read_task(connection: &Connection, task_id: &str) -> Result<Option<Task>, DomainError> {
        let mut stmt = connection
            .prepare(
                "SELECT id, title, status, epoch, created_at, updated_at, metadata, contract_json FROM tasks WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

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

            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;
            let meta_str: String = row.get(6)?;
            let contract_json: Option<String> = row.get(7)?;

            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));

            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                status,
                epoch: row.get::<_, i64>(3)? as u64,
                state_version: 0,
                active_revision: None,
                revision_history: Vec::new(),
                created_at,
                updated_at,
                contract: contract_json
                    .map(|value| serde_json::from_str(&value))
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            7,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                active_contract_revision: None,
                metadata,
            })
        });

        match task {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    pub fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        Self::write_task(&conn, task)
    }

    pub fn save_task_with_event(&self, task: &Task, event: &TaskEvent) -> Result<(), DomainError> {
        if task.id != event.task_id() {
            return Err(DomainError::InvariantViolation(
                "task snapshot and event IDs do not match".into(),
            ));
        }
        let event_json = serde_json::to_string(event)
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let event_sequence = i64::try_from(event.sequence()).map_err(|_| {
            DomainError::InvariantViolation(
                "task event sequence exceeds SQLite integer range".into(),
            )
        })?;
        let mut conn = self.db.lock()?;
        let transaction = conn
            .transaction()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT event_json FROM task_events WHERE task_id = ?1 AND sequence = ?2",
                params![event.task_id(), event_sequence],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        if let Some(existing) = existing {
            if existing == event_json {
                transaction
                    .commit()
                    .map_err(|error| DomainError::Validation(error.to_string()))?;
                return Ok(());
            }
            return Err(DomainError::Conflict(format!(
                "Task event sequence {} already contains a different event",
                event.sequence()
            )));
        }

        // A database upgraded from snapshot-only persistence has no creation event.
        // Seed its existing state exactly once before appending the first new transition.
        let journal_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM task_events WHERE task_id = ?1",
                params![event.task_id()],
                |row| row.get(0),
            )
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        if journal_count == 0 && event.event_type() != "task_created" {
            if let Some(previous) = Self::read_task(&transaction, event.task_id())? {
                let expected_sequence = previous.epoch.checked_add(1).ok_or_else(|| {
                    DomainError::InvariantViolation("task epoch exhausted".into())
                })?;
                if event.sequence() != expected_sequence {
                    return Err(DomainError::Conflict(format!(
                        "first journaled transition must follow snapshot epoch {}, got {}",
                        previous.epoch,
                        event.sequence()
                    )));
                }
                let imported = TaskEvent::SnapshotImported(TaskSnapshotImported { task: previous });
                let imported_sequence = i64::try_from(imported.sequence()).map_err(|_| {
                    DomainError::InvariantViolation(
                        "imported task sequence exceeds SQLite integer range".into(),
                    )
                })?;
                let imported_json = serde_json::to_string(&imported)
                    .map_err(|error| DomainError::Validation(error.to_string()))?;
                transaction
                    .execute(
                        "INSERT INTO task_events (task_id, sequence, event_type, event_json) VALUES (?1, ?2, ?3, ?4)",
                        params![imported.task_id(), imported_sequence, imported.event_type(), imported_json],
                    )
                    .map_err(|error| DomainError::Validation(error.to_string()))?;
            }
        }

        if event.event_type() == "task_created" && event.sequence() != 0 {
            return Err(DomainError::InvariantViolation(
                "task creation event must use sequence zero".into(),
            ));
        }
        if task.epoch != event.sequence() {
            return Err(DomainError::InvariantViolation(
                "task snapshot epoch does not match event sequence".into(),
            ));
        }

        Self::write_task(&transaction, task)?;
        transaction
            .execute(
                "INSERT INTO task_events (task_id, sequence, event_type, event_json) VALUES (?1, ?2, ?3, ?4)",
                params![event.task_id(), event_sequence, event.event_type(), event_json],
            )
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| DomainError::Validation(error.to_string()))
    }

    pub fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
        let conn = self.db.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT sequence, event_json FROM task_events WHERE task_id = ?1 ORDER BY sequence",
            )
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let rows = statement
            .query_map(params![task_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, json) =
                row.map_err(|error| DomainError::Validation(error.to_string()))?;
            let event: TaskEvent = serde_json::from_str(&json)
                .map_err(|error| DomainError::Validation(format!("Invalid task event: {error}")))?;
            if event.task_id() != task_id || event.sequence() != sequence as u64 {
                return Err(DomainError::InvariantViolation(
                    "task event index does not match serialized event".into(),
                ));
            }
            events.push(event);
        }
        Ok(events)
    }

    fn write_task(connection: &Connection, task: &Task) -> Result<(), DomainError> {
        connection.execute(
            "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at, metadata, contract_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
             title=excluded.title, status=excluded.status, epoch=excluded.epoch,
             updated_at=excluded.updated_at, metadata=excluded.metadata,
             contract_json=excluded.contract_json",
            params![
                task.id,
                task.title,
                task.status.to_string(),
                task.epoch as i64,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.metadata.to_string(),
                task.contract
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| DomainError::Validation(error.to_string()))?,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, title, status, epoch, created_at, updated_at, metadata, contract_json FROM tasks ORDER BY created_at DESC")
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
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

                let created_at_str: String = row.get(4)?;
                let updated_at_str: String = row.get(5)?;
                let meta_str: String = row.get(6)?;
                let contract_json: Option<String> = row.get(7)?;

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));

                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    status,
                    epoch: row.get::<_, i64>(3)? as u64,
                    state_version: 0,
                    active_revision: None,
                    revision_history: Vec::new(),
                    created_at,
                    updated_at,
                    contract: contract_json
                        .map(|value| serde_json::from_str(&value))
                        .transpose()
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                    active_contract_revision: None,
                    metadata,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut tasks = Vec::new();
        for r in rows {
            tasks.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(tasks)
    }
}
