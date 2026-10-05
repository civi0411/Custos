use crate::connection::DbConnection;
use custos_domain::{DomainError, Run, RunStatus, WorkerRun};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct RunRepository {
    db: DbConnection,
}

impl RunRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn save_run(&self, run: &Run) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let meta_str = serde_json::to_string(&run.metadata).unwrap_or_else(|_| "{}".to_string());
        let ended_at_str = run.ended_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "INSERT INTO runs (id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                task_id = excluded.task_id,
                workflow_revision = excluded.workflow_revision,
                status = excluded.status,
                attempt = excluded.attempt,
                current_span_num = excluded.current_span_num,
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                metadata_json = excluded.metadata_json",
            params![
                run.id,
                run.task_id,
                run.workflow_revision,
                run.status.to_string(),
                run.attempt as i64,
                run.current_span_num as i64,
                run.started_at.to_rfc3339(),
                ended_at_str,
                meta_str,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json
                 FROM runs WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let run = stmt
            .query_row(params![run_id], Self::map_run)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(run)
    }

    pub fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json
                 FROM runs WHERE task_id = ?1 ORDER BY started_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], Self::map_run)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(runs)
    }

    pub fn save_worker_run(&self, wrun: &WorkerRun) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let ended_at_str = wrun.ended_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                run_id = excluded.run_id,
                task_id = excluded.task_id,
                worker_id = excluded.worker_id,
                attempt_id = excluded.attempt_id,
                max_attempts = excluded.max_attempts,
                status = excluded.status,
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                continuation_packet_ref = excluded.continuation_packet_ref",
            params![
                wrun.id,
                wrun.run_id,
                wrun.task_id,
                wrun.worker_id,
                wrun.attempt_id as i64,
                wrun.max_attempts as i64,
                wrun.status.to_string(),
                wrun.started_at.to_rfc3339(),
                ended_at_str,
                wrun.continuation_packet_ref,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref
                 FROM worker_runs WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let wrun = stmt
            .query_row(params![wrun_id], Self::map_worker_run)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(wrun)
    }

    pub fn list_worker_runs_for_task(&self, task_id: &str) -> Result<Vec<WorkerRun>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref
                 FROM worker_runs WHERE task_id = ?1 ORDER BY started_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![task_id], Self::map_worker_run)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut wruns = Vec::new();
        for row in rows {
            wruns.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(wruns)
    }

    pub fn list_worker_runs_for_run(&self, run_id: &str) -> Result<Vec<WorkerRun>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref
                 FROM worker_runs WHERE run_id = ?1 ORDER BY started_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![run_id], Self::map_worker_run)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut wruns = Vec::new();
        for row in rows {
            wruns.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(wruns)
    }

    fn parse_status(s: &str) -> RunStatus {
        match s {
            "pending" => RunStatus::Pending,
            "active" => RunStatus::Active,
            "suspended" => RunStatus::Suspended,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            _ => RunStatus::Cancelled,
        }
    }

    fn map_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
        let status_str: String = row.get(3)?;
        let started_at_str: String = row.get(6)?;
        let ended_at_str: Option<String> = row.get(7)?;
        let meta_str: Option<String> = row.get(8)?;

        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let ended_at = ended_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        let metadata = meta_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(Run {
            id: row.get(0)?,
            task_id: row.get(1)?,
            workflow_revision: row.get(2)?,
            status: Self::parse_status(&status_str),
            attempt: row.get::<_, i64>(4)? as u32,
            current_span_num: row.get::<_, i64>(5)? as u32,
            started_at,
            ended_at,
            metadata,
        })
    }

    fn map_worker_run(row: &rusqlite::Row) -> rusqlite::Result<WorkerRun> {
        let status_str: String = row.get(6)?;
        let started_at_str: String = row.get(7)?;
        let ended_at_str: Option<String> = row.get(8)?;

        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let ended_at = ended_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        Ok(WorkerRun {
            id: row.get(0)?,
            run_id: row.get(1)?,
            task_id: row.get(2)?,
            worker_id: row.get(3)?,
            attempt_id: row.get::<_, i64>(4)? as u32,
            max_attempts: row.get::<_, i64>(5)? as u32,
            status: Self::parse_status(&status_str),
            started_at,
            ended_at,
            continuation_packet_ref: row.get(9)?,
        })
    }
}
