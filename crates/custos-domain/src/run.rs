//! Task Run Entity & Execution Sessions
//!
//! A Run represents an active execution attempt or session for a Task.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Active,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Pending => write!(f, "pending"),
            RunStatus::Active => write!(f, "active"),
            RunStatus::Suspended => write!(f, "suspended"),
            RunStatus::Completed => write!(f, "completed"),
            RunStatus::Failed => write!(f, "failed"),
            RunStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(&self, next: RunStatus) -> bool {
        match self {
            RunStatus::Pending => matches!(next, RunStatus::Active | RunStatus::Cancelled),
            RunStatus::Active => matches!(
                next,
                RunStatus::Suspended
                    | RunStatus::Completed
                    | RunStatus::Failed
                    | RunStatus::Cancelled
            ),
            RunStatus::Suspended => matches!(next, RunStatus::Active | RunStatus::Cancelled),
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub task_id: String,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    pub status: RunStatus,
    pub attempt: u32,
    pub current_span_num: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl Run {
    pub fn new(task_id: String, attempt: u32) -> Self {
        Self {
            id: new_id("run"),
            task_id,
            workflow_revision: None,
            status: RunStatus::Pending,
            attempt,
            current_span_num: 0,
            started_at: Utc::now(),
            ended_at: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_workflow_revision(mut self, revision: impl Into<String>) -> Self {
        self.workflow_revision = Some(revision.into());
        self
    }

    pub fn transition(&mut self, next: RunStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        if next.is_terminal() {
            self.ended_at = Some(Utc::now());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRun {
    pub id: String,
    #[serde(default)]
    pub run_id: Option<String>,
    pub task_id: String,
    pub worker_id: String,
    pub attempt_id: u32,
    pub max_attempts: u32,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub continuation_packet_ref: Option<String>,
}

impl WorkerRun {
    pub fn new(task_id: String, worker_id: String, max_attempts: u32) -> Self {
        Self {
            id: new_id("wrun"),
            run_id: None,
            task_id,
            worker_id,
            attempt_id: 1,
            max_attempts,
            status: RunStatus::Pending,
            started_at: Utc::now(),
            ended_at: None,
            continuation_packet_ref: None,
        }
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn can_retry(&self) -> bool {
        self.attempt_id < self.max_attempts
    }

    pub fn record_attempt(&mut self) -> Result<u32, DomainError> {
        if !self.can_retry() {
            return Err(DomainError::Validation(format!(
                "Max attempts ({}) reached for worker run {}",
                self.max_attempts, self.id
            )));
        }
        self.attempt_id += 1;
        self.status = RunStatus::Active;
        Ok(self.attempt_id)
    }

    pub fn transition(&mut self, next: RunStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        if next.is_terminal() {
            self.ended_at = Some(Utc::now());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttempt {
    pub id: String,
    pub worker_run_id: String,
    pub attempt_number: u32,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub exit_reason: Option<String>,
}

impl NodeAttempt {
    pub fn new(worker_run_id: String, attempt_number: u32) -> Self {
        Self {
            id: new_id("node_att"),
            worker_run_id,
            attempt_number,
            status: RunStatus::Active,
            started_at: Utc::now(),
            ended_at: None,
            exit_reason: None,
        }
    }

    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.ended_at = Some(Utc::now());
    }

    pub fn fail(&mut self, reason: String) {
        self.status = RunStatus::Failed;
        self.ended_at = Some(Utc::now());
        self.exit_reason = Some(reason);
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunCommand {
    pub task_id: String,
    pub actor: String,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    #[serde(default)]
    pub preferred_mode: Option<String>,
    #[serde(default)]
    pub harness_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
}

impl StartRunCommand {
    pub fn new(task_id: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            actor: actor.into(),
            workflow_revision: None,
            preferred_mode: None,
            harness_id: None,
            workspace_root: None,
        }
    }

    pub fn with_workflow_revision(mut self, revision: impl Into<String>) -> Self {
        self.workflow_revision = Some(revision.into());
        self
    }

    pub fn with_preferred_mode(mut self, mode: impl Into<String>) -> Self {
        self.preferred_mode = Some(mode.into());
        self
    }

    pub fn with_harness_id(mut self, harness_id: impl Into<String>) -> Self {
        self.harness_id = Some(harness_id.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHandle {
    pub run_id: String,
    pub task_id: String,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelReceipt {
    pub run_id: String,
    pub cancelled_at: DateTime<Utc>,
    pub reason: String,
    pub uncertain_effects_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_lifecycle() {
        let mut run = Run::new("task_001".into(), 1);
        assert_eq!(run.status, RunStatus::Pending);
        assert!(run.transition(RunStatus::Active).is_ok());
        assert!(run.transition(RunStatus::Suspended).is_ok());
        assert!(run.transition(RunStatus::Active).is_ok());
        assert!(run.transition(RunStatus::Completed).is_ok());
        assert!(run.status.is_terminal());
        assert!(run.ended_at.is_some());
        assert!(run.transition(RunStatus::Active).is_err());
    }

    #[test]
    fn test_worker_run_bounded_retry() {
        let mut wrun = WorkerRun::new("task_002".into(), "worker_alpha".into(), 3);
        assert_eq!(wrun.attempt_id, 1);
        assert!(wrun.can_retry());

        assert_eq!(wrun.record_attempt().unwrap(), 2);
        assert!(wrun.can_retry());

        assert_eq!(wrun.record_attempt().unwrap(), 3);
        assert!(!wrun.can_retry());

        // Exceeded bounded retry
        assert!(wrun.record_attempt().is_err());
    }

    #[test]
    fn test_dispatch_claim_lifecycle() {
        let mut claim = DispatchClaim::new("task_claim_1", "worker_1", 1);
        assert_eq!(claim.status, ClaimStatus::Pending);
        assert_eq!(claim.depth, 1);
        assert!(claim.dispatched_at.is_none());

        assert!(claim.mark_dispatched("wrun_1").is_ok());
        assert_eq!(claim.status, ClaimStatus::Dispatched);
        assert_eq!(claim.worker_run_id, Some("wrun_1".to_string()));
        assert!(claim.dispatched_at.is_some());

        // Cannot transition from dispatched to dispatched again
        assert!(claim.mark_dispatched("wrun_2").is_err());

        // Release
        assert!(claim.release().is_ok());
        assert_eq!(claim.status, ClaimStatus::Released);
        assert!(claim.released_at.is_some());
    }
}
