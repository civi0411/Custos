//! Explicit Task State Machine Engine
//!
//! Validates state graph transitions and provides introspection.

use custos_core_domain::{DomainError, TaskStatus};

pub struct TaskStateMachine;

impl TaskStateMachine {
    /// Returns all valid next states for a given state
    pub fn valid_next_states(current: TaskStatus) -> &'static [TaskStatus] {
        match current {
            TaskStatus::Draft => &[TaskStatus::Queued, TaskStatus::Cancelled],
            TaskStatus::Queued => &[TaskStatus::Running, TaskStatus::Cancelled],
            TaskStatus::Running => &[
                TaskStatus::Blocked,
                TaskStatus::Succeeded,
                TaskStatus::Failed,
                TaskStatus::Cancelled,
            ],
            TaskStatus::Blocked => &[TaskStatus::Running, TaskStatus::Cancelled],
            TaskStatus::Succeeded | TaskStatus::Failed | TaskStatus::Cancelled => &[],
        }
    }

    /// Verifies if a transition is legal
    pub fn can_transition(from: TaskStatus, to: TaskStatus) -> bool {
        Self::valid_next_states(from).contains(&to)
    }

    /// Ensures transition legality or raises DomainError
    pub fn ensure_can_transition(from: TaskStatus, to: TaskStatus) -> Result<(), DomainError> {
        if !Self::can_transition(from, to) {
            return Err(DomainError::InvalidStateTransition {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        Ok(())
    }
}
