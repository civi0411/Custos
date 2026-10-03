use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    new_id, CancelReceipt, ContinuationPacket, DomainError, Run, RunHandle, RunStatus,
    StartRunCommand, TaskStatus, WorkerRun,
};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelPort;

pub struct TaskRuntime {
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    active_runs: Arc<RwLock<HashMap<String, Run>>>,
    active_workers: Arc<RwLock<HashMap<String, WorkerRun>>>,
}

impl TaskRuntime {
    pub fn new() -> Self {
        Self {
            kernel: None,
            model: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ports(kernel: Arc<dyn KernelPort>, model: Arc<dyn ModelPort>) -> Self {
        Self {
            kernel: Some(kernel),
            model: Some(model),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        // 1. Verify and drive Task lifecycle if KernelPort is available
        if let Some(ref kernel) = self.kernel {
            let task = kernel
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

            if task.status == TaskStatus::Draft {
                kernel.transition_task(&task.id, TaskStatus::Queued).await?;
                kernel.transition_task(&task.id, TaskStatus::Running).await?;
            } else if task.status == TaskStatus::Queued {
                kernel.transition_task(&task.id, TaskStatus::Running).await?;
            } else if task.status.is_terminal() {
                return Err(DomainError::InvalidStateTransition {
                    from: task.status.to_string(),
                    to: "running".into(),
                });
            }
        }

        // 2. Instantiate and activate Run
        let mut run = Run::new(cmd.task_id.clone(), 1);
        run.workflow_revision = cmd.workflow_revision;
        run.transition(RunStatus::Active)?;

        // 3. Instantiate bounded WorkerRun
        let mut wrun = WorkerRun::new(cmd.task_id.clone(), cmd.actor.clone(), 3);
        wrun.run_id = Some(run.id.clone());
        wrun.transition(RunStatus::Active)?;

        // 4. Connect to ModelPort if available
        if let Some(ref model) = self.model {
            let req = ProviderRequest::new(
                new_id("req"),
                &cmd.task_id,
                1,
                format!("Initial run turn for task {}", cmd.task_id),
                model.provider_id(),
            );
            let _ = model.generate(&req).await?;
        }

        let handle = RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
        };

        // 5. Register active executions
        self.active_runs.write().await.insert(run.id.clone(), run);
        self.active_workers.write().await.insert(wrun.id.clone(), wrun);

        Ok(handle)
    }

    async fn request_cancel(&self, run_id: &str, reason: &str) -> Result<CancelReceipt, DomainError> {
        let mut runs = self.active_runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "Run".into(),
                id: run_id.to_string(),
            })?;

        run.transition(RunStatus::Cancelled)?;

        if let Some(ref kernel) = self.kernel {
            let _ = kernel.transition_task(&run.task_id, TaskStatus::Cancelled).await;
        }

        Ok(CancelReceipt {
            run_id: run_id.to_string(),
            cancelled_at: Utc::now(),
            reason: reason.to_string(),
            uncertain_effects_count: 0,
        })
    }

    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError> {
        let mut runs = self.active_runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "Run".into(),
                id: run_id.to_string(),
            })?;

        if run.status == RunStatus::Suspended {
            run.transition(RunStatus::Active)?;
        }

        Ok(RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
        })
    }

    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError> {
        let runs = self.active_runs.read().await;
        let run = runs
            .get(run_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "Run".into(),
                id: run_id.to_string(),
            })?;

        ContinuationPacket::create(
            run.task_id.clone(),
            run.current_span_num,
            run.current_span_num + 1,
            "custos-runtime".into(),
            "default".into(),
            format!("Checkpoint for run {run_id}"),
            serde_json::json!({
                "run_id": run.id,
                "status": run.status.to_string(),
                "attempt": run.attempt,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_runtime_lifecycle() {
        let runtime = TaskRuntime::new();
        let cmd = StartRunCommand::new("task_test_123", "test_actor");
        let handle = runtime.start_run(cmd).await.unwrap();
        assert_eq!(handle.task_id, "task_test_123");
        assert_eq!(handle.status, RunStatus::Active);

        // Checkpoint
        let cp = runtime.checkpoint(&handle.run_id).await.unwrap();
        assert_eq!(cp.task_id, "task_test_123");

        // Cancel
        let receipt = runtime.request_cancel(&handle.run_id, "user request").await.unwrap();
        assert_eq!(receipt.run_id, handle.run_id);
        assert_eq!(receipt.reason, "user request");
    }
}
