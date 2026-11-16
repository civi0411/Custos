use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Pending,
    Dispatched,
    Released,
    Expired,
}

impl std::fmt::Display for ClaimStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimStatus::Pending => write!(f, "pending"),
            ClaimStatus::Dispatched => write!(f, "dispatched"),
            ClaimStatus::Released => write!(f, "released"),
            ClaimStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Sovereign dispatch claim for atomic task/node dispatch (Orca fencing parity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchClaim {
    pub id: String,
    pub task_id: String,
    pub node_id: Option<String>,
    pub assignee: String,
    pub depth: u32,
    pub status: ClaimStatus,
    pub claimed_at: DateTime<Utc>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub worker_run_id: Option<String>,
}

impl DispatchClaim {
    pub fn new(task_id: impl Into<String>, assignee: impl Into<String>, depth: u32) -> Self {
        Self {
            id: new_id("claim"),
            task_id: task_id.into(),
            node_id: None,
            assignee: assignee.into(),
            depth,
            status: ClaimStatus::Pending,
            claimed_at: Utc::now(),
            dispatched_at: None,
            released_at: None,
            worker_run_id: None,
        }
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    pub fn mark_dispatched(&mut self, worker_run_id: impl Into<String>) -> Result<(), DomainError> {
        if self.status != ClaimStatus::Pending {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: "dispatched".into(),
            });
        }
        self.status = ClaimStatus::Dispatched;
        self.worker_run_id = Some(worker_run_id.into());
        self.dispatched_at = Some(Utc::now());
        Ok(())
    }

    pub fn release(&mut self) -> Result<(), DomainError> {
        if self.status == ClaimStatus::Released {
            return Ok(());
        }
        self.status = ClaimStatus::Released;
        self.released_at = Some(Utc::now());
        Ok(())
    }
}
