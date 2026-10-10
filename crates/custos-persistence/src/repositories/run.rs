use crate::connection::DbConnection;
use custos_domain::{
    ClaimStatus, DispatchClaim, DomainError, LaunchAttempt, LaunchStatus, Run, RunStatus, WorkerRun,
};
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

    pub fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let res = conn.execute(
            "INSERT INTO dispatch_claims (id, task_id, node_id, assignee, depth, status, claimed_at, dispatched_at, released_at, worker_run_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                claim.id,
                claim.task_id,
                claim.node_id,
                claim.assignee,
                claim.depth as i64,
                claim.status.to_string(),
                claim.claimed_at.to_rfc3339(),
                claim.dispatched_at.map(|dt| dt.to_rfc3339()),
                claim.released_at.map(|dt| dt.to_rfc3339()),
                claim.worker_run_id,
            ],
        );

        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, Some(ref msg)))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(DomainError::Conflict(format!(
                    "Task '{}' is already claimed in active state: {}",
                    claim.task_id, msg
                )))
            }
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    pub fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "UPDATE dispatch_claims SET
                status = ?2,
                dispatched_at = ?3,
                released_at = ?4,
                worker_run_id = ?5
             WHERE id = ?1",
            params![
                claim.id,
                claim.status.to_string(),
                claim.dispatched_at.map(|dt| dt.to_rfc3339()),
                claim.released_at.map(|dt| dt.to_rfc3339()),
                claim.worker_run_id,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, node_id, assignee, depth, status, claimed_at, dispatched_at, released_at, worker_run_id
                 FROM dispatch_claims WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let claim = stmt
            .query_row(params![claim_id], Self::map_dispatch_claim)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(claim)
    }

    pub fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, node_id, assignee, depth, status, claimed_at, dispatched_at, released_at, worker_run_id
                 FROM dispatch_claims
                 WHERE task_id = ?1 AND COALESCE(node_id, '') = ?2 AND status IN ('pending', 'dispatched')
                 LIMIT 1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let claim = stmt
            .query_row(params![task_id, node_id.unwrap_or("")], Self::map_dispatch_claim)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(claim)
    }

    pub fn save_launch_attempt(&self, attempt: &LaunchAttempt) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO launch_attempts (id, worker_run_id, requested_mode, actual_mode, harness_id, status, detail, prepared_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                actual_mode = excluded.actual_mode,
                harness_id = excluded.harness_id,
                status = excluded.status,
                detail = excluded.detail,
                resolved_at = excluded.resolved_at",
            params![
                attempt.id,
                attempt.worker_run_id,
                attempt.requested_mode,
                attempt.actual_mode,
                attempt.harness_id,
                attempt.status.to_string(),
                attempt.detail,
                attempt.prepared_at.to_rfc3339(),
                attempt.resolved_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_launch_attempt(&self, attempt_id: &str) -> Result<Option<LaunchAttempt>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, worker_run_id, requested_mode, actual_mode, harness_id, status, detail, prepared_at, resolved_at
                 FROM launch_attempts WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let attempt = stmt
            .query_row(params![attempt_id], Self::map_launch_attempt)
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(attempt)
    }

    pub fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().to_rfc3339();

        let updated_wruns = conn
            .execute(
                "UPDATE worker_runs SET status = 'failed', ended_at = ?1 WHERE status = 'active'",
                params![now],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let updated_runs = conn
            .execute(
                "UPDATE runs SET status = 'failed', ended_at = ?1 WHERE status = 'active'",
                params![now],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let _ = conn.execute(
            "UPDATE dispatch_claims SET status = 'released', released_at = ?1 WHERE status IN ('pending', 'dispatched')",
            params![now],
        );

        Ok(updated_wruns + updated_runs)
    }

    fn parse_claim_status(s: &str) -> ClaimStatus {
        match s {
            "pending" => ClaimStatus::Pending,
            "dispatched" => ClaimStatus::Dispatched,
            "released" => ClaimStatus::Released,
            _ => ClaimStatus::Pending,
        }
    }

    fn map_dispatch_claim(row: &rusqlite::Row) -> rusqlite::Result<DispatchClaim> {
        let status_str: String = row.get(5)?;
        let claimed_at_str: String = row.get(6)?;
        let dispatched_at_str: Option<String> = row.get(7)?;
        let released_at_str: Option<String> = row.get(8)?;

        let claimed_at = chrono::DateTime::parse_from_rfc3339(&claimed_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let dispatched_at = dispatched_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        let released_at = released_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        Ok(DispatchClaim {
            id: row.get(0)?,
            task_id: row.get(1)?,
            node_id: row.get(2)?,
            assignee: row.get(3)?,
            depth: row.get::<_, i64>(4)? as u32,
            status: Self::parse_claim_status(&status_str),
            claimed_at,
            dispatched_at,
            released_at,
            worker_run_id: row.get(9)?,
        })
    }

    fn parse_launch_status(s: &str) -> LaunchStatus {
        match s {
            "prepared" => LaunchStatus::Prepared,
            "launched" => LaunchStatus::Launched,
            "refused" => LaunchStatus::Refused,
            "unknown" => LaunchStatus::Unknown,
            "failed" => LaunchStatus::Failed,
            _ => LaunchStatus::Unknown,
        }
    }

    fn map_launch_attempt(row: &rusqlite::Row) -> rusqlite::Result<LaunchAttempt> {
        let status_str: String = row.get(5)?;
        let prepared_at_str: String = row.get(7)?;
        let resolved_at_str: Option<String> = row.get(8)?;

        let prepared_at = chrono::DateTime::parse_from_rfc3339(&prepared_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let resolved_at = resolved_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        Ok(LaunchAttempt {
            id: row.get(0)?,
            worker_run_id: row.get(1)?,
            requested_mode: row.get(2)?,
            actual_mode: row.get(3)?,
            harness_id: row.get(4)?,
            status: Self::parse_launch_status(&status_str),
            detail: row.get(6)?,
            prepared_at,
            resolved_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use custos_domain::Task;

    #[test]
    fn test_durable_dispatch_claim_and_fencing() {
        let db = DbConnection::open_in_memory().unwrap();
        let task = Task::new("task_fencing_test_1".to_string(), "Test Task".to_string());
        {
            let c = db.lock().unwrap();
            run_migrations(&c).unwrap();
            c.execute(
                "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES (?1, ?2, 'running', 'now', 'now')",
                rusqlite::params![task.id, task.title],
            ).unwrap();
        }

        let repo = RunRepository::new(db);

        // 1. First claim succeeds
        let claim1 = DispatchClaim::new(&task.id, "worker_1", 1);
        repo.claim_ready(&claim1).expect("First claim must succeed");

        let loaded = repo.get_claim(&claim1.id).unwrap().unwrap();
        assert_eq!(loaded.assignee, "worker_1");
        assert_eq!(loaded.status, ClaimStatus::Pending);

        // Active claim lookup matches
        let active = repo.get_active_claim(&task.id, None).unwrap().unwrap();
        assert_eq!(active.id, claim1.id);

        // 2. Second claim on same task while first is active fails with Conflict
        let claim2 = DispatchClaim::new(&task.id, "worker_2", 1);
        let err = repo.claim_ready(&claim2).expect_err("Second claim must conflict");
        assert!(matches!(err, DomainError::Conflict(_)));

        // 3. Releasing first claim allows second claim to succeed
        let mut released_claim1 = claim1.clone();
        released_claim1.release().unwrap();
        repo.update_claim(&released_claim1).unwrap();

        repo.claim_ready(&claim2).expect("Claim must succeed after release");
        let active2 = repo.get_active_claim(&task.id, None).unwrap().unwrap();
        assert_eq!(active2.assignee, "worker_2");
    }

    #[test]
    fn test_durable_launch_attempt_persistence() {
        let db = DbConnection::open_in_memory().unwrap();
        {
            let c = db.lock().unwrap();
            run_migrations(&c).unwrap();
            c.execute(
                "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_launch_test', 'Launch Task', 'running', 'now', 'now')",
                [],
            ).unwrap();
            c.execute(
                "INSERT INTO runs (id, task_id, status, attempt, current_span_num, started_at) VALUES ('run_1', 'task_launch_test', 'active', 1, 1, 'now')",
                [],
            ).unwrap();
            c.execute(
                "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at) VALUES ('wrun_1', 'run_1', 'task_launch_test', 'worker_alpha', 1, 3, 'active', 'now')",
                [],
            ).unwrap();
        }

        let repo = RunRepository::new(db);
        let mut attempt = LaunchAttempt::new("wrun_1", "model").unwrap();
        attempt.mark_launched("model", Some("claude-sonnet".into())).unwrap();

        repo.save_launch_attempt(&attempt).unwrap();

        let loaded = repo.get_launch_attempt(&attempt.id).unwrap().unwrap();
        assert_eq!(loaded.worker_run_id, "wrun_1");
        assert_eq!(loaded.actual_mode.as_deref(), Some("model"));
        assert_eq!(loaded.harness_id.as_deref(), Some("claude-sonnet"));
        assert_eq!(loaded.status, LaunchStatus::Launched);
    }

    #[test]
    fn test_reconcile_runs_on_startup() {
        let db = DbConnection::open_in_memory().unwrap();
        {
            let c = db.lock().unwrap();
            run_migrations(&c).unwrap();
            c.execute(
                "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_rec_test', 'Rec Task', 'running', 'now', 'now')",
                [],
            ).unwrap();
            c.execute(
                "INSERT INTO runs (id, task_id, status, attempt, current_span_num, started_at) VALUES ('run_stale', 'task_rec_test', 'active', 1, 1, 'now')",
                [],
            ).unwrap();
            c.execute(
                "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at) VALUES ('wrun_stale', 'run_stale', 'task_rec_test', 'worker_alpha', 1, 3, 'active', 'now')",
                [],
            ).unwrap();
            c.execute(
                "INSERT INTO dispatch_claims (id, task_id, node_id, assignee, depth, status, claimed_at) VALUES ('claim_stale', 'task_rec_test', '', 'worker_alpha', 1, 'dispatched', 'now')",
                [],
            ).unwrap();
        }

        let repo = RunRepository::new(db);
        let reconciled_count = repo.reconcile_runs_on_startup().expect("Reconcile should succeed");
        assert_eq!(reconciled_count, 2);

        let wrun = repo.get_worker_run("wrun_stale").unwrap().unwrap();
        assert_eq!(wrun.status, RunStatus::Failed);

        let run = repo.get_run("run_stale").unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Failed);

        let claim = repo.get_claim("claim_stale").unwrap().unwrap();
        assert_eq!(claim.status, ClaimStatus::Released);
    }
}
