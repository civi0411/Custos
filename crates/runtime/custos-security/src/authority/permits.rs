//! Execution Permit Issuance and Verification
//!
//! Permits are short-lived, unforgeable credentials required to execute side effects.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_core_domain::{DomainError, Permit, RiskClass};

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
        let permit = Permit::new(
            task_id,
            action_id,
            capability,
            grant_id,
            risk_class,
            ttl_seconds,
        );
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
}
