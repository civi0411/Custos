//! Human-In-The-Loop Approval Contracts
//!
//! When an Action exceeds autonomous risk boundaries, an ApprovalRequest is created
//! and execution is suspended until an ApprovalDecision is recorded.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub description: String,
    pub requested_capability: String,
    pub risk_level: String,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ApprovalRequest {
    pub fn new(
        task_id: String,
        action_id: String,
        description: String,
        requested_capability: String,
        risk_level: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: new_id("appr"),
            task_id,
            action_id,
            description,
            requested_capability,
            risk_level,
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub request_id: String,
    pub approved: bool,
    pub decided_by: String,
    pub rationale: Option<String>,
    pub decided_at: DateTime<Utc>,
}

impl ApprovalDecision {
    pub fn approve(request_id: String, decided_by: String, rationale: Option<String>) -> Self {
        Self {
            request_id,
            approved: true,
            decided_by,
            rationale,
            decided_at: Utc::now(),
        }
    }

    pub fn reject(request_id: String, decided_by: String, rationale: Option<String>) -> Self {
        Self {
            request_id,
            approved: false,
            decided_by,
            rationale,
            decided_at: Utc::now(),
        }
    }

    pub fn apply_to(&self, request: &mut ApprovalRequest) -> Result<(), DomainError> {
        if self.request_id != request.id {
            return Err(DomainError::Conflict(format!(
                "Approval decision for {} cannot resolve request {}",
                self.request_id, request.id
            )));
        }
        if request.status != ApprovalStatus::Pending {
            return Err(DomainError::Conflict(format!(
                "Approval request {} is already resolved as {:?}",
                request.id, request.status
            )));
        }
        if request
            .expires_at
            .is_some_and(|expires_at| self.decided_at >= expires_at)
        {
            request.status = ApprovalStatus::TimedOut;
            return Err(DomainError::Conflict(format!(
                "Approval request {} expired before the decision was recorded",
                request.id
            )));
        }
        request.status = if self.approved {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Rejected
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_request(expires_at: Option<DateTime<Utc>>) -> ApprovalRequest {
        ApprovalRequest::new(
            "task_1".into(),
            "action_1".into(),
            "Write a file".into(),
            "workspace.write".into(),
            "high".into(),
            expires_at,
        )
    }

    #[test]
    fn decision_must_match_request_identity() {
        let mut request = pending_request(None);
        let decision = ApprovalDecision::approve("appr_other".into(), "operator".into(), None);

        assert!(matches!(
            decision.apply_to(&mut request),
            Err(DomainError::Conflict(_))
        ));
        assert_eq!(request.status, ApprovalStatus::Pending);
    }

    #[test]
    fn expired_request_cannot_be_approved() {
        let mut request = pending_request(Some(Utc::now() - chrono::Duration::seconds(1)));
        let decision = ApprovalDecision::approve(request.id.clone(), "operator".into(), None);

        assert!(matches!(
            decision.apply_to(&mut request),
            Err(DomainError::Conflict(_))
        ));
        assert_eq!(request.status, ApprovalStatus::TimedOut);
    }

    #[test]
    fn matching_unexpired_decision_resolves_request_once() {
        let mut request = pending_request(Some(Utc::now() + chrono::Duration::minutes(1)));
        let decision = ApprovalDecision::approve(request.id.clone(), "operator".into(), None);

        assert!(decision.apply_to(&mut request).is_ok());
        assert_eq!(request.status, ApprovalStatus::Approved);
        assert!(decision.apply_to(&mut request).is_err());
    }
}
