use custos_domain::{DomainError, WorkerRun};
use rusqlite::{params, OptionalExtension};

use super::RunRepository;

impl RunRepository {
    pub fn save_worker_run(&self, wrun: &mut WorkerRun) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let next_version = wrun
            .state_version
            .checked_add(1)
            .filter(|v| *v <= i64::MAX as u64)
            .ok_or_else(|| DomainError::Validation("Worker state version exhausted".into()))?;
        let ended_at_str = wrun.ended_at.map(|dt| dt.to_rfc3339());

        let changed = conn.execute(
            "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref, state_version)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
             WHERE ?12 = 0 OR EXISTS (SELECT 1 FROM worker_runs WHERE id = ?1)
             ON CONFLICT(id) DO UPDATE SET
                attempt_id = excluded.attempt_id,
                max_attempts = excluded.max_attempts,
                status = excluded.status,
                ended_at = excluded.ended_at,
                continuation_packet_ref = excluded.continuation_packet_ref,
                state_version = excluded.state_version
             WHERE worker_runs.state_version = ?12
               AND worker_runs.run_id IS excluded.run_id AND worker_runs.task_id = excluded.task_id
               AND worker_runs.worker_id = excluded.worker_id AND worker_runs.started_at = excluded.started_at
               AND excluded.attempt_id >= worker_runs.attempt_id
               AND (worker_runs.status = excluded.status
                 OR (worker_runs.status = 'pending' AND excluded.status IN ('active', 'cancelled'))
                 OR (worker_runs.status = 'active' AND excluded.status IN ('suspended', 'completed', 'failed', 'cancelled', 'uncertain'))
                 OR (worker_runs.status = 'uncertain' AND excluded.status IN ('completed', 'failed', 'cancelled'))
                 OR (worker_runs.status = 'suspended' AND excluded.status IN ('active', 'cancelled')))",
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
                next_version as i64,
                wrun.state_version as i64,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        if changed != 1 {
            return Err(DomainError::Conflict(format!(
                "Stale or invalid worker update: {}",
                wrun.id
            )));
        }
        wrun.state_version = next_version;
        Ok(())
    }

    pub fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref, state_version
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
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref, state_version
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
                "SELECT id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at, ended_at, continuation_packet_ref, state_version
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

    pub(crate) fn map_worker_run(row: &rusqlite::Row) -> rusqlite::Result<WorkerRun> {
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
            state_version: row.get(10)?,
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
