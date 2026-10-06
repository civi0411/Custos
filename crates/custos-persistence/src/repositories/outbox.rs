use crate::connection::DbConnection;
use async_trait::async_trait;
use custos_core::contracts::storage::{EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::{DomainError, EffectAttempt, EffectStatus, ExecutionReceipt};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct OutboxRepository {
    db: DbConnection,
}

impl OutboxRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    pub fn effect_status_as_str(status: EffectStatus) -> &'static str {
        match status {
            EffectStatus::Pending => "pending",
            EffectStatus::InFlight => "in_flight",
            EffectStatus::Succeeded => "succeeded",
            EffectStatus::Failed => "failed",
            EffectStatus::Uncertain => "uncertain",
        }
    }

    pub fn outbox_status_as_str(status: OutboxStatus) -> &'static str {
        match status {
            OutboxStatus::Pending => "pending",
            OutboxStatus::Dispatching => "dispatching",
            OutboxStatus::Receipted => "receipted",
            OutboxStatus::Failed => "failed",
            OutboxStatus::Uncertain => "uncertain",
        }
    }

    pub fn record_effect(&self, effect: &EffectAttempt) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let receipt_json = effect
            .receipt
            .as_ref()
            .map(|r| serde_json::to_string(r).map_err(|e| DomainError::Validation(e.to_string())))
            .transpose()?;

        let res = conn.execute(
            "INSERT INTO effect_attempts (id, node_attempt_id, permit_id, idempotency_key, status, started_at, ended_at, receipt_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                effect.id,
                effect.node_attempt_id,
                effect.permit_id,
                effect.idempotency_key,
                Self::effect_status_as_str(effect.status),
                effect.started_at.to_rfc3339(),
                effect.ended_at.map(|dt| dt.to_rfc3339()),
                receipt_json,
            ],
        );

        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(_err, Some(msg))) if msg.contains("UNIQUE") => {
                Err(DomainError::Conflict(format!(
                    "Effect with idempotency key '{}' already recorded (Gate 3 Idempotency Enforcement)",
                    effect.idempotency_key
                )))
            }
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    pub fn update_effect_status(
        &self,
        id: &str,
        status: EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let receipt_json = receipt
            .map(|r| serde_json::to_string(r).map_err(|e| DomainError::Validation(e.to_string())))
            .transpose()?;

        let ended_at = if status.is_terminal() {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        conn.execute(
            "UPDATE effect_attempts SET status = ?1, ended_at = coalesce(?2, ended_at), receipt_json = coalesce(?3, receipt_json) WHERE id = ?4",
            params![
                Self::effect_status_as_str(status),
                ended_at,
                receipt_json,
                id,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<EffectAttempt>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_attempt_id, permit_id, idempotency_key, status, started_at, ended_at, receipt_json
                 FROM effect_attempts WHERE idempotency_key = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let res = stmt
            .query_row(params![key], |row| {
                let id: String = row.get(0)?;
                let node_attempt_id: String = row.get(1)?;
                let permit_id: String = row.get(2)?;
                let idempotency_key: String = row.get(3)?;
                let status_str: String = row.get(4)?;
                let started_at_str: String = row.get(5)?;
                let ended_at_str: Option<String> = row.get(6)?;
                let receipt_json: Option<String> = row.get(7)?;

                let status = match status_str.as_str() {
                    "pending" => EffectStatus::Pending,
                    "in_flight" | "inflight" => EffectStatus::InFlight,
                    "succeeded" => EffectStatus::Succeeded,
                    "failed" => EffectStatus::Failed,
                    _ => EffectStatus::Uncertain,
                };

                let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let ended_at = ended_at_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                });

                let receipt = receipt_json.and_then(|json| serde_json::from_str(&json).ok());

                Ok(EffectAttempt {
                    id,
                    node_attempt_id,
                    permit_id,
                    idempotency_key,
                    status,
                    started_at,
                    ended_at,
                    receipt,
                })
            })
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(res)
    }

    /// Reconciles all in-flight effect attempts and dispatching outbox entries to Uncertain on daemon restart (Gate 3).
    pub fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
        let conn = self.db.lock()?;
        let c1 = conn
            .execute(
                "UPDATE effect_attempts SET status = 'uncertain' WHERE status = 'in_flight' OR status = 'inflight'",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let c2 = conn
            .execute(
                "UPDATE outbox_entries SET status = 'uncertain' WHERE status = 'dispatching'",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(c1 + c2)
    }

    /// Reconciles all in-flight effect attempts to Uncertain on crash recovery (Gate 3).
    pub fn reconcile_inflight_to_uncertain(&self) -> Result<usize, DomainError> {
        self.reconcile_on_startup()
    }

    fn row_to_outbox_entry(row: &rusqlite::Row) -> Result<OutboxEntry, rusqlite::Error> {
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
            "dispatching" => OutboxStatus::Dispatching,
            "receipted" => OutboxStatus::Receipted,
            "failed" => OutboxStatus::Failed,
            _ => OutboxStatus::Uncertain,
        };

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let receipt = receipt_json.and_then(|s| serde_json::from_str(&s).ok());

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
        })
    }
}

#[async_trait]
impl EffectLedgerPort for OutboxRepository {
    async fn record_effect(&self, effect: &EffectAttempt) -> Result<(), DomainError> {
        self.record_effect(effect)
    }

    async fn update_effect_status(
        &self,
        id: &str,
        status: EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError> {
        self.update_effect_status(id, status, receipt)
    }

    async fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<EffectAttempt>, DomainError> {
        self.get_effect_by_idempotency_key(key)
    }

    async fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
        self.reconcile_on_startup()
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
            "INSERT INTO outbox_entries (id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    /// CAS state transition: Pending -> Dispatching
    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let rows = conn
            .execute(
                "UPDATE outbox_entries SET status = 'dispatching' WHERE id = ?1 AND status = 'pending'",
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
                "UPDATE outbox_entries SET status = 'uncertain' WHERE id = ?1 AND status IN ('pending', 'dispatching')",
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

    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json
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
                "SELECT id, task_id, action_id, permit_id, argument_digest, idempotency_key, status, created_at, receipt_json
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use custos_domain::{Assurance, ReceiptStatus};

    #[tokio::test]
    async fn test_effect_idempotency_unique_constraint_enforcement() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = OutboxRepository::new(db);

        let effect1 = EffectAttempt {
            id: "eff_01".into(),
            node_attempt_id: "node_att_01".into(),
            permit_id: "pmt_01".into(),
            idempotency_key: "idemp_unique_key_01".into(),
            status: EffectStatus::InFlight,
            started_at: Utc::now(),
            ended_at: None,
            receipt: None,
        };

        // First insert must succeed
        repo.record_effect(&effect1)
            .expect("first effect insert succeeds");

        let effect2 = EffectAttempt {
            id: "eff_02".into(),
            node_attempt_id: "node_att_02".into(),
            permit_id: "pmt_02".into(),
            idempotency_key: "idemp_unique_key_01".into(), // Duplicate idempotency key!
            status: EffectStatus::InFlight,
            started_at: Utc::now(),
            ended_at: None,
            receipt: None,
        };

        // Second insert with same idempotency key must be rejected with Conflict error
        let duplicate_res = repo.record_effect(&effect2);
        assert!(duplicate_res.is_err());
        assert!(
            matches!(duplicate_res.unwrap_err(), DomainError::Conflict(msg) if msg.contains("Gate 3 Idempotency Enforcement")),
            "Duplicate effect insert must fail with Gate 3 Conflict error"
        );
    }

    #[tokio::test]
    async fn test_outbox_cas_state_transitions() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = OutboxRepository::new(db);

        // First insert task row to satisfy foreign key constraint
        {
            let conn = repo.db.lock().unwrap();
            conn.execute(
                "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_cas_01', 'CAS Task', 'running', datetime('now'), datetime('now'))",
                [],
            ).unwrap();
        }

        let entry_id = "outbox_entry_cas_01";
        let entry = OutboxEntry {
            id: entry_id.into(),
            task_id: "task_cas_01".into(),
            action_id: "act_01".into(),
            permit_id: "pmt_01".into(),
            argument_digest: "sha256:test_digest".into(),
            idempotency_key: Some("idemp_cas_01".into()),
            status: OutboxStatus::Pending,
            created_at: Utc::now(),
            receipt: None,
        };

        repo.enqueue(entry).await.expect("enqueue succeeds");

        // 1. Pending -> Dispatching CAS succeeds
        repo.mark_dispatching(entry_id)
            .await
            .expect("mark dispatching succeeds");

        // 2. Calling mark_dispatching again on already Dispatching entry must fail CAS
        let err_double_dispatch = repo.mark_dispatching(entry_id).await.unwrap_err();
        assert!(
            matches!(err_double_dispatch, DomainError::InvalidStateTransition { ref from, ref to } if from == "dispatching" && to == "dispatching"),
            "Invalid CAS transition: {}",
            err_double_dispatch
        );

        // 3. Dispatching -> Receipted CAS succeeds
        let receipt = ExecutionReceipt {
            receipt_id: "rcpt_01".into(),
            permit_id: "pmt_01".into(),
            action_id: "act_01".into(),
            status: ReceiptStatus::Success,
            output_digest: "sha256:out".into(),
            output_data: None,
            error_message: None,
            duration_ms: Some(10),
            executed_at: Utc::now(),
            assurance: Assurance::CustosMediated,
        };

        repo.mark_receipted(entry_id, receipt)
            .await
            .expect("mark receipted succeeds");

        // 4. Calling mark_uncertain on already Receipted entry must fail CAS
        let err_uncertain_on_receipted = repo.mark_uncertain(entry_id).await.unwrap_err();
        assert!(
            matches!(err_uncertain_on_receipted, DomainError::InvalidStateTransition { ref from, ref to } if from == "receipted" && to == "uncertain"),
            "Invalid CAS transition: {}",
            err_uncertain_on_receipted
        );
    }
}
