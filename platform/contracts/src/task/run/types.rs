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
    Uncertain,
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
            RunStatus::Uncertain => write!(f, "uncertain"),
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
                    | RunStatus::Uncertain
            ),
            RunStatus::Suspended => matches!(next, RunStatus::Active | RunStatus::Cancelled),
            RunStatus::Uncertain => matches!(
                next,
                RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
            ),
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    #[serde(default)]
    pub state_version: u64,
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
            state_version: 0,
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
    pub state_version: u64,
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
            state_version: 0,
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
