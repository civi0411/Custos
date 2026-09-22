use crate::connection::DbConnection;
use custos_core_domain::{DomainError, Task, TaskStatus};
use rusqlite::params;

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
        let mut stmt = conn
            .prepare(
                "SELECT id, title, status, epoch, created_at, updated_at, metadata FROM tasks WHERE id = ?1",
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
                created_at,
                updated_at,
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

    pub fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, title, status, epoch, created_at, updated_at, metadata FROM tasks ORDER BY created_at DESC")
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
                    created_at,
                    updated_at,
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
