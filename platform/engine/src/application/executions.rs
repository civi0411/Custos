//! Agent Execution & Workflow Runtime Application Service

use std::sync::Arc;
use custos_contracts::*;
use crate::contracts::workflow::WorkflowPort;
use crate::orchestration::TaskRuntime;

/// Application service orchestrating agent runs, workflow execution and candidate races.
pub struct ExecutionApplicationService {
    task_runtime: Arc<TaskRuntime>,
}

impl ExecutionApplicationService {
    pub fn new(task_runtime: Arc<TaskRuntime>) -> Self {
        Self { task_runtime }
    }

    pub async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        self.task_runtime.start_run(cmd).await
    }

    pub async fn cancel_run(&self, task_id: &TaskId, reason: &str) -> Result<CancelReceipt, DomainError> {
        self.task_runtime.request_cancel(task_id, reason).await
    }
}
