//! Telemetry persistence for run execution step spans, logs, errors, and watchdog findings.

use custos_domain::{
    DomainError, RunStepSpan, StepError, SupervisionFinding, SupervisionLogEntry,
    SupervisionRunRecord,
};
use rusqlite::params;

use super::RunRepository;

impl RunRepository {
    pub fn persist_telemetry_batch(
        &self,
        runs: &[SupervisionRunRecord],
        spans: &[RunStepSpan],
        logs: &[SupervisionLogEntry],
        errors: &[StepError],
        findings: &[SupervisionFinding],
    ) -> Result<(), DomainError> {
        if runs.is_empty()
            && spans.is_empty()
            && logs.is_empty()
            && errors.is_empty()
            && findings.is_empty()
        {
            return Ok(());
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(format!("Telemetry tx open failed: {e}")))?;

        if !runs.is_empty() {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO supervision_runs
                     (run_id, workbench_id, agent, status, started_at_ms, ended_at_ms, duration_ms, tokens, cost_usd, usage_measurement_json, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(run_id) DO UPDATE SET
                       workbench_id = COALESCE(excluded.workbench_id, supervision_runs.workbench_id),
                       agent = CASE WHEN excluded.agent != '' AND excluded.agent != 'unknown-agent' THEN excluded.agent ELSE supervision_runs.agent END,
                       status = CASE
                         WHEN supervision_runs.status IN ('success', 'error') AND excluded.status = 'running' THEN supervision_runs.status
                         ELSE excluded.status
                       END,
                       started_at_ms = MIN(supervision_runs.started_at_ms, excluded.started_at_ms),
                       ended_at_ms = COALESCE(excluded.ended_at_ms, supervision_runs.ended_at_ms),
                       duration_ms = COALESCE(excluded.duration_ms, supervision_runs.duration_ms),
                       tokens = COALESCE(excluded.tokens, supervision_runs.tokens),
                       cost_usd = COALESCE(excluded.cost_usd, supervision_runs.cost_usd),
                       usage_measurement_json = CASE
                         WHEN excluded.usage_measurement_json LIKE '%unknown%' AND supervision_runs.usage_measurement_json NOT LIKE '%unknown%'
                           THEN supervision_runs.usage_measurement_json
                         ELSE excluded.usage_measurement_json
                       END,
                       updated_at_ms = excluded.updated_at_ms",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            for run in runs {
                let usage_json = serde_json::to_string(&run.usage_measurement)
                    .unwrap_or_else(|_| r#"{"certainty":"unknown"}"#.into());
                stmt.execute(params![
                    run.run_id,
                    run.workbench_id,
                    run.agent,
                    run.status.as_str(),
                    run.started_at_ms,
                    run.ended_at_ms,
                    run.duration_ms,
                    run.tokens.map(|t| t as i64),
                    run.cost_usd,
                    usage_json,
                    now_ms,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        if !spans.is_empty() {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO run_step_spans
                     (id, run_id, worker_run_id, parent_id, kind, name, status, started_at_ms, ended_at_ms, duration_ms, metadata_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(id) DO UPDATE SET
                       worker_run_id = COALESCE(excluded.worker_run_id, run_step_spans.worker_run_id),
                       parent_id = COALESCE(excluded.parent_id, run_step_spans.parent_id),
                       status = CASE WHEN excluded.status = 'running' AND run_step_spans.status != 'running' THEN run_step_spans.status ELSE excluded.status END,
                       ended_at_ms = COALESCE(excluded.ended_at_ms, run_step_spans.ended_at_ms),
                       duration_ms = COALESCE(excluded.duration_ms, run_step_spans.duration_ms),
                       metadata_json = CASE WHEN excluded.metadata_json != '{}' THEN excluded.metadata_json ELSE run_step_spans.metadata_json END",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            for span in spans {
                let metadata_json =
                    serde_json::to_string(&span.metadata).unwrap_or_else(|_| "{}".into());
                stmt.execute(params![
                    span.id,
                    span.run_id,
                    span.worker_run_id,
                    span.parent_id,
                    span.kind.as_str(),
                    span.name,
                    span.status.as_str(),
                    span.started_at_ms,
                    span.ended_at_ms,
                    span.duration_ms,
                    metadata_json,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        if !logs.is_empty() {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO supervision_logs
                     (id, run_id, worker_run_id, span_id, timestamp_ms, level, message, metadata_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            for log in logs {
                let metadata_json =
                    serde_json::to_string(&log.metadata).unwrap_or_else(|_| "{}".into());
                stmt.execute(params![
                    log.id,
                    log.run_id,
                    None::<String>,
                    log.span_id,
                    log.timestamp_ms,
                    log.level.as_str(),
                    log.message,
                    metadata_json,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        if !errors.is_empty() {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO supervision_errors
                     (id, run_id, worker_run_id, span_id, error_type, message, stack, agent, signature_hash, timestamp_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            for err in errors {
                stmt.execute(params![
                    err.id,
                    err.run_id,
                    None::<String>,
                    err.span_id,
                    err.error_type,
                    err.message,
                    err.stack,
                    err.agent,
                    err.signature_hash,
                    err.timestamp_ms,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        if !findings.is_empty() {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO supervision_findings
                     (id, run_id, worker_run_id, kind, severity, message, created_at_ms, resolved_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)
                     ON CONFLICT(id) DO UPDATE SET
                       message = excluded.message",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            for finding in findings {
                stmt.execute(params![
                    finding.id,
                    finding.run_id,
                    None::<String>,
                    finding.kind.as_str(),
                    finding.severity.as_str(),
                    finding.message,
                    finding.created_at_ms,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| DomainError::Validation(format!("Telemetry commit failed: {e}")))?;
        Ok(())
    }

    pub fn prune_telemetry_before(
        &self,
        log_cutoff_ms: i64,
        span_cutoff_ms: i64,
    ) -> Result<usize, DomainError> {
        let conn = self.db.lock()?;
        let pruned_logs = conn
            .execute(
                "DELETE FROM supervision_logs WHERE timestamp_ms < ?1",
                params![log_cutoff_ms],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let pruned_spans = conn
            .execute(
                "DELETE FROM run_step_spans
                 WHERE started_at_ms < ?1
                   AND run_id NOT IN (SELECT run_id FROM supervision_runs WHERE status = 'running')",
                params![span_cutoff_ms],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(pruned_logs + pruned_spans)
    }
}
