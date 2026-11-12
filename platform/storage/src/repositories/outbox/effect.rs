use super::OutboxRepository;
use async_trait::async_trait;
use custos_core::contracts::storage::EffectLedgerPort;
use custos_domain::{DomainError, EffectAttempt, EffectStatus, ExecutionReceipt};
use rusqlite::{params, OptionalExtension};

impl OutboxRepository {
    pub fn effect_status_as_str(status: EffectStatus) -> &'static str {
        match status {
            EffectStatus::Pending => "pending",
            EffectStatus::InFlight => "in_flight",
            EffectStatus::Succeeded => "succeeded",
            EffectStatus::Failed => "failed",
            EffectStatus::Uncertain => "uncertain",
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

