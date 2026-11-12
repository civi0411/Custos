use super::OutboxRepository;
use async_trait::async_trait;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::{DomainError, ExecutionReceipt};
use rusqlite::{params, OptionalExtension};

impl OutboxRepository {
    pub fn outbox_status_as_str(status: OutboxStatus) -> &'static str {
        match status {
            OutboxStatus::Pending => "pending",
            OutboxStatus::Approved => "approved",
            OutboxStatus::Dispatching => "dispatching",
            OutboxStatus::Receipted => "receipted",
            OutboxStatus::Failed => "failed",
            OutboxStatus::Uncertain => "uncertain",
        }
    }

    pub(crate) fn row_to_outbox_entry(row: &rusqlite::Row) -> Result<OutboxEntry, rusqlite::Error> {
        let id: String = row.get(0)?;
        let task_id: String = row.get(1)?;
        let action_id: String = row.get(2)?;
        let permit_id: String = row.get(3)?;
        let argument_digest: String = row.get(4)?;
        let idempotency_key: Option<String> = row.get(5)?;
        let status_str: String = row.get(6)?;
        let created_at_str: String = row.get(7)?;
        let receipt_json: Option<String> = row.get(8)?;

        let status = match status_str.as_str() {
            "pending" => OutboxStatus::Pending,
            "approved" => OutboxStatus::Approved,
            "dispatching" => OutboxStatus::Dispatching,
            "receipted" => OutboxStatus::Receipted,
            "failed" => OutboxStatus::Failed,
            _ => OutboxStatus::Uncertain,
        };

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let receipt = receipt_json.and_then(|s| serde_json::from_str(&s).ok());
        let recipient: Option<String> = row.get(9).ok();
        let payload_json: Option<String> = row.get(10).ok();
        let error_message: Option<String> = row.get(11).ok();

        Ok(OutboxEntry {
            id,
            task_id,
            action_id,
            permit_id,
            argument_digest,
            idempotency_key,
            status,
            created_at,
            receipt,
            recipient,
            payload_json,
            error_message,
        })
    }
}

#[async_trait]
impl OutboxPort for OutboxRepository {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let receipt_json = entry
            .receipt
            .as_ref()
            .map(|r| serde_json::to_string(r).map_err(|e| DomainError::Validation(e.to_string())))
            .transpose()?;

        conn.execute(
            "INSERT INTO outbox_entries (id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json, recipient, payload_json, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.id,
                entry.task_id,
                entry.action_id,
                entry.permit_id,
                entry.argument_digest,
                entry.idempotency_key,
                Self::outbox_status_as_str(entry.status),
                entry.created_at.to_rfc3339(),
                receipt_json,
                entry.recipient,
                entry.payload_json,
                entry.error_message,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
        let conn = self.db.lock()?;
        let entry = conn
            .query_row(
                "SELECT id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json, recipient, payload_json, error_message
                 FROM outbox_entries WHERE id = ?1",
                params![id],
                Self::row_to_outbox_entry,
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(entry)
    }

    async fn mark_approved(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let rows = conn.execute(
            "UPDATE outbox_entries SET status = 'approved' WHERE id = ?1 AND status = 'pending'",
            params![id],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        if rows != 1 {
            return Err(DomainError::Conflict(format!(
                "Outbox entry '{id}' is not pending approval"
            )));
        }
        Ok(())
    }

    /// CAS state transition: Approved -> Dispatching
    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let rows = conn
            .execute(
                "UPDATE outbox_entries SET status = 'dispatching' WHERE id = ?1 AND status = 'approved'",
                params![id],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if rows == 0 {
            let current: Option<String> = conn
                .query_row(
                    "SELECT status FROM outbox_entries WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            match current {
                Some(status) => {
                    return Err(DomainError::InvalidStateTransition {
                        from: status,
                        to: "dispatching".into(),
                    })
                }
                None => {
                    return Err(DomainError::NotFound {
                        kind: "OutboxEntry".into(),
                        id: id.to_string(),
                    })
                }
            }
        }

        Ok(())
    }

    /// CAS state transition: Dispatching | Uncertain -> Receipted
    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let receipt_json =
            serde_json::to_string(&receipt).map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = conn
            .execute(
                "UPDATE outbox_entries SET status = 'receipted', receipt_json = ?1 WHERE id = ?2 AND status IN ('dispatching', 'uncertain')",
                params![receipt_json, id],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if rows == 0 {
            let current: Option<String> = conn
                .query_row(
                    "SELECT status FROM outbox_entries WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            match current {
                Some(status) => {
                    return Err(DomainError::InvalidStateTransition {
                        from: status,
                        to: "receipted".into(),
                    })
                }
                None => {
                    return Err(DomainError::NotFound {
                        kind: "OutboxEntry".into(),
                        id: id.to_string(),
                    })
                }
            }
        }

        Ok(())
    }

    /// CAS state transition: Pending | Dispatching -> Uncertain
    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let rows = conn
            .execute(
                "UPDATE outbox_entries SET status = 'uncertain' WHERE id = ?1 AND status IN ('pending', 'approved', 'dispatching')",
                params![id],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if rows == 0 {
            let current: Option<String> = conn
                .query_row(
                    "SELECT status FROM outbox_entries WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            match current {
                Some(status) if status == "uncertain" => return Ok(()),
                Some(status) => {
                    return Err(DomainError::InvalidStateTransition {
                        from: status,
                        to: "uncertain".into(),
                    })
                }
                None => {
                    return Err(DomainError::NotFound {
                        kind: "OutboxEntry".into(),
                        id: id.to_string(),
                    })
                }
            }
        }

        Ok(())
    }

    /// CAS state transition: Pending | Dispatching | Uncertain -> Failed
    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let rows = conn
            .execute(
                "UPDATE outbox_entries SET status = 'failed', error_message = ?1 WHERE id = ?2 AND status IN ('pending', 'approved', 'dispatching', 'uncertain')",
                params![error, id],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if rows == 0 {
            let current: Option<String> = conn
                .query_row(
                    "SELECT status FROM outbox_entries WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            match current {
                Some(status) if status == "failed" => return Ok(()),
                Some(status) => {
                    return Err(DomainError::InvalidStateTransition {
                        from: status,
                        to: "failed".into(),
                    })
                }
                None => {
                    return Err(DomainError::NotFound {
                        kind: "OutboxEntry".into(),
                        id: id.to_string(),
                    })
                }
            }
        }

        Ok(())
    }

    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json, recipient, payload_json, error_message
                 FROM outbox_entries WHERE status = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status_str = Self::outbox_status_as_str(status);
        let rows = stmt
            .query_map(params![status_str], Self::row_to_outbox_entry)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(results)
    }

    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json, recipient, payload_json, error_message
                 FROM outbox_entries WHERE task_id = ?1 AND status = ?2 ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let status_str = Self::outbox_status_as_str(status);
        let rows = stmt
            .query_map(params![task_id, status_str], Self::row_to_outbox_entry)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(results)
    }
}

