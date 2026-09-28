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
        if request.status != ApprovalStatus::Pending {
            return Err(DomainError::Conflict(format!(
                "Approval request {} is already resolved as {:?}",
                request.id, request.status
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
