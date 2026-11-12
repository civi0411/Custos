use custos_domain::{ClaimStatus, DispatchClaim, DomainError};
use rusqlite::{params, OptionalExtension};

use super::RunRepository;

impl RunRepository {
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
        let affected = match claim.status {
            ClaimStatus::Dispatched => conn.execute(
                "UPDATE dispatch_claims SET
                    status = ?2,
                    dispatched_at = ?3,
                    released_at = ?4,
                    worker_run_id = ?5
                 WHERE id = ?1 AND status = 'pending'",
                params![
                    claim.id,
                    claim.status.to_string(),
                    claim.dispatched_at.map(|dt| dt.to_rfc3339()),
                    claim.released_at.map(|dt| dt.to_rfc3339()),
                    claim.worker_run_id,
                ],
            ),
            ClaimStatus::Released => conn.execute(
                "UPDATE dispatch_claims SET
                    status = ?2,
                    dispatched_at = ?3,
                    released_at = ?4,
                    worker_run_id = ?5
                 WHERE id = ?1 AND status IN ('pending', 'dispatched')",
                params![
                    claim.id,
                    claim.status.to_string(),
                    claim.dispatched_at.map(|dt| dt.to_rfc3339()),
                    claim.released_at.map(|dt| dt.to_rfc3339()),
                    claim.worker_run_id,
                ],
            ),
            _ => conn.execute(
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
            ),
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        if affected == 0 {
            let existing: Option<String> = conn
                .query_row(
                    "SELECT status FROM dispatch_claims WHERE id = ?1",
                    params![claim.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            if let Some(current_status) = existing {
                if current_status == claim.status.to_string() {
                    return Ok(());
                }
                return Err(DomainError::Conflict(format!(
                    "DispatchClaim '{}' CAS conflict: cannot transition from status '{}' to '{}'",
                    claim.id, current_status, claim.status
                )));
            } else {
                return Err(DomainError::NotFound {
                    kind: "DispatchClaim".into(),
                    id: claim.id.clone(),
                });
            }
        }

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
            .query_row(
                params![task_id, node_id.unwrap_or("")],
                Self::map_dispatch_claim,
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(claim)
    }

    pub fn list_active_claims(&self) -> Result<Vec<DispatchClaim>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, node_id, assignee, depth, status, claimed_at, dispatched_at, released_at, worker_run_id
                 FROM dispatch_claims
                 WHERE status IN ('pending', 'dispatched')
                 ORDER BY claimed_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let claims = stmt
            .query_map([], Self::map_dispatch_claim)
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(claims)
    }

    pub fn fence_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE dispatch_claims SET status = 'released', released_at = ?1 WHERE id = ?2 AND status IN ('pending', 'dispatched')",
            params![now, claim_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(crate) fn parse_claim_status(s: &str) -> ClaimStatus {
        match s {
            "pending" => ClaimStatus::Pending,
            "dispatched" => ClaimStatus::Dispatched,
            "released" => ClaimStatus::Released,
            _ => ClaimStatus::Pending,
        }
    }

    pub(crate) fn map_dispatch_claim(row: &rusqlite::Row) -> rusqlite::Result<DispatchClaim> {
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
}
