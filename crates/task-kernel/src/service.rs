//! Task Orchestration Service
//!
//! Handles creating, advancing, and completing tasks with strict state validation,
//! invariants enforcement, event generation, and persistence.

use chrono::Utc;
use std::sync::Arc;

use custos_core_domain::{new_id, DomainError, Task, TaskStatus};

use crate::commands::{AdvanceTask, BlockTask, CancelTask, CompleteTask, CreateTask};
use crate::completion::CompletionGate;
use crate::events::{
    TaskAdvanced, TaskBlocked, TaskCancelled, TaskCompleted, TaskCreated, TaskEvent,
};
use crate::invariants::TaskInvariants;
use crate::ports::TaskStore;
use crate::reducer::TaskReducer;
use crate::state_machine::TaskStateMachine;

pub struct TaskService {
    store: Arc<dyn TaskStore>,
}

impl TaskService {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self { store }
    }

    /// CQRS command: CreateTask
    pub async fn execute_create(
        &self,
        cmd: CreateTask,
    ) -> Result<(Task, TaskCreated), DomainError> {
        TaskInvariants::assert_valid_title(&cmd.title)?;

        let task_id = new_id("task");
        let now = Utc::now();
        let event = TaskCreated {
            task_id: task_id.clone(),
            title: cmd.title,
            created_at: now,
        };

        let mut task = TaskReducer::from_created(&event);
        if let Some(meta) = cmd.metadata {
            task.metadata = meta;
        }

        self.store.save_task(&task).await?;
        Ok((task, event))
    }

    /// CQRS command: AdvanceTask
    pub async fn execute_advance(
        &self,
        cmd: AdvanceTask,
    ) -> Result<(Task, TaskAdvanced), DomainError> {
        let task =
            self.store
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

        TaskInvariants::assert_not_terminal(&task)?;
        TaskInvariants::assert_epoch(&task, cmd.expected_epoch)?;
        TaskStateMachine::ensure_can_transition(task.status, cmd.next_status)?;

        let event = TaskAdvanced {
            task_id: task.id.clone(),
            from_status: task.status,
            to_status: cmd.next_status,
            new_epoch: task.epoch + 1,
            advanced_at: Utc::now(),
            rationale: cmd.rationale,
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Advanced(event.clone()))?;
        self.store.save_task(&updated).await?;
        Ok((updated, event))
    }

    /// CQRS command: BlockTask
    pub async fn execute_block(&self, cmd: BlockTask) -> Result<(Task, TaskBlocked), DomainError> {
        let task =
            self.store
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

        TaskInvariants::assert_not_terminal(&task)?;
        TaskInvariants::assert_epoch(&task, cmd.expected_epoch)?;
        TaskStateMachine::ensure_can_transition(task.status, TaskStatus::Blocked)?;

        let event = TaskBlocked {
            task_id: task.id.clone(),
            from_status: task.status,
            new_epoch: task.epoch + 1,
            reason: cmd.reason,
            blocked_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Blocked(event.clone()))?;
        self.store.save_task(&updated).await?;
        Ok((updated, event))
    }

    /// CQRS command: CompleteTask
    pub async fn execute_complete(
        &self,
        cmd: CompleteTask,
    ) -> Result<(Task, TaskCompleted), DomainError> {
        let task =
            self.store
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

        TaskInvariants::assert_epoch(&task, cmd.expected_epoch)?;
        CompletionGate::can_complete(&task)?;

        let event = TaskCompleted {
            task_id: task.id.clone(),
            final_epoch: task.epoch + 1,
            summary: cmd.summary,
            completed_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Completed(event.clone()))?;
        self.store.save_task(&updated).await?;
        Ok((updated, event))
    }

    /// CQRS command: CancelTask
    pub async fn execute_cancel(
        &self,
        cmd: CancelTask,
    ) -> Result<(Task, TaskCancelled), DomainError> {
        let task =
            self.store
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

        TaskInvariants::assert_epoch(&task, cmd.expected_epoch)?;
        CompletionGate::can_cancel(&task)?;

        let event = TaskCancelled {
            task_id: task.id.clone(),
            final_epoch: task.epoch + 1,
            reason: cmd.reason,
            cancelled_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Cancelled(event.clone()))?;
        self.store.save_task(&updated).await?;
        Ok((updated, event))
    }

    /// Simple backward-compatible creation
    pub async fn create_task(&self, title: String) -> Result<Task, DomainError> {
        let (task, _) = self
            .execute_create(CreateTask {
                title,
                metadata: None,
            })
            .await?;
        Ok(task)
    }

    /// Simple backward-compatible transition
    pub async fn transition_task(
        &self,
        task_id: &str,
        next: TaskStatus,
    ) -> Result<Task, DomainError> {
        let task = self
            .store
            .get_task(task_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                kind: "Task".into(),
                id: task_id.to_string(),
            })?;

        let (updated, _) = self
            .execute_advance(AdvanceTask {
                task_id: task.id,
                expected_epoch: task.epoch,
                next_status: next,
                rationale: None,
            })
            .await?;
        Ok(updated)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        self.store.get_task(task_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_core_domain::{ContinuationPacket, Span};
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Default)]
    struct MockTaskStore {
        tasks: RwLock<HashMap<String, Task>>,
    }

    #[async_trait]
    impl TaskStore for MockTaskStore {
        async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
            let guard = self.tasks.read().unwrap();
            Ok(guard.get(task_id).cloned())
        }
        async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
            let mut guard = self.tasks.write().unwrap();
            guard.insert(task.id.clone(), task.clone());
            Ok(())
        }
        async fn get_span(&self, _span_id: &str) -> Result<Option<Span>, DomainError> {
            Ok(None)
        }
        async fn save_span(&self, _span: &Span) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_spans(&self, _task_id: &str) -> Result<Vec<Span>, DomainError> {
            Ok(vec![])
        }
        async fn save_continuation(&self, _packet: &ContinuationPacket) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_latest_continuation(
            &self,
            _task_id: &str,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_full_cqrs_lifecycle() {
        let store = Arc::new(MockTaskStore::default());
        let service = TaskService::new(store);

        // 1. Create
        let (task, created) = service
            .execute_create(CreateTask {
                title: "Refactor architecture".into(),
                metadata: None,
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);
        assert_eq!(created.title, "Refactor architecture");

        // 2. Advance to Queued
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                expected_epoch: 0,
                next_status: TaskStatus::Queued,
                rationale: Some("Enqueued for runner".into()),
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.epoch, 1);

        // 3. Advance to Running
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                expected_epoch: 1,
                next_status: TaskStatus::Running,
                rationale: None,
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.epoch, 2);

        // 4. Complete
        let (task, completed) = service
            .execute_complete(CompleteTask {
                task_id: task.id.clone(),
                expected_epoch: 2,
                summary: "Done cleanly".into(),
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Succeeded);
        assert_eq!(task.epoch, 3);
        assert_eq!(completed.summary, "Done cleanly");

        // 5. Verify cannot advance after terminal
        let err = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                expected_epoch: 3,
                next_status: TaskStatus::Running,
                rationale: None,
            })
            .await;
        assert!(err.is_err());
    }
}
