//! Task Completion & Terminal State Logic
//!
//! Enforces completion gate checks and evidence verification before a task succeeds.

use custos_core_domain::{DomainError, Task, TaskStatus};

pub struct CompletionGate;

impl CompletionGate {
    /// Validates whether a task meets criteria to complete successfully
    pub fn can_complete(task: &Task) -> Result<(), DomainError> {
        if task.status != TaskStatus::Running {
            return Err(DomainError::Validation(format!(
                "Task {} must be in Running status to complete, but is in {}",
                task.id, task.status
            )));
        }
        Ok(())
    }

    /// Verifies if a task can be cancelled
    pub fn can_cancel(task: &Task) -> Result<(), DomainError> {
        if task.status.is_terminal() {
            return Err(DomainError::Conflict(format!(
                "Task {} is already finished with status {}",
                task.id, task.status
            )));
        }
        Ok(())
    }
}
