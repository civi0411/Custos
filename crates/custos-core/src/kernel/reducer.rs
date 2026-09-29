//! Pure State Reducer
//!
//! Applies immutable events to state in a deterministic manner.

use custos_domain::{DomainError, Task, TaskStatus};

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
        task.contract = event.contract.clone();
        task.metadata = event.metadata.clone();
        task
    }

    pub fn replay(events: &[TaskEvent]) -> Result<Task, DomainError> {
        let mut task = match events.first() {
            Some(TaskEvent::Created(created)) if events[0].sequence() == 0 => {
                if created.task_id.is_empty() {
                    return Err(DomainError::InvariantViolation(
                        "TaskCreated event has empty identity".into(),
                    ));
                }
                Self::from_created(created)
            }
            Some(TaskEvent::SnapshotImported(imported))
                if imported.task.id == events[0].task_id()
                    && imported.task.epoch == events[0].sequence() =>
            {
                imported.task.clone()
            }
            _ => {
                return Err(DomainError::Validation(
                    "task event stream must begin with TaskCreated or TaskSnapshotImported".into(),
                ));
            }
        };
        for (index, event) in events.iter().enumerate().skip(1) {
            let expected_sequence = task.epoch.checked_add(1).ok_or_else(|| {
                DomainError::InvariantViolation("task event sequence exhausted".into())
            })?;
            if event.task_id() != task.id || event.sequence() != expected_sequence {
                return Err(DomainError::InvariantViolation(format!(
                    "invalid task event identity or sequence at position {index}"
                )));
            }
            task = Self::apply(&task, event)?;
        }
        Ok(task)
    }

    /// Pure function applying an event to an existing Task
    pub fn apply(task: &Task, event: &TaskEvent) -> Result<Task, DomainError> {
        if event.task_id() != task.id {
            return Err(DomainError::Conflict(format!(
                "Cannot apply event for task {} to task {}",
                event.task_id(),
                task.id
            )));
        }
        let mut updated = task.clone();
        match event {
            TaskEvent::Created(_) => {
                return Err(DomainError::Conflict(format!(
                    "Cannot apply TaskCreated to existing task {}",
                    task.id
                )));
            }
            TaskEvent::SnapshotImported(_) => {
                return Err(DomainError::Conflict(format!(
                    "Cannot apply TaskSnapshotImported to existing task {}",
                    task.id
                )));
            }
            TaskEvent::Advanced(TaskAdvanced {
                from_status,
                to_status,
                new_epoch,
                advanced_at,
                ..
            }) => {
                let expected_epoch = task.epoch.checked_add(1).ok_or_else(|| {
                    DomainError::InvariantViolation("task epoch exhausted".into())
                })?;
                if *from_status != task.status
                    || *new_epoch != expected_epoch
                    || !task.status.can_transition_to(*to_status)
                {
                    return Err(DomainError::InvariantViolation(
                        "TaskAdvanced event violates the task transition".into(),
                    ));
                }
                updated.status = *to_status;
                updated.epoch = *new_epoch;
                updated.updated_at = *advanced_at;
            }
            TaskEvent::Blocked(TaskBlocked {
                from_status,
                new_epoch,
                blocked_at,
                ..
            }) => {
                let expected_epoch = task.epoch.checked_add(1).ok_or_else(|| {
                    DomainError::InvariantViolation("task epoch exhausted".into())
                })?;
                if *from_status != task.status
                    || *new_epoch != expected_epoch
                    || !task.status.can_transition_to(TaskStatus::Blocked)
                {
                    return Err(DomainError::InvariantViolation(
                        "TaskBlocked event violates the task transition".into(),
                    ));
                }
                updated.status = TaskStatus::Blocked;
                updated.epoch = *new_epoch;
                updated.updated_at = *blocked_at;
            }
            TaskEvent::Completed(TaskCompleted {
                final_epoch,
                completed_at,
                ..
            }) => {
                if Some(final_epoch) != task.epoch.checked_add(1).as_ref()
                    || !task.status.can_transition_to(TaskStatus::Succeeded)
                {
                    return Err(DomainError::InvariantViolation(
                        "TaskCompleted event violates the task transition".into(),
                    ));
                }
                updated.status = TaskStatus::Succeeded;
                updated.epoch = *final_epoch;
                updated.updated_at = *completed_at;
            }
            TaskEvent::Cancelled(TaskCancelled {
                final_epoch,
                cancelled_at,
                ..
            }) => {
                if Some(final_epoch) != task.epoch.checked_add(1).as_ref()
                    || !task.status.can_transition_to(TaskStatus::Cancelled)
                {
                    return Err(DomainError::InvariantViolation(
                        "TaskCancelled event violates the task transition".into(),
                    ));
                }
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
            contract: None,
            metadata: serde_json::json!({}),
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
