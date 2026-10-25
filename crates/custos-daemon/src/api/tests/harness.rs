use async_trait::async_trait;
use custos_core::{TaskEvent, TaskStore};
use custos_domain::{ContinuationPacket, DomainError, Span, Task};
use std::sync::Mutex;

pub struct MockStore {
    pub tasks: Mutex<Vec<Task>>,
}

impl MockStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl TaskStore for MockStore {
    async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        let list = self.tasks.lock().unwrap();
        Ok(list.clone())
    }
    async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        let mut list = self.tasks.lock().unwrap();
        if let Some(pos) = list.iter().position(|t| t.id == task.id) {
            list[pos] = task.clone();
        } else {
            list.push(task.clone());
        }
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

    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        let list = self.tasks.lock().unwrap();
        Ok(list.iter().find(|t| t.id == task_id).cloned())
    }

    async fn get_span(&self, _span_id: &str) -> Result<Option<Span>, DomainError> {
        Ok(None)
    }

    async fn save_span(&self, _span: &Span) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_spans(&self, _task_id: &str) -> Result<Vec<Span>, DomainError> {
        Ok(Vec::new())
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
