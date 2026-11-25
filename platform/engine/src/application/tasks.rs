//! Task Application Service & Lifecycle Use Cases

use crate::task::{AdvanceTask, CancelTask, CompleteTask, CreateTask, TaskService};
use crate::task::{TaskAdvanced, TaskCancelled, TaskCompleted, TaskCreated};
use custos_contracts::{DomainError, Task};
use std::sync::Arc;

/// Primary application service orchestrating task lifecycle and transitions.
pub struct TaskApplicationService {
    task_service: Arc<TaskService>,
}

impl TaskApplicationService {
    pub fn new(task_service: Arc<TaskService>) -> Self {
        Self { task_service }
    }

    pub async fn create_task(&self, cmd: CreateTask) -> Result<(Task, TaskCreated), DomainError> {
        self.task_service.execute_create(cmd).await
    }

    pub async fn advance_task(
        &self,
        cmd: AdvanceTask,
    ) -> Result<(Task, TaskAdvanced), DomainError> {
        self.task_service.execute_advance(cmd).await
    }

    pub async fn complete_task(
        &self,
        cmd: CompleteTask,
    ) -> Result<(Task, TaskCompleted), DomainError> {
        self.task_service.execute_complete(cmd).await
    }

    pub async fn cancel_task(&self, cmd: CancelTask) -> Result<(Task, TaskCancelled), DomainError> {
        self.task_service.execute_cancel(cmd).await
    }
}
