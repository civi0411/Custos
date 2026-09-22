//! Task Invariant Validations
//!
//! Enforces business logic rules, optimistic concurrency checks, and state transitions.

use custos_core_domain::{DomainError, Task, TaskStatus};

pub struct TaskInvariants;

impl TaskInvariants {
    /// Asserts title is valid and non-empty
    pub fn assert_valid_title(title: &str) -> Result<(), DomainError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("Task title cannot be empty".into()));
        }
        if trimmed.len() > 256 {
            return Err(DomainError::Validation(
                "Task title cannot exceed 256 characters".into(),
            ));
        }
        Ok(())
    }

    /// Asserts optimistic concurrency matches expected epoch
    pub fn assert_epoch(task: &Task, expected_epoch: u64) -> Result<(), DomainError> {
        if task.epoch != expected_epoch {
            return Err(DomainError::Conflict(format!(
                "Epoch conflict for task {}: expected {}, current {}",
                task.id, expected_epoch, task.epoch
            )));
        }
        Ok(())
    }

    /// Asserts task is not in a terminal state
    pub fn assert_not_terminal(task: &Task) -> Result<(), DomainError> {
        if task.status.is_terminal() {
            return Err(DomainError::Conflict(format!(
                "Task {} is already in terminal state {}",
                task.id, task.status
            )));
        }
        Ok(())
    }

    /// Asserts that a transition from current status to next status is permitted
    pub fn assert_valid_transition(from: TaskStatus, to: TaskStatus) -> Result<(), DomainError> {
        if !from.can_transition_to(to) {
            return Err(DomainError::InvalidStateTransition {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        Ok(())
    }
}
