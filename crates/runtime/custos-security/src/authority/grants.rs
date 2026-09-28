//! Capability Grants Store & Verification
//!
//! Stores active capability grants endowed upon tasks and agents.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_core_domain::Grant;

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

    pub fn find_active_grant(&self, task_id: &str, capability: &str) -> Option<Grant> {
        let guard = self.grants.read().expect("lock poisoned");
        guard
            .values()
            .find(|g| g.task_id == task_id && g.capability == capability && g.is_valid())
            .cloned()
    }
}
