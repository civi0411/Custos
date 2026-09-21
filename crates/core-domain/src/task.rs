//! Task State Machine & Lifecycle
//!
//! Implements strict, auditable lifecycle transitions for tasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Queued,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Draft => write!(f, "draft"),
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Succeeded => write!(f, "succeeded"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(&self, next: TaskStatus) -> bool {
        match self {
            TaskStatus::Draft => matches!(next, TaskStatus::Queued | TaskStatus::Cancelled),
            TaskStatus::Queued => matches!(next, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Running => matches!(
                next,
                TaskStatus::Blocked
                    | TaskStatus::Succeeded
                    | TaskStatus::Failed
                    | TaskStatus::Cancelled
            ),
            TaskStatus::Blocked => matches!(next, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Succeeded | TaskStatus::Failed | TaskStatus::Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub epoch: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl Task {
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            status: TaskStatus::Draft,
            epoch: 0,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        }
    }

    pub fn transition(&mut self, next: TaskStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        self.epoch += 1;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_task_transitions() {
        let mut t = Task::new("task_123".into(), "Test Task".into());
        assert_eq!(t.status, TaskStatus::Draft);
        assert!(t.transition(TaskStatus::Queued).is_ok());
        assert_eq!(t.epoch, 1);
        assert!(t.transition(TaskStatus::Running).is_ok());
        assert_eq!(t.epoch, 2);
        assert!(t.transition(TaskStatus::Succeeded).is_ok());
        assert_eq!(t.epoch, 3);
        assert!(t.status.is_terminal());

        // Cannot transition out of terminal state
        assert!(t.transition(TaskStatus::Running).is_err());
    }
}
