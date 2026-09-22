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
            status: RunStatus::Pending,
            attempt,
            current_span_num: 0,
            started_at: Utc::now(),
            ended_at: None,
            metadata: serde_json::json!({}),
        }
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
}
