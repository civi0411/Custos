//! Read-only queries for run execution step spans, logs, errors, and watchdog findings.

use custos_core::contracts::workflow::{
    SupervisionFindingFilter, SupervisionLogFilter, SupervisionRunFilter,
};
use custos_domain::{
    DomainError, FindingKind, LogSeverity, RunStepSpan, StepError, StepSpanKind, StepSpanStatus,
    SupervisionFinding, SupervisionLogEntry, SupervisionRunRecord, SupervisionRunStatus,
    SupervisionSeverity, UsageMeasurement,
};
use rusqlite::{params, OptionalExtension, Row};
use serde_json::Value;

use super::RunRepository;

fn map_run_row(row: &Row<'_>) -> rusqlite::Result<SupervisionRunRecord> {
    let status_str: String = row.get(3)?;
    let usage_json: String = row.get(9)?;
    let usage_measurement =
        serde_json::from_str::<UsageMeasurement>(&usage_json).unwrap_or(UsageMeasurement::Unknown);

    Ok(SupervisionRunRecord {
        run_id: row.get(0)?,
        workbench_id: row.get(1)?,
        agent: row.get(2)?,
        status: SupervisionRunStatus::parse(&status_str),
        started_at_ms: row.get(4)?,
        ended_at_ms: row.get(5)?,
        duration_ms: row.get(6)?,
        tokens: row.get::<_, Option<i64>>(7)?.map(|v| v.max(0) as u64),
        cost_usd: row.get(8)?,
        usage_measurement,
    })
}

fn map_span_row(row: &Row<'_>) -> rusqlite::Result<RunStepSpan> {
    let kind_str: String = row.get(4)?;
    let status_str: String = row.get(6)?;
    let meta_str: String = row.get(10)?;
    Ok(RunStepSpan {
        id: row.get(0)?,
        run_id: row.get(1)?,
        worker_run_id: row.get(2)?,
        parent_id: row.get(3)?,
        kind: StepSpanKind::parse(&kind_str),
        name: row.get(5)?,
        status: StepSpanStatus::parse(&status_str),
        started_at_ms: row.get(7)?,
        ended_at_ms: row.get(8)?,
        duration_ms: row.get(9)?,
        metadata: serde_json::from_str::<Value>(&meta_str)
            .unwrap_or(Value::Object(Default::default())),
    })
}

impl RunRepository {
    pub fn get_telemetry_run_record(
        &self,
        run_id: &str,
    ) -> Result<Option<SupervisionRunRecord>, DomainError> {
        self.db.read(|conn| {
            conn.query_row(
                "SELECT run_id, workbench_id, agent, status, started_at_ms, ended_at_ms, duration_ms, tokens, cost_usd, usage_measurement_json
                 FROM supervision_runs WHERE run_id = ?1",
                params![run_id],
                map_run_row,
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))
        })
    }

    pub fn list_telemetry_runs(
        &self,
        filter: &SupervisionRunFilter,
    ) -> Result<Vec<SupervisionRunRecord>, DomainError> {
        let limit = filter.limit.unwrap_or(100).clamp(1, 500) as i64;
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT run_id, workbench_id, agent, status, started_at_ms, ended_at_ms, duration_ms, tokens, cost_usd, usage_measurement_json
                     FROM supervision_runs
                     WHERE (?1 IS NULL OR workbench_id = ?1)
                       AND (?2 IS NULL OR status = ?2)
                     ORDER BY started_at_ms DESC
                     LIMIT ?3",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(
                    params![filter.workbench_id.as_deref(), filter.status.as_deref(), limit],
                    map_run_row,
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn list_run_spans(&self, run_id: &str) -> Result<Vec<RunStepSpan>, DomainError> {
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, run_id, worker_run_id, parent_id, kind, name, status, started_at_ms, ended_at_ms, duration_ms, metadata_json
                     FROM run_step_spans
                     WHERE run_id = ?1
                     ORDER BY started_at_ms ASC",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(params![run_id], map_span_row)
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn list_logs(
        &self,
        filter: &SupervisionLogFilter,
    ) -> Result<Vec<SupervisionLogEntry>, DomainError> {
        let limit = filter.limit.unwrap_or(200).clamp(1, 1000) as i64;
        let level_str = filter.level.map(|l| l.as_str().to_string());
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, run_id, span_id, timestamp_ms, level, message, metadata_json
                     FROM supervision_logs
                     WHERE (?1 IS NULL OR run_id = ?1)
                       AND (?2 IS NULL OR level = ?2)
                       AND (?3 IS NULL OR span_id = ?3)
                     ORDER BY timestamp_ms DESC
                     LIMIT ?4",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(
                    params![
                        filter.run_id.as_deref(),
                        level_str.as_deref(),
                        filter.span_id.as_deref(),
                        limit
                    ],
                    |row| {
                        let level_raw: String = row.get(4)?;
                        let meta_str: String = row.get(6)?;
                        Ok(SupervisionLogEntry {
                            id: row.get(0)?,
                            run_id: row.get(1)?,
                            worker_run_id: None,
                            span_id: row.get(2)?,
                            timestamp_ms: row.get(3)?,
                            level: LogSeverity::parse(&level_raw),
                            message: row.get(5)?,
                            metadata: serde_json::from_str::<Value>(&meta_str)
                                .unwrap_or(Value::Object(Default::default())),
                        })
                    },
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn list_run_errors(&self, run_id: &str) -> Result<Vec<StepError>, DomainError> {
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, run_id, span_id, error_type, message, signature_hash, stack, agent, timestamp_ms
                     FROM supervision_errors
                     WHERE run_id = ?1
                     ORDER BY timestamp_ms ASC",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(params![run_id], |row| {
                    Ok(StepError {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        worker_run_id: None,
                        span_id: row.get(2)?,
                        error_type: row.get(3)?,
                        message: row.get(4)?,
                        signature_hash: row.get(5)?,
                        stack: row.get(6)?,
                        agent: row.get(7)?,
                        timestamp_ms: row.get(8)?,
                    })
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn list_errors(
        &self,
        workbench_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StepError>, DomainError> {
        let lim = limit.clamp(1, 500) as i64;
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT e.id, e.run_id, e.span_id, e.error_type, e.message, e.signature_hash, e.stack, e.agent, e.timestamp_ms
                     FROM supervision_errors e
                     LEFT JOIN supervision_runs r ON e.run_id = r.run_id
                     WHERE (?1 IS NULL OR r.workbench_id = ?1)
                     ORDER BY e.timestamp_ms DESC
                     LIMIT ?2",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(params![workbench_id, lim], |row| {
                    Ok(StepError {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        worker_run_id: None,
                        span_id: row.get(2)?,
                        error_type: row.get(3)?,
                        message: row.get(4)?,
                        signature_hash: row.get(5)?,
                        stack: row.get(6)?,
                        agent: row.get(7)?,
                        timestamp_ms: row.get(8)?,
                    })
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn list_findings(
        &self,
        filter: &SupervisionFindingFilter,
    ) -> Result<Vec<SupervisionFinding>, DomainError> {
        let limit = filter.limit.unwrap_or(200).clamp(1, 500) as i64;
        let sev_str = filter.severity.map(|s| s.as_str().to_string());
        self.db.read(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT f.id, f.run_id, f.kind, f.severity, f.message, f.created_at_ms
                     FROM supervision_findings f
                     LEFT JOIN supervision_runs r ON f.run_id = r.run_id
                     WHERE (?1 IS NULL OR f.run_id = ?1)
                       AND (?2 IS NULL OR r.workbench_id = ?2)
                       AND (?3 IS NULL OR f.severity = ?3)
                       AND (?4 = 0 OR f.resolved_at_ms IS NULL)
                     ORDER BY f.created_at_ms DESC
                     LIMIT ?5",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let rows = stmt
                .query_map(
                    params![
                        filter.run_id.as_deref(),
                        filter.workbench_id.as_deref(),
                        sev_str.as_deref(),
                        if filter.unresolved_only { 1 } else { 0 },
                        limit
                    ],
                    |row| {
                        let kind_str: String = row.get(2)?;
                        let sev_raw: String = row.get(3)?;
                        Ok(SupervisionFinding {
                            id: row.get(0)?,
                            run_id: row.get(1)?,
                            kind: FindingKind::parse(&kind_str).unwrap_or(FindingKind::LongRunning),
                            severity: SupervisionSeverity::parse(&sev_raw),
                            message: row.get(4)?,
                            created_at_ms: row.get(5)?,
                        })
                    },
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(rows)
        })
    }

    pub fn count_overview_stats(&self) -> Result<(usize, usize, usize, usize, usize), DomainError> {
        self.db.read(|conn| {
            let total_runs: usize = conn
                .query_row("SELECT COUNT(*) FROM supervision_runs", [], |r| r.get(0))
                .unwrap_or(0);
            let active_runs: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM supervision_runs WHERE status = 'running'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let failed_runs: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM supervision_runs WHERE status = 'error'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let open_findings: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM supervision_findings WHERE resolved_at_ms IS NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let critical_findings: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM supervision_findings WHERE resolved_at_ms IS NULL AND severity = 'critical'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            Ok((
                active_runs,
                total_runs,
                failed_runs,
                open_findings,
                critical_findings,
            ))
        })
    }
}
