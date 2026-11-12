//! Execution Permit Issuance and Verification (INV-04)
//!
//! Permits are short-lived, unforgeable credentials required to execute side effects.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_domain::{DomainError, Permit, RiskClass};

/// Optional durable persistence hook for sovereign execution permits (INV-04)
pub trait PermitStore: Send + Sync {
    fn record_issued(&self, permit: &Permit) -> Result<(), DomainError>;
    fn record_consumed(&self, permit: &Permit) -> Result<(), DomainError>;
}

#[derive(Default, Clone)]
pub struct PermitIssuer {
    permits: Arc<RwLock<HashMap<String, Permit>>>,
    store: Option<Arc<dyn PermitStore>>,
}

impl std::fmt::Debug for PermitIssuer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermitIssuer")
            .field("has_store", &self.store.is_some())
            .finish()
    }
}

impl PermitIssuer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches an optional durable storage sink to persist permit lifecycle events
    pub fn with_store(mut self, store: Arc<dyn PermitStore>) -> Self {
        self.store = Some(store);
        self
    }

    fn acquire_write(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, Permit>> {
        match self.permits.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("PermitIssuer write lock recovered from poison");
                poisoned.into_inner()
            }
        }
    }

    fn acquire_read(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, Permit>> {
        match self.permits.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("PermitIssuer read lock recovered from poison");
                poisoned.into_inner()
            }
        }
    }

    pub fn issue_permit(
        &self,
        task_id: String,
        action_id: String,
        capability: String,
        grant_id: Option<String>,
        risk_class: RiskClass,
        ttl_seconds: i64,
    ) -> Permit {
        self.issue_permit_with_digest(
            task_id,
            action_id,
            capability,
            grant_id,
            risk_class,
            String::new(),
            ttl_seconds,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn issue_permit_with_digest(
        &self,
        task_id: String,
        action_id: String,
        capability: String,
        grant_id: Option<String>,
        risk_class: RiskClass,
        argument_digest: String,
        ttl_seconds: i64,
    ) -> Permit {
        let mut permit = Permit::new(
            task_id,
            action_id,
            capability,
            grant_id,
            risk_class,
            ttl_seconds,
        );
        if !argument_digest.is_empty() {
            permit = permit.with_argument_digest(argument_digest);
        }
        let mut guard = self.acquire_write();
        guard.insert(permit.id.clone(), permit.clone());
        if let Some(ref store) = self.store {
            if let Err(e) = store.record_issued(&permit) {
                tracing::warn!(permit_id = %permit.id, "Failed to persist issued permit to durable store: {e}");
            }
        }
        permit
    }

    pub fn verify_permit(&self, permit_id: &str) -> Result<Permit, DomainError> {
        let guard = self.acquire_read();
        let permit = guard.get(permit_id).ok_or_else(|| DomainError::NotFound {
            kind: "Permit".into(),
            id: permit_id.into(),
        })?;

        permit.verify_active()?;
        Ok(permit.clone())
    }

    /// Atomically consumes/burns the permit, verifying expiration, single-use status,
    /// and cryptographic argument digest matching (INV-04 Confused Deputy and Replay prevention).
    pub fn consume_permit(
        &self,
        permit_id: &str,
        expected_digest: &str,
    ) -> Result<Permit, DomainError> {
        let mut guard = self.acquire_write();
        let permit = guard
            .get_mut(permit_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "Permit".into(),
                id: permit_id.into(),
            })?;

        permit.consume(expected_digest)?;
        let consumed = permit.clone();
        if let Some(ref store) = self.store {
            if let Err(e) = store.record_consumed(&consumed) {
                tracing::warn!(permit_id = %consumed.id, "Failed to persist consumed permit state to durable store: {e}");
            }
        }
        Ok(consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permit_issuer_consume_and_single_use() {
        let issuer = PermitIssuer::new();
        let digest = "sha256:abc123expected";

        let permit = issuer.issue_permit_with_digest(
            "task_01".into(),
            "act_01".into(),
            "read_file".into(),
            None,
            RiskClass::Low,
            digest.into(),
            300,
        );

        // Mismatched digest fails
        let mismatch_res = issuer.consume_permit(&permit.id, "sha256:wrong_digest");
        assert!(mismatch_res.is_err());

        // First consume succeeds
        let consumed = issuer.consume_permit(&permit.id, digest);
        assert!(consumed.is_ok());
        assert!(consumed.unwrap().used_at.is_some());

        // Replay consume fails
        let replay_res = issuer.consume_permit(&permit.id, digest);
        assert!(replay_res.is_err());
    }

    struct MockPermitStore {
        issued: Arc<RwLock<Vec<String>>>,
        consumed: Arc<RwLock<Vec<String>>>,
    }

    impl PermitStore for MockPermitStore {
        fn record_issued(&self, permit: &Permit) -> Result<(), DomainError> {
            self.issued.write().unwrap().push(permit.id.clone());
            Ok(())
        }
        fn record_consumed(&self, permit: &Permit) -> Result<(), DomainError> {
            self.consumed.write().unwrap().push(permit.id.clone());
            Ok(())
        }
    }

    #[test]
    fn test_permit_issuer_durable_store_and_poison_resilience() {
        let store = Arc::new(MockPermitStore {
            issued: Arc::new(RwLock::new(Vec::new())),
            consumed: Arc::new(RwLock::new(Vec::new())),
        });

        let issuer = PermitIssuer::new().with_store(store.clone());
        let digest = "sha256:durable_digest";

        let permit = issuer.issue_permit_with_digest(
            "t1".into(),
            "a1".into(),
            "fs_write".into(),
            None,
            RiskClass::High,
            digest.into(),
            60,
        );

        assert_eq!(store.issued.read().unwrap().len(), 1);
        assert_eq!(store.issued.read().unwrap()[0], permit.id);

        let consumed = issuer.consume_permit(&permit.id, digest).unwrap();
        assert_eq!(store.consumed.read().unwrap().len(), 1);
        assert_eq!(store.consumed.read().unwrap()[0], consumed.id);

        let permit2 = issuer.issue_permit_with_digest(
            "t2".into(),
            "a2".into(),
            "fs_read".into(),
            None,
            RiskClass::Low,
            "".into(),
            60,
        );

        // Verify lock poison resilience: simulate a poisoned lock
        let permits_clone = issuer.permits.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = permits_clone.write().unwrap();
            panic!("Simulated thread panic while holding permit write lock");
        }));

        assert!(issuer.permits.is_poisoned());

        // Verifying and consuming permits should NOT panic despite poisoned lock
        let verify_result = issuer.verify_permit(&permit2.id);
        assert!(verify_result.is_ok());
    }
}
