use custos_domain::{DomainError, LaunchAttempt, LaunchStatus};
use rusqlite::{params, OptionalExtension};

use super::RunRepository;

impl RunRepository {
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

    pub fn get_launch_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<LaunchAttempt>, DomainError> {
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

    pub(crate) fn parse_launch_status(s: &str) -> LaunchStatus {
        match s {
            "prepared" => LaunchStatus::Prepared,
            "launched" => LaunchStatus::Launched,
            "refused" => LaunchStatus::Refused,
            "unknown" => LaunchStatus::Unknown,
            "failed" => LaunchStatus::Failed,
            _ => LaunchStatus::Unknown,
        }
    }

    pub(crate) fn map_launch_attempt(row: &rusqlite::Row) -> rusqlite::Result<LaunchAttempt> {
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
