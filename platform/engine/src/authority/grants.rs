//! Capability Grants Store & Verification
//!
//! Stores active capability grants endowed upon tasks and agents.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_domain::{Action, Grant, RiskLevel};

#[derive(Debug, Default, Clone)]
pub struct GrantStore {
    grants: Arc<RwLock<HashMap<String, Grant>>>,
}

impl GrantStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, grant: Grant) {
        let mut guard = self.grants.write().expect("lock poisoned");
        guard.insert(grant.id.clone(), grant);
    }

    pub fn get(&self, id: &str) -> Option<Grant> {
        let guard = self.grants.read().expect("lock poisoned");
        guard.get(id).cloned()
    }

    /// Capability discovery only; authorization must use `find_grant_for_action`.
    pub fn find_active_grant(&self, task_id: &str, capability: &str) -> Option<Grant> {
        let guard = self.grants.read().expect("lock poisoned");
        guard
            .values()
            .find(|g| g.task_id == task_id && g.capability == capability && g.is_valid())
            .cloned()
    }

    /// Match every constraint before selecting a grant; map order must not change authority.
    pub fn find_grant_for_action(
        &self,
        task_id: &str,
        action: &Action,
        actor: &str,
    ) -> Option<Grant> {
        if task_id.trim().is_empty() || actor.trim().is_empty() {
            return None;
        }
        let guard = self.grants.read().ok()?;
        guard
            .values()
            .find(|grant| {
                grant.task_id == task_id
                    && grant.capability == action.name
                    && (grant.grantee == actor || grant.grantee == "*")
                    && grant.is_valid()
                    && constraints_match(grant, action)
            })
            .cloned()
    }
}

fn constraints_match(grant: &Grant, action: &Action) -> bool {
    let Some(constraints) = grant.constraints.as_object() else {
        return false;
    };
    constraints.iter().all(|(key, value)| match key.as_str() {
        "target" => value.as_str().is_some_and(|target| target == action.target),
        "allowed_targets" => value.as_array().is_some_and(|targets| {
            targets.iter().all(|target| target.is_string())
                && targets
                    .iter()
                    .any(|target| target.as_str() == Some(&action.target))
        }),
        "max_risk" => match value.as_str() {
            Some("low") => action.risk_level == RiskLevel::Low,
            Some("medium") => matches!(action.risk_level, RiskLevel::Low | RiskLevel::Medium),
            Some("high") => action.risk_level != RiskLevel::Critical,
            Some("critical") => true,
            _ => false,
        },
        "argument_digest" | "payload_digest" => value
            .as_str()
            .is_some_and(|digest| !digest.is_empty() && digest == action.argument_digest()),
        "status" => matches!(value.as_str(), Some("granted" | "approved")),
        // An unsupported constraint must never be interpreted as unrestricted authority.
        _ => false,
    })
}
