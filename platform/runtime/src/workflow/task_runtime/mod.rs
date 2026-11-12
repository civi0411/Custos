use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::{CommandRunnerPort, SandboxPort};
use custos_core::contracts::storage::{EffectLedgerPort, OutboxPort, RunPort};
use custos_core::contracts::supervision::{NoopSupervisionSink, SupervisionTraceSink};
pub use custos_core::contracts::workflow::WorkflowPort;
use custos_core::SessionStore;
use custos_domain::{
    DomainError, Run, RunStatus, TaskStatus, WorkerRun,
};
use custos_provider::ModelPort;

#[derive(Clone)]
pub struct TaskRuntime {
    live: super::live::LiveRuns,
    journal_manager: Option<Arc<crate::session::SessionManager>>,
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    harness: Option<Arc<dyn AgentRuntimePort>>,
    harnesses: HashMap<String, Arc<dyn AgentRuntimePort>>,
    sandbox: Option<Arc<dyn SandboxPort>>,
    command_runner: Option<Arc<dyn CommandRunnerPort>>,
    outbox: Option<Arc<dyn OutboxPort>>,
    effect_ledger: Option<Arc<dyn EffectLedgerPort>>,
    run_store: Option<Arc<dyn RunPort>>,
    session_store: Option<Arc<dyn SessionStore>>,
    dispatcher: Arc<super::dispatcher::WorkflowDispatcher>,
    lease_manager: Option<Arc<crate::workspace::lease::WorkspaceLeaseManager>>,
    workspace_coordinator: Option<Arc<crate::workspace::WorkspaceCoordinator>>,
    admission_coordinator: Option<Arc<crate::compute::WorkloadAdmissionCoordinator>>,
    supervision: Arc<dyn SupervisionTraceSink>,
    active_runs: Arc<RwLock<HashMap<String, Run>>>,
    active_workers: Arc<RwLock<HashMap<String, WorkerRun>>>,
}

impl TaskRuntime {
    pub fn new() -> Self {
        Self {
            live: super::live::LiveRuns::default(),
            journal_manager: None,
            kernel: None,
            model: None,
            harness: None,
            harnesses: HashMap::new(),
            sandbox: None,
            command_runner: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            session_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            workspace_coordinator: None,
            admission_coordinator: None,
            supervision: Arc::new(NoopSupervisionSink),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ports(kernel: Arc<dyn KernelPort>, model: Arc<dyn ModelPort>) -> Self {
        Self {
            live: super::live::LiveRuns::default(),
            journal_manager: None,
            kernel: Some(kernel),
            model: Some(model),
            harness: None,
            harnesses: HashMap::new(),
            sandbox: None,
            command_runner: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            session_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            workspace_coordinator: None,
            admission_coordinator: None,
            supervision: Arc::new(NoopSupervisionSink),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_journal_manager(mut self, manager: Arc<crate::session::SessionManager>) -> Self {
        self.journal_manager = Some(manager);
        self
    }

    pub fn with_kernel(mut self, kernel: Arc<dyn KernelPort>) -> Self {
        self.kernel = Some(kernel);
        self
    }

    pub fn with_model(mut self, model: Arc<dyn ModelPort>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_harness(mut self, harness: Arc<dyn AgentRuntimePort>) -> Self {
        let harness_id = harness.harness_id().to_string();
        self.harnesses.insert(harness_id, harness.clone());
        self.harness = Some(harness);
        self
    }

    pub fn with_harnesses(
        mut self,
        harnesses: HashMap<String, Arc<dyn AgentRuntimePort>>,
        default_harness_id: Option<&str>,
    ) -> Self {
        self.harness = default_harness_id
            .and_then(|id| harnesses.get(id).cloned())
            .or_else(|| harnesses.values().next().cloned());
        self.harnesses = harnesses;
        self
    }

    fn resolve_harness(
        &self,
        requested_harness_id: Option<&str>,
    ) -> Result<Arc<dyn AgentRuntimePort>, DomainError> {
        if let Some(id) = requested_harness_id.filter(|id| !id.trim().is_empty()) {
            return self.harnesses.get(id).cloned().ok_or_else(|| {
                DomainError::Validation(format!(
                    "Requested harness '{id}' is not composed into this runtime"
                ))
            });
        }

        self.harness.clone().ok_or_else(|| {
            DomainError::Validation("No native harness is composed into this runtime".into())
        })
    }

    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxPort>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    pub fn with_command_runner(mut self, command_runner: Arc<dyn CommandRunnerPort>) -> Self {
        self.command_runner = Some(command_runner);
        self
    }

    pub fn with_outbox(mut self, outbox: Arc<dyn OutboxPort>) -> Self {
        self.outbox = Some(outbox);
        self
    }

    pub fn with_effect_ledger(mut self, effect_ledger: Arc<dyn EffectLedgerPort>) -> Self {
        self.effect_ledger = Some(effect_ledger);
        self
    }

    pub fn with_run_store(mut self, run_store: Arc<dyn RunPort>) -> Self {
        self.dispatcher = Arc::new(
            super::dispatcher::WorkflowDispatcher::new().with_run_store(run_store.clone()),
        );
        self.run_store = Some(run_store);
        self
    }

    pub fn with_session_store(mut self, session_store: Arc<dyn SessionStore>) -> Self {
        self.session_store = Some(session_store);
        self
    }

    pub fn with_dispatcher(
        mut self,
        dispatcher: Arc<super::dispatcher::WorkflowDispatcher>,
    ) -> Self {
        self.dispatcher = dispatcher;
        self
    }

    pub fn with_lease_manager(
        mut self,
        lease_manager: Arc<crate::workspace::lease::WorkspaceLeaseManager>,
    ) -> Self {
        self.lease_manager = Some(lease_manager);
        self
    }

    pub fn lease_manager(&self) -> Option<&Arc<crate::workspace::lease::WorkspaceLeaseManager>> {
        self.lease_manager.as_ref()
    }

    pub fn with_workspace_coordinator(
        mut self,
        workspace_coordinator: Arc<crate::workspace::WorkspaceCoordinator>,
    ) -> Self {
        self.workspace_coordinator = Some(workspace_coordinator);
        self
    }

    pub fn workspace_coordinator(&self) -> Option<&Arc<crate::workspace::WorkspaceCoordinator>> {
        self.workspace_coordinator.as_ref()
    }

    pub fn with_admission_coordinator(
        mut self,
        admission_coordinator: Arc<crate::compute::WorkloadAdmissionCoordinator>,
    ) -> Self {
        self.admission_coordinator = Some(admission_coordinator);
        self
    }

    pub fn with_supervision(mut self, sink: Arc<dyn SupervisionTraceSink>) -> Self {
        self.supervision = sink;
        self
    }

    pub fn with_live_runs(mut self, live: super::live::LiveRuns) -> Self {
        self.live = live;
        self
    }

    pub fn live_runs(&self) -> super::live::LiveRuns {
        self.live.clone()
    }

    pub fn admission_coordinator(
        &self,
    ) -> Option<&Arc<crate::compute::WorkloadAdmissionCoordinator>> {
        self.admission_coordinator.as_ref()
    }

    /// Validates that all mandatory composition dependencies are present before accepting production work.
    /// Distinguishes intentionally disabled capabilities from missing required wiring (A03).
    pub fn validate_ready(&self) -> Result<(), DomainError> {
        if self.kernel.is_none() {
            return Err(DomainError::Validation(
                "composition_incomplete: kernel port is required".into(),
            ));
        }
        if self.run_store.is_none() {
            return Err(DomainError::Validation(
                "composition_incomplete: run_store port is required".into(),
            ));
        }
        if self.session_store.is_none() {
            return Err(DomainError::Validation(
                "composition_incomplete: session_store port is required".into(),
            ));
        }
        if self.model.is_none() {
            return Err(DomainError::Validation(
                "composition_incomplete: model port is required".into(),
            ));
        }
        if self.workspace_coordinator.is_none() {
            return Err(DomainError::Validation(
                "composition_incomplete: workspace_coordinator is required".into(),
            ));
        }
        Ok(())
    }

    pub fn kernel(&self) -> Option<&Arc<dyn KernelPort>> {
        self.kernel.as_ref()
    }

    pub fn harness(&self) -> Option<&Arc<dyn AgentRuntimePort>> {
        self.harness.as_ref()
    }

    pub fn sandbox(&self) -> Option<&Arc<dyn SandboxPort>> {
        self.sandbox.as_ref()
    }

    pub fn outbox(&self) -> Option<&Arc<dyn OutboxPort>> {
        self.outbox.as_ref()
    }

    pub fn effect_ledger(&self) -> Option<&Arc<dyn EffectLedgerPort>> {
        self.effect_ledger.as_ref()
    }

    pub fn run_store(&self) -> Option<&Arc<dyn RunPort>> {
        self.run_store.as_ref()
    }

    pub fn dispatcher(&self) -> &Arc<super::dispatcher::WorkflowDispatcher> {
        &self.dispatcher
    }

    /// C2: Ingress funnel for native harness events into the sovereign observer stream
    pub fn observe_harness_event(&self, run_id: &str, kind: &str, payload: serde_json::Value) {
        self.live.emit(run_id, kind, payload);
    }

    /// Checks if there is currently an active, running worker for a given task
    pub async fn has_active_worker_for_task(&self, task_id: &str) -> bool {
        let workers = self.active_workers.read().await;
        workers.values().any(|w| w.task_id == task_id)
    }

    /// Checks if there is currently an active worker for a specific run ID
    pub async fn has_active_worker_for_run(&self, run_id: &str) -> bool {
        let workers = self.active_workers.read().await;
        workers
            .values()
            .any(|w| w.run_id.as_deref() == Some(run_id) || w.id == run_id)
    }

    /// Delivers mid-flight steering guidance to the active worker for a task
    pub async fn steer_task(&self, task_id: &str, message: &str) -> Result<bool, DomainError> {
        let active_run = {
            let runs = self.active_runs.read().await;
            runs.values().find(|r| r.task_id == task_id).cloned()
        };

        if let Some(run) = active_run {
            if let Some(ref harness) = self.harness {
                if harness.profile().supports_steer {
                    harness.steer_run(&run.id, message).await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn recover_completed_model_chat_turn(&self, task_id: &str) -> Result<(), DomainError> {
        let Some(ref store) = self.run_store else {
            return Ok(());
        };

        let mut recovered_any = false;
        let active_workers = store.list_worker_runs_for_task(task_id).await?;
        let active_runs_guard = self.active_runs.read().await;

        for mut worker in active_workers
            .into_iter()
            .filter(|w| w.status == RunStatus::Active)
        {
            let Some(run_id) = worker.run_id.clone() else {
                continue;
            };

            // If the run is actively running in-memory in this process, do not disrupt it
            if active_runs_guard.contains_key(&run_id) {
                continue;
            }

            let Some(mut run) = store.get_run(&run_id).await? else {
                continue;
            };

            let model_mode =
                run.metadata.get("actual_mode").and_then(|v| v.as_str()) == Some("model");
            let has_model_output = run
                .metadata
                .get("output")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            if run.status == RunStatus::Active {
                if model_mode && has_model_output {
                    run.transition(RunStatus::Completed)?;
                    run.metadata["completion_reason"] =
                        serde_json::json!("recovered_model_turn_completed");
                    worker.transition(RunStatus::Completed)?;
                } else {
                    run.transition(RunStatus::Failed)?;
                    run.metadata["completion_reason"] =
                        serde_json::json!("recovered_stale_active_turn");
                    worker.transition(RunStatus::Failed)?;
                }
                store.save_run(&mut run).await?;
                store.save_worker_run(&mut worker).await?;
                recovered_any = true;
            }
        }
        drop(active_runs_guard);

        if recovered_any {
            if let Some(claim) = self.dispatcher.get_active_claim(task_id, None).await {
                self.dispatcher.release_claim(&claim.id).await?;
            }
            if let Some(mut claim) = store.get_active_claim(task_id, None).await? {
                claim.release()?;
                store.update_claim(&claim).await?;
            }
        }

        Ok(())
    }
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self::new()
    }
}

mod execution;
mod port;

#[cfg(test)]
mod tests;
