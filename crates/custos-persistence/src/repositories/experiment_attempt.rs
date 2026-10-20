//! Experiment Attempt & Promotion Repository (F10 / P3).
//!
//! Provides single-writer SQLite persistence for `ExecutionAttempt`, idempotency-keyed `Invocation`s,
//! generation-CAS checkpoint promotions (fencing stale writers), and startup crash reconciliation.

use std::sync::{Arc, Mutex};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use custos_core::contracts::checkpoint_store::PromotionOutcome;
use custos_domain::{
    new_id, DomainError, ExecState, ExecutionAttempt, Invocation, InvocationOutcome, ResourceState,
};

#[derive(Clone)]
pub struct ExperimentAttemptRepository {
    conn: Arc<Mutex<Connection>>,
}

impl ExperimentAttemptRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Saves or updates an ExecutionAttempt atomically with an outbox event
    pub fn save_attempt(&self, attempt: &ExecutionAttempt) -> Result<(), DomainError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| DomainError::Validation(e.to_string()))?;

        let attempt_json = serde_json::to_string(attempt)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let ext_json = attempt
            .external_ref
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let now = Utc::now().timestamp();

        tx.execute(
            "INSERT INTO experiment_attempts (
                id, experiment_id, recipe_id, backend, external_ref_json, exec_state,
                resource_state, parent_checkpoint_id, generation, output_prefix,
                attempt_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                exec_state = excluded.exec_state,
                resource_state = excluded.resource_state,
                external_ref_json = excluded.external_ref_json,
                generation = excluded.generation,
                attempt_json = excluded.attempt_json,
                updated_at = excluded.updated_at",
            params![
                attempt.id,
                attempt.experiment_id,
                attempt.recipe_id,
                attempt.backend.to_string(),
                ext_json,
                format!("{:?}", attempt.exec_state).to_lowercase(),
                format!("{:?}", attempt.resource_state).to_lowercase(),
                attempt.parent_checkpoint_id,
                attempt.generation,
                attempt.output_prefix,
                attempt_json,
                now,
                now,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        // Outbox event
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                new_id("attempt_event"),
                "experiment.attempt.saved",
                serde_json::json!({
                    "attempt_id": attempt.id,
                    "experiment_id": attempt.experiment_id,
                    "exec_state": format!("{:?}", attempt.exec_state).to_lowercase(),
                    "resource_state": format!("{:?}", attempt.resource_state).to_lowercase(),
                    "generation": attempt.generation
                })
                .to_string(),
                now
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit().map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Fetches an attempt by ID
    pub fn get_attempt(&self, id: &str) -> Result<Option<ExecutionAttempt>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let json_opt: Option<String> = conn
            .query_row(
                "SELECT attempt_json FROM experiment_attempts WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match json_opt {
            Some(json) => {
                let attempt = serde_json::from_str(&json)
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                Ok(Some(attempt))
            }
            None => Ok(None),
        }
    }

    /// Lists all attempts for an experiment ordered chronologically
    pub fn list_attempts_for_experiment(
        &self,
        experiment_id: &str,
    ) -> Result<Vec<ExecutionAttempt>, DomainError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT attempt_json FROM experiment_attempts WHERE experiment_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![experiment_id], |row| row.get::<_, String>(0))
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut attempts = Vec::new();
        for r in rows {
            let json = r.map_err(|e| DomainError::Validation(e.to_string()))?;
            let attempt: ExecutionAttempt = serde_json::from_str(&json)
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            attempts.push(attempt);
        }
        Ok(attempts)
    }

    /// Records an Invocation with strict idempotency key uniqueness
    pub fn record_invocation(&self, invocation: &Invocation) -> Result<(), DomainError> {
        let conn = self.conn.lock().unwrap();
        let outcome_str = match invocation.outcome {
            InvocationOutcome::Ok => "ok",
            InvocationOutcome::Err => "err",
            InvocationOutcome::Unknown => "unknown",
        };
        let now = Utc::now().timestamp();

        let affected = conn
            .execute(
                "INSERT INTO attempt_invocations (
                    id, attempt_id, op, idempotency_key, argv_fingerprint, outcome, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(idempotency_key) DO NOTHING",
                params![
                    invocation.id,
                    invocation.attempt_id,
                    invocation.op,
                    invocation.idempotency_key,
                    invocation.argv_fingerprint,
                    outcome_str,
                    now,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if affected == 0 {
            return Err(DomainError::Conflict(format!(
                "Invocation with idempotency key '{}' already exists",
                invocation.idempotency_key
            )));
        }
        Ok(())
    }

    /// Promotes a checkpoint with generation CAS fencing (D4: stale writers rejected)
    pub fn promote_checkpoint(
        &self,
        experiment_id: &str,
        expected_generation: u64,
        checkpoint_id: &str,
        canonical_step: u64,
    ) -> Result<PromotionOutcome, DomainError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| DomainError::Validation(e.to_string()))?;

        // Query current max generation promoted for this experiment
        let current_max_gen: Option<u64> = tx
            .query_row(
                "SELECT MAX(generation) FROM checkpoint_promotions WHERE experiment_id = ?1",
                params![experiment_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .flatten();

        if let Some(cur) = current_max_gen {
            if expected_generation < cur {
                return Ok(PromotionOutcome::StaleGeneration { actual_generation: cur });
            }
        }

        let now = Utc::now().timestamp();
        let insert_res = tx.execute(
            "INSERT INTO checkpoint_promotions (experiment_id, generation, checkpoint_id, promoted_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![experiment_id, expected_generation, checkpoint_id, now],
        );

        match insert_res {
            Ok(_) => {
                tx.commit().map_err(|e| DomainError::Validation(e.to_string()))?;
                Ok(PromotionOutcome::Promoted { canonical_step })
            }
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Ok(PromotionOutcome::StaleGeneration {
                    actual_generation: expected_generation,
                })
            }
            Err(e) => Err(DomainError::Validation(e.to_string())),
        }
    }

    /// Reconciles in-flight attempts on daemon crash/startup:
    /// In-flight states -> Uncertain, ReleaseRequested -> Unknown
    pub fn reconcile_attempts_on_startup(&self) -> Result<usize, DomainError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut stmt = tx
            .prepare(
                "SELECT id, attempt_json FROM experiment_attempts
                 WHERE exec_state IN ('submitting', 'submitted', 'running', 'stopping')
                    OR resource_state = 'releaserequested'",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        drop(stmt);

        let mut count = 0;
        let now = Utc::now().timestamp();

        for (id, json) in rows {
            let mut attempt: ExecutionAttempt = serde_json::from_str(&json)
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            if matches!(
                attempt.exec_state,
                ExecState::Submitting | ExecState::Submitted | ExecState::Running | ExecState::Stopping
            ) {
                attempt.exec_state = ExecState::Uncertain;
            }
            if attempt.resource_state == ResourceState::ReleaseRequested {
                attempt.resource_state = ResourceState::Unknown;
            }

            let updated_json = serde_json::to_string(&attempt)
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            tx.execute(
                "UPDATE experiment_attempts
                 SET exec_state = ?1, resource_state = ?2, attempt_json = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![
                    format!("{:?}", attempt.exec_state).to_lowercase(),
                    format!("{:?}", attempt.resource_state).to_lowercase(),
                    updated_json,
                    now,
                    id,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

            count += 1;
        }

        tx.commit().map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::ComputeBackendKind;

    fn in_memory_repo() -> ExperimentAttemptRepository {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        ExperimentAttemptRepository::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_attempt_crud_and_outbox() {
        let repo = in_memory_repo();
        let attempt = ExecutionAttempt::new("exp_1", "recipe_1", ComputeBackendKind::Local, None);
        repo.save_attempt(&attempt).unwrap();

        let fetched = repo.get_attempt(&attempt.id).unwrap().expect("Must find attempt");
        assert_eq!(fetched.id, attempt.id);
        assert_eq!(fetched.experiment_id, "exp_1");
        assert_eq!(fetched.exec_state, ExecState::Planned);

        let list = repo.list_attempts_for_experiment("exp_1").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, attempt.id);
    }

    #[test]
    fn test_invocation_idempotency_conflict() {
        let repo = in_memory_repo();
        let attempt = ExecutionAttempt::new("exp_1", "recipe_1", ComputeBackendKind::Local, None);
        repo.save_attempt(&attempt).unwrap();

        let inv = Invocation {
            id: new_id("inv"),
            attempt_id: attempt.id.clone(),
            op: "submit".into(),
            idempotency_key: "idem_123".into(),
            argv_fingerprint: "sha256:abc".into(),
            outcome: InvocationOutcome::Ok,
        };
        repo.record_invocation(&inv).unwrap();

        // Duplicate idempotency key must return Conflict error
        let dup = Invocation {
            id: new_id("inv"),
            attempt_id: attempt.id.clone(),
            op: "submit".into(),
            idempotency_key: "idem_123".into(),
            argv_fingerprint: "sha256:abc".into(),
            outcome: InvocationOutcome::Ok,
        };
        assert!(repo.record_invocation(&dup).is_err());
    }

    #[test]
    fn test_d4_checkpoint_promotion_generation_fencing() {
        let repo = in_memory_repo();
        let exp = "exp_fencing";

        // Generation 1 promotes successfully
        let res1 = repo.promote_checkpoint(exp, 1, "ckpt_1", 100).unwrap();
        assert_eq!(res1, PromotionOutcome::Promoted { canonical_step: 100 });

        // Generation 2 promotes successfully
        let res2 = repo.promote_checkpoint(exp, 2, "ckpt_2", 200).unwrap();
        assert_eq!(res2, PromotionOutcome::Promoted { canonical_step: 200 });

        // D4: Stale writer with Generation 1 tries to promote -> rejected with StaleGeneration
        let stale = repo.promote_checkpoint(exp, 1, "ckpt_stale", 150).unwrap();
        assert_eq!(stale, PromotionOutcome::StaleGeneration { actual_generation: 2 });
    }

    #[test]
    fn test_crash_reconciliation_to_uncertain() {
        let repo = in_memory_repo();
        let mut attempt = ExecutionAttempt::new("exp_crash", "recipe_1", ComputeBackendKind::Local, None);
        attempt.exec_state = ExecState::Running;
        attempt.resource_state = ResourceState::ReleaseRequested;
        repo.save_attempt(&attempt).unwrap();

        // Reconcile on crash
        let count = repo.reconcile_attempts_on_startup().unwrap();
        assert_eq!(count, 1);

        let reconciled = repo.get_attempt(&attempt.id).unwrap().unwrap();
        assert_eq!(reconciled.exec_state, ExecState::Uncertain);
        assert_eq!(reconciled.resource_state, ResourceState::Unknown);
    }
}
