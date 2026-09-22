//! Human Approval Manager
//!
//! Tracks and resolves approval requests for high-risk operations.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use custos_core_domain::{ApprovalDecision, ApprovalRequest, ApprovalStatus, DomainError};

#[derive(Debug, Default, Clone)]
pub struct ApprovalManager {
    requests: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_approval(&self, req: ApprovalRequest) {
        let mut guard = self.requests.write().expect("lock poisoned");
        guard.insert(req.id.clone(), req);
    }

    pub fn get(&self, id: &str) -> Option<ApprovalRequest> {
        let guard = self.requests.read().expect("lock poisoned");
        guard.get(id).cloned()
    }

    pub fn list_pending(&self, task_id: &str) -> Vec<ApprovalRequest> {
        let guard = self.requests.read().expect("lock poisoned");
        guard
            .values()
            .filter(|r| r.task_id == task_id && r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn decide(&self, decision: &ApprovalDecision) -> Result<ApprovalRequest, DomainError> {
        let mut guard = self.requests.write().expect("lock poisoned");
        let req = guard
            .get_mut(&decision.request_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "ApprovalRequest".into(),
                id: decision.request_id.clone(),
            })?;

        decision.apply_to(req)?;
        Ok(req.clone())
    }
}
