//! Task Orchestration Service
//!
//! Handles creating, advancing, and completing tasks with strict state validation,
//! invariants enforcement, event generation, and persistence.

use chrono::Utc;
use std::sync::Arc;

use custos_domain::{new_id, DomainError, Span, Task, TaskStatus};

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
            contract: cmd.contract,
            metadata: cmd.metadata.unwrap_or_else(|| serde_json::json!({})),
            created_at: now,
        };

        let task = TaskReducer::from_created(&event);

        self.store
            .commit_task_event(&TaskEvent::Created(event.clone()), &task)
            .await?;
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
        if cmd.next_status == TaskStatus::Succeeded {
            return Err(DomainError::Validation(
                "Cannot advance directly to Succeeded via execute_advance; must complete through execute_complete with proof closure".into(),
            ));
        }
        TaskStateMachine::ensure_can_transition(task.status, cmd.next_status)?;
        let new_epoch = task
            .epoch
            .checked_add(1)
            .ok_or_else(|| DomainError::InvariantViolation("task epoch exhausted".into()))?;

        let event = TaskAdvanced {
            task_id: task.id.clone(),
            from_status: task.status,
            to_status: cmd.next_status,
            new_epoch,
            advanced_at: Utc::now(),
            rationale: cmd.rationale,
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Advanced(event.clone()))?;
        self.store
            .commit_task_event(&TaskEvent::Advanced(event.clone()), &updated)
            .await?;
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
        let new_epoch = task
            .epoch
            .checked_add(1)
            .ok_or_else(|| DomainError::InvariantViolation("task epoch exhausted".into()))?;

        let event = TaskBlocked {
            task_id: task.id.clone(),
            from_status: task.status,
            new_epoch,
            reason: cmd.reason,
            blocked_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Blocked(event.clone()))?;
        self.store
            .commit_task_event(&TaskEvent::Blocked(event.clone()), &updated)
            .await?;
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
        CompletionGate::can_complete_with_evidence(&task, &cmd.evidence_claims)?;
        let final_epoch = task
            .epoch
            .checked_add(1)
            .ok_or_else(|| DomainError::InvariantViolation("task epoch exhausted".into()))?;

        let event = TaskCompleted {
            task_id: task.id.clone(),
            final_epoch,
            summary: cmd.summary,
            completed_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Completed(event.clone()))?;
        self.store
            .commit_task_event(&TaskEvent::Completed(event.clone()), &updated)
            .await?;
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
        let final_epoch = task
            .epoch
            .checked_add(1)
            .ok_or_else(|| DomainError::InvariantViolation("task epoch exhausted".into()))?;

        let event = TaskCancelled {
            task_id: task.id.clone(),
            final_epoch,
            reason: cmd.reason,
            cancelled_at: Utc::now(),
        };

        let updated = TaskReducer::apply(&task, &TaskEvent::Cancelled(event.clone()))?;
        self.store
            .commit_task_event(&TaskEvent::Cancelled(event.clone()), &updated)
            .await?;
        Ok((updated, event))
    }

    /// Simple backward-compatible creation
    pub async fn create_task(&self, title: String) -> Result<Task, DomainError> {
        let (task, _) = self
            .execute_create(CreateTask {
                title,
                metadata: None,
                contract: None,
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

    pub async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        self.store.list_tasks().await
    }

    pub async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError> {
        self.store.list_spans(task_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_domain::{ContinuationPacket, Span};
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
        async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
            let guard = self.tasks.read().unwrap();
            Ok(guard.values().cloned().collect())
        }
        async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
            let mut guard = self.tasks.write().unwrap();
            guard.insert(task.id.clone(), task.clone());
            Ok(())
        }
        async fn commit_task_event(
            &self,
            _event: &TaskEvent,
            task: &Task,
        ) -> Result<(), DomainError> {
            self.save_task(task).await
        }
        async fn get_task_events(&self, _task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
            Ok(Vec::new())
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
        async fn get_continuation(
            &self,
            _task_id: &str,
            _to_span: u32,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            Ok(None)
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
                contract: None,
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
                evidence_claims: Vec::new(),
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
