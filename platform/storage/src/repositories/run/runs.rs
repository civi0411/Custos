use custos_domain::{DomainError, Run, RunStatus};
use rusqlite::{params, OptionalExtension};

use super::RunRepository;

impl RunRepository {
    pub fn save_run(&self, run: &mut Run) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let next_version = run
            .state_version
            .checked_add(1)
            .filter(|v| *v <= i64::MAX as u64)
            .ok_or_else(|| DomainError::Validation("Run state version exhausted".into()))?;
        let meta_str = serde_json::to_string(&run.metadata)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let ended_at_str = run.ended_at.map(|dt| dt.to_rfc3339());

        let changed = conn.execute(
            "INSERT INTO runs (id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json, state_version)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
             WHERE ?11 = 0 OR EXISTS (SELECT 1 FROM runs WHERE id = ?1)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                attempt = excluded.attempt,
                current_span_num = excluded.current_span_num,
                ended_at = excluded.ended_at,
                metadata_json = excluded.metadata_json,
                state_version = excluded.state_version
             WHERE runs.state_version = ?11 AND runs.task_id = excluded.task_id
               AND runs.workflow_revision IS excluded.workflow_revision
               AND runs.started_at = excluded.started_at
               AND excluded.attempt >= runs.attempt
               AND (runs.status = excluded.status
                 OR (runs.status = 'pending' AND excluded.status IN ('active', 'cancelled'))
                 OR (runs.status = 'active' AND excluded.status IN ('suspended', 'completed', 'failed', 'cancelled', 'uncertain'))
                 OR (runs.status = 'uncertain' AND excluded.status IN ('completed', 'failed', 'cancelled'))
                 OR (runs.status = 'suspended' AND excluded.status IN ('active', 'cancelled')))",
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
                next_version as i64,
                run.state_version as i64,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        if changed != 1 {
            return Err(DomainError::Conflict(format!(
                "Stale or invalid run update: {}",
                run.id
            )));
        }
        run.state_version = next_version;

        // Synchronize canonical Run with telemetry index
        let workbench_id = run
            .metadata
            .get("pack_id")
            .or_else(|| run.metadata.get("workbench_id"))
            .and_then(|v| v.as_str());
        let agent = run
            .metadata
            .get("harness_id")
            .or_else(|| run.metadata.get("model"))
            .and_then(|v| v.as_str())
            .unwrap_or("agent");
        let sup_status = if run.status.is_terminal() {
            if run.status == RunStatus::Completed {
                "success"
            } else {
                "error"
            }
        } else {
            "running"
        };
        let started_at_ms = run.started_at.timestamp_millis();
        let ended_at_ms = run.ended_at.map(|t| t.timestamp_millis());
        let duration_ms = ended_at_ms.map(|e| (e - started_at_ms).max(0));
        let now_ms = chrono::Utc::now().timestamp_millis();

        let _ = conn.execute(
            "INSERT INTO supervision_runs
             (run_id, workbench_id, agent, status, started_at_ms, ended_at_ms, duration_ms, tokens, cost_usd, usage_measurement_json, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, '{\"certainty\":\"unknown\"}', ?8)
             ON CONFLICT(run_id) DO UPDATE SET
               workbench_id = COALESCE(excluded.workbench_id, supervision_runs.workbench_id),
               agent = CASE WHEN excluded.agent != '' AND excluded.agent != 'agent' THEN excluded.agent ELSE supervision_runs.agent END,
               status = excluded.status,
               ended_at_ms = COALESCE(excluded.ended_at_ms, supervision_runs.ended_at_ms),
               duration_ms = COALESCE(excluded.duration_ms, supervision_runs.duration_ms),
               updated_at_ms = excluded.updated_at_ms",
            params![
                run.id,
                workbench_id,
                agent,
                sup_status,
                started_at_ms,
                ended_at_ms,
                duration_ms,
                now_ms,
            ],
        );

        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json, state_version
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
                "SELECT id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json, state_version
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

    pub fn list_uncertain_runs(&self) -> Result<Vec<Run>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, workflow_revision, status, attempt, current_span_num, started_at, ended_at, metadata_json, state_version
                 FROM runs WHERE status = 'uncertain' ORDER BY started_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_run)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(row.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(runs)
    }

    pub fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().to_rfc3339();

        // 1. Remote runs transition to 'uncertain' to avoid killing live cloud pods
        let updated_remote_runs = conn
            .execute(
                "UPDATE runs SET status = 'uncertain', state_version = state_version + 1 \
                 WHERE status = 'active' AND ( \
                     json_extract(metadata_json, '$.external_job_ref') IS NOT NULL \
                     OR json_extract(metadata_json, '$.backend') IN ('colab', 'kaggle', 'runpod', 'vast_ai', 'dedicated_ssh', 'remote') \
                     OR id IN ( \
                         SELECT run_id FROM worker_runs WHERE id IN ( \
                             SELECT worker_run_id FROM dispatch_claims \
                              WHERE status IN ('pending', 'dispatched') \
                                AND node_id IS NOT NULL AND node_id != '' \
                                AND worker_run_id IS NOT NULL \
                         ) \
                     ) \
                 )",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let updated_remote_wruns = conn
            .execute(
                "UPDATE worker_runs SET status = 'uncertain', state_version = state_version + 1 \
                 WHERE status = 'active' AND ( \
                     run_id IN (SELECT id FROM runs WHERE status = 'uncertain') \
                     OR id IN (SELECT worker_run_id FROM launch_attempts WHERE actual_mode IN ('colab', 'kaggle', 'runpod', 'vast_ai', 'dedicated_ssh', 'remote')) \
                     OR id IN ( \
                         SELECT worker_run_id FROM dispatch_claims \
                          WHERE status IN ('pending', 'dispatched') \
                            AND node_id IS NOT NULL AND node_id != '' \
                            AND worker_run_id IS NOT NULL \
                     ) \
                 )",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        // 2. Remaining active local runs/worker_runs fail-closed to 'failed'
        let updated_local_wruns = conn
            .execute(
                "UPDATE worker_runs SET status = 'failed', ended_at = ?1, state_version = state_version + 1 WHERE status = 'active'",
                params![now],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let updated_local_runs = conn
            .execute(
                "UPDATE runs SET status = 'failed', ended_at = ?1, state_version = state_version + 1 WHERE status = 'active'",
                params![now],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "UPDATE dispatch_claims SET status = 'released', released_at = ?1 
             WHERE status IN ('pending', 'dispatched') AND (node_id IS NULL OR node_id = '')",
            params![now],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "UPDATE launch_attempts SET status = 'unknown', detail = 'Daemon restart during in-flight execution; recovered fail-closed', resolved_at = ?1 WHERE status IN ('prepared')",
            params![now],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(updated_remote_runs + updated_remote_wruns + updated_local_wruns + updated_local_runs)
    }

    pub(crate) fn parse_status(s: &str) -> RunStatus {
        match s {
            "pending" => RunStatus::Pending,
            "active" => RunStatus::Active,
            "suspended" => RunStatus::Suspended,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "uncertain" => RunStatus::Uncertain,
            _ => RunStatus::Cancelled,
        }
    }

    pub(crate) fn map_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
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
            state_version: row.get(9)?,
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
}
