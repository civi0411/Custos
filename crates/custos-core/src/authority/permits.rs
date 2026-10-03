//! Execution Permit Issuance and Verification
//!
//! Permits are short-lived, unforgeable credentials required to execute side effects.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_domain::{DomainError, Permit, RiskClass};

#[derive(Debug, Default, Clone)]
pub struct PermitIssuer {
    permits: Arc<RwLock<HashMap<String, Permit>>>,
}

impl PermitIssuer {
    pub fn new() -> Self {
        Self::default()
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
        let mut guard = self.permits.write().expect("lock poisoned");
        guard.insert(permit.id.clone(), permit.clone());
        permit
    }

    pub fn verify_permit(&self, permit_id: &str) -> Result<Permit, DomainError> {
        let guard = self.permits.read().expect("lock poisoned");
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
        let mut guard = self.permits.write().expect("lock poisoned");
        let permit = guard.get_mut(permit_id).ok_or_else(|| DomainError::NotFound {
            kind: "Permit".into(),
            id: permit_id.into(),
        })?;

        permit.consume(expected_digest)?;
        Ok(permit.clone())
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
}
