//! Pure State Reducer
//!
//! Applies immutable events to state in a deterministic manner.

use custos_core_domain::{DomainError, Task, TaskStatus};

use crate::events::{
    TaskAdvanced, TaskBlocked, TaskCancelled, TaskCompleted, TaskCreated, TaskEvent,
};

pub struct TaskReducer;

impl TaskReducer {
    /// Creates an initial Task from TaskCreated event
    pub fn from_created(event: &TaskCreated) -> Task {
        let mut task = Task::new(event.task_id.clone(), event.title.clone());
        task.created_at = event.created_at;
        task.updated_at = event.created_at;
        task
    }

    /// Pure function applying an event to an existing Task
    pub fn apply(task: &Task, event: &TaskEvent) -> Result<Task, DomainError> {
        let mut updated = task.clone();
        match event {
            TaskEvent::Created(_) => {
                return Err(DomainError::Conflict(format!(
                    "Cannot apply TaskCreated to existing task {}",
                    task.id
                )));
            }
            TaskEvent::Advanced(TaskAdvanced {
                to_status,
                new_epoch,
                advanced_at,
                ..
            }) => {
                updated.status = *to_status;
                updated.epoch = *new_epoch;
                updated.updated_at = *advanced_at;
            }
            TaskEvent::Blocked(TaskBlocked {
                new_epoch,
                blocked_at,
                ..
            }) => {
                updated.status = TaskStatus::Blocked;
                updated.epoch = *new_epoch;
                updated.updated_at = *blocked_at;
            }
            TaskEvent::Completed(TaskCompleted {
                final_epoch,
                completed_at,
                ..
            }) => {
                updated.status = TaskStatus::Succeeded;
                updated.epoch = *final_epoch;
                updated.updated_at = *completed_at;
            }
            TaskEvent::Cancelled(TaskCancelled {
                final_epoch,
                cancelled_at,
                ..
            }) => {
                updated.status = TaskStatus::Cancelled;
                updated.epoch = *final_epoch;
                updated.updated_at = *cancelled_at;
            }
        }
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_reducer_advancement() {
        let now = Utc::now();
        let created = TaskCreated {
            task_id: "task_1".into(),
            title: "Test".into(),
            created_at: now,
        };
        let task = TaskReducer::from_created(&created);
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);

        let advanced = TaskEvent::Advanced(TaskAdvanced {
            task_id: "task_1".into(),
            from_status: TaskStatus::Draft,
            to_status: TaskStatus::Queued,
            new_epoch: 1,
            advanced_at: now,
            rationale: None,
        });

        let updated = TaskReducer::apply(&task, &advanced).expect("apply must succeed");
        assert_eq!(updated.status, TaskStatus::Queued);
        assert_eq!(updated.epoch, 1);
    }
}
