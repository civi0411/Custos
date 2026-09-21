//! Task Orchestration Service
//!
//! Handles creating, advancing, and completing tasks with strict state validation.

use crate::ports::TaskStore;
use custos_core_domain::{new_id, DomainError, Task, TaskStatus};
use std::sync::Arc;

pub struct TaskService {
    store: Arc<dyn TaskStore>,
}

impl TaskService {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self { store }
    }

    pub async fn create_task(&self, title: String) -> Result<Task, DomainError> {
        let task_id = new_id("task");
        let task = Task::new(task_id, title);
        self.store.save_task(&task).await?;
        Ok(task)
    }

    pub async fn transition_task(
        &self,
        task_id: &str,
        next: TaskStatus,
    ) -> Result<Task, DomainError> {
        let mut task = self
            .store
            .get_task(task_id)
            .await?
            .ok_or_else(|| DomainError::Validation(format!("Task {task_id} not found")))?;

        task.transition(next)?;
        self.store.save_task(&task).await?;
        Ok(task)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        self.store.get_task(task_id).await
    }
}
