use async_trait::async_trait;
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{ContinuationPacket, DomainError, WorkerRun};

pub struct TaskRuntime;

impl TaskRuntime {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, _task_id: &str) -> Result<WorkerRun, DomainError> {
        // TODO: Implement actual run start logic
        Err(DomainError::Validation("TaskRuntime::start_run not yet implemented".into()))
    }

    async fn cancel(&self, _task_id: &str, _reason: &str) -> Result<(), DomainError> {
        // TODO: Implement cancellation logic (mark effects as Uncertain)
        Err(DomainError::Validation("TaskRuntime::cancel not yet implemented".into()))
    }

    async fn checkpoint(&self, _task_id: &str) -> Result<ContinuationPacket, DomainError> {
        // TODO: Implement checkpoint logic
        Err(DomainError::Validation("TaskRuntime::checkpoint not yet implemented".into()))
    }
}
