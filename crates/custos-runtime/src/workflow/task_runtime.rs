use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::storage::{
    EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus, RunPort,
};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    new_id, Assurance, CancelReceipt, ContextPack, ContinuationPacket, DomainError, Run, RunHandle,
    RunStatus, StartRunCommand, TaskStatus, WorkerRun,
};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelPort;

pub struct TaskRuntime {
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    harness: Option<Arc<dyn AgentRuntimePort>>,
    sandbox: Option<Arc<dyn SandboxPort>>,
    outbox: Option<Arc<dyn OutboxPort>>,
    effect_ledger: Option<Arc<dyn EffectLedgerPort>>,
    run_store: Option<Arc<dyn RunPort>>,
    dispatcher: Arc<super::dispatcher::WorkflowDispatcher>,
    lease_manager: Option<Arc<super::lease::WorkspaceLeaseManager>>,
    active_runs: Arc<RwLock<HashMap<String, Run>>>,
    active_workers: Arc<RwLock<HashMap<String, WorkerRun>>>,
}

impl TaskRuntime {
    pub fn new() -> Self {
        Self {
            kernel: None,
            model: None,
            harness: None,
            sandbox: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ports(kernel: Arc<dyn KernelPort>, model: Arc<dyn ModelPort>) -> Self {
        Self {
            kernel: Some(kernel),
            model: Some(model),
            harness: None,
            sandbox: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
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
        self.harness = Some(harness);
        self
    }

    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxPort>) -> Self {
        self.sandbox = Some(sandbox);
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

    pub fn with_dispatcher(
        mut self,
        dispatcher: Arc<super::dispatcher::WorkflowDispatcher>,
    ) -> Self {
        self.dispatcher = dispatcher;
        self
    }

    pub fn with_lease_manager(
        mut self,
        lease_manager: Arc<super::lease::WorkspaceLeaseManager>,
    ) -> Self {
        self.lease_manager = Some(lease_manager);
        self
    }

    pub fn lease_manager(&self) -> Option<&Arc<super::lease::WorkspaceLeaseManager>> {
        self.lease_manager.as_ref()
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

    async fn recover_completed_model_chat_turn(&self, task_id: &str) -> Result<(), DomainError> {
        let Some(ref kernel) = self.kernel else {
            return Ok(());
        };
        let Some(ref store) = self.run_store else {
            return Ok(());
        };

        let Some(task) = kernel.get_task(task_id).await? else {
            return Ok(());
        };
        if task.metadata.get("kind").and_then(|v| v.as_str()) != Some("conversation_chat_turn") {
            return Ok(());
        }

        let mut recovered_any = false;
        let active_workers = store.list_worker_runs_for_task(task_id).await?;
        for mut worker in active_workers
            .into_iter()
            .filter(|w| w.status == RunStatus::Active)
        {
            let Some(run_id) = worker.run_id.clone() else {
                continue;
            };
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

            if run.status == RunStatus::Active && model_mode && has_model_output {
                run.transition(RunStatus::Completed)?;
                run.metadata["completion_reason"] =
                    serde_json::json!("recovered_model_turn_completed");
                store.save_run(&run).await?;
                worker.transition(RunStatus::Completed)?;
                store.save_worker_run(&worker).await?;
                recovered_any = true;
            }
        }

        if recovered_any {
            if let Some(claim) = self.dispatcher.get_active_claim(task_id, None).await {
                let _ = self.dispatcher.release_claim(&claim.id).await;
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

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        self.recover_completed_model_chat_turn(&cmd.task_id).await?;

        // 1. Atomic claim via WorkflowDispatcher (Gate A Fencing & double-dispatch prevention)
        let claim = self
            .dispatcher
            .claim_ready_task(&cmd.task_id, None, &cmd.actor, 1)
            .await?;

        // 2. Verify and drive Task lifecycle if KernelPort is available
        if let Some(ref kernel) = self.kernel {
            let task = match kernel.get_task(&cmd.task_id).await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    let _ = self.dispatcher.release_claim(&claim.id).await;
                    return Err(DomainError::NotFound {
                        kind: "Task".into(),
                        id: cmd.task_id.clone(),
                    });
                }
                Err(e) => {
                    let _ = self.dispatcher.release_claim(&claim.id).await;
                    return Err(e);
                }
            };

            if task.status == TaskStatus::Draft {
                if let Err(e) = kernel.transition_task(&task.id, TaskStatus::Queued).await {
                    let _ = self.dispatcher.release_claim(&claim.id).await;
                    return Err(e);
                }
                if let Err(e) = kernel.transition_task(&task.id, TaskStatus::Running).await {
                    let _ = self.dispatcher.release_claim(&claim.id).await;
                    return Err(e);
                }
            } else if task.status == TaskStatus::Queued {
                if let Err(e) = kernel.transition_task(&task.id, TaskStatus::Running).await {
                    let _ = self.dispatcher.release_claim(&claim.id).await;
                    return Err(e);
                }
            } else if task.status.is_terminal() {
                let _ = self.dispatcher.release_claim(&claim.id).await;
                return Err(DomainError::InvalidStateTransition {
                    from: task.status.to_string(),
                    to: "running".into(),
                });
            }
        }

        // 3. Negotiate execution mode and record in metadata (Orca mode sequencing parity)
        let requested_mode = cmd
            .preferred_mode
            .as_deref()
            .unwrap_or(if self.harness.is_some() {
                "native"
            } else {
                "model"
            });

        let (actual_mode, downgraded, reason) = match requested_mode {
            "native" => {
                if self.harness.is_some() {
                    ("native", false, None)
                } else if self.model.is_some() {
                    (
                        "model",
                        true,
                        Some("No harness runtime configured, falling back to model"),
                    )
                } else {
                    ("none", true, Some("Neither harness nor model configured"))
                }
            }
            "model" => {
                if self.model.is_some() {
                    ("model", false, None)
                } else if self.harness.is_some() {
                    (
                        "native",
                        true,
                        Some("No model configured, falling back to harness"),
                    )
                } else {
                    ("none", true, Some("Neither model nor harness configured"))
                }
            }
            other => (other, false, None),
        };

        // 4. Instantiate and activate Run with mode recording
        let mut run = Run::new(cmd.task_id.clone(), 1);
        run.workflow_revision = cmd.workflow_revision;
        run.transition(RunStatus::Active)?;

        run.metadata["requested_mode"] = serde_json::json!(requested_mode);
        run.metadata["actual_mode"] = serde_json::json!(actual_mode);
        if downgraded {
            run.metadata["mode_downgraded"] = serde_json::json!(true);
            if let Some(r) = reason {
                run.metadata["downgrade_reason"] = serde_json::json!(r);
            }
        }
        if let Some(ref root) = cmd.workspace_root {
            run.metadata["workspace_root"] = serde_json::json!(root);
        }

        if let Some(ref store) = self.run_store {
            store.save_run(&run).await?;
        }

        // 5. Instantiate bounded WorkerRun and mark claim dispatched
        let mut wrun = WorkerRun::new(cmd.task_id.clone(), cmd.actor.clone(), 3);
        wrun.run_id = Some(run.id.clone());
        wrun.transition(RunStatus::Active)?;

        if let Some(ref store) = self.run_store {
            store.save_worker_run(&wrun).await?;
        }

        let _ = self.dispatcher.mark_dispatched(&claim.id, &wrun.id).await;

        // 6. Allocate isolated workspace lease if lease manager is configured
        let mut lease = if let Some(ref lm) = self.lease_manager {
            lm.acquire_lease(&cmd.task_id, &wrun.id).ok()
        } else {
            None
        };

        if let Some(ref l) = lease {
            run.metadata["lease_id"] = serde_json::json!(l.lease_id());
            run.metadata["lease_path"] = serde_json::json!(l.lease_path().display().to_string());
            if let Some(ref store) = self.run_store {
                store.save_run(&run).await?;
            }
        }

        // 6. Execute governed agent turn based on actual_mode
        let turn_result = async {
            let mut turn_output: Option<String> = None;
            if actual_mode == "native" {
                if let Some(ref harness) = self.harness {
                    let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, requested_mode)?;
                    let harness_id = harness.profile().harness_id.clone();
                    let _ = launch.mark_launched("native", Some(harness_id));
                    run.metadata["launch_attempt_id"] = serde_json::json!(launch.id);
                    wrun.continuation_packet_ref = Some(format!("launch:{}", launch.id));

                    let context_pack =
                        ContextPack::new(new_id("ctx"), Vec::new(), 0, "sha256:empty".into());

                    let intents = harness.execute_turn(&wrun, &context_pack).await?;

                    // Process generated action intents through Gate 4, Kernel, Outbox, and Sandbox
                    for mut intent in intents {
                        // Gate 4: Validate intent assurance against harness profile
                        harness.profile().validate_intent_assurance(&intent)?;

                        // If intent is custos-mediated and we have kernel and sandbox:
                        if intent.assurance == Assurance::CustosMediated {
                            if let Some(ref kernel) = self.kernel {
                                // Ensure intent is explicitly bound to task
                                intent.task_id = Some(cmd.task_id.clone());

                                // Request permit (Gate 1 Budget, Gate 2 Policy, Gate 3 Idempotency)
                                let permit = kernel
                                    .request_permit(&cmd.task_id, &intent, &cmd.actor)
                                    .await?;
                                let burned_permit =
                                    kernel.consume_permit(&permit.id, &intent).await?;

                                let idempotency_key = intent
                                    .idempotency_key
                                    .clone()
                                    .unwrap_or_else(|| intent.id.clone());

                                // Record effect attempt in effect ledger if available
                                let effect_id = if let Some(ref ledger) = self.effect_ledger {
                                    let mut effect = custos_domain::EffectAttempt::new(
                                        wrun.id.clone(),
                                        burned_permit.id.clone(),
                                        idempotency_key.clone(),
                                    );
                                    effect.status = custos_domain::EffectStatus::InFlight;
                                    ledger.record_effect(&effect).await?;
                                    Some(effect.id)
                                } else {
                                    None
                                };

                                // T3: Transactional Outbox write BEFORE execution
                                let outbox_id = new_id("outbox");
                                if let Some(ref outbox) = self.outbox {
                                    let outbox_entry = OutboxEntry {
                                        id: outbox_id.clone(),
                                        task_id: cmd.task_id.clone(),
                                        action_id: intent.id.clone(),
                                        permit_id: burned_permit.id.clone(),
                                        argument_digest: intent.argument_digest(),
                                        idempotency_key: Some(idempotency_key),
                                        status: OutboxStatus::Pending,
                                        created_at: Utc::now(),
                                        receipt: None,
                                    };
                                    outbox.enqueue(outbox_entry).await?;
                                    outbox.mark_dispatching(&outbox_id).await?;
                                }

                                // T4: Execute in Sandbox
                                if let Some(ref sandbox) = self.sandbox {
                                    let receipt = sandbox.execute(&intent, &burned_permit).await?;

                                    // Update Outbox and EffectLedger with verified receipt
                                    if let Some(ref outbox) = self.outbox {
                                        outbox.mark_receipted(&outbox_id, receipt.clone()).await?;
                                    }
                                    if let Some(ref ledger) = self.effect_ledger {
                                        if let Some(eff_id) = effect_id {
                                            ledger
                                                .update_effect_status(
                                                    &eff_id,
                                                    custos_domain::EffectStatus::Succeeded,
                                                    Some(&receipt),
                                                )
                                                .await?;
                                        }
                                    }
                                }
                            }
                        } else if let Some(ref ledger) = self.effect_ledger {
                            // For observe-only or provider-governed effects, record observation in ledger
                            let idempotency_key = intent
                                .idempotency_key
                                .clone()
                                .unwrap_or_else(|| intent.id.clone());
                            let mut effect = custos_domain::EffectAttempt::new(
                                wrun.id.clone(),
                                "native-unmediated",
                                idempotency_key,
                            );
                            effect.status = custos_domain::EffectStatus::Succeeded;
                            effect.ended_at = Some(Utc::now());
                            let _ = ledger.record_effect(&effect).await;
                        }
                    }
                }
            } else if actual_mode == "model" {
                if let Some(ref model) = self.model {
                    let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, requested_mode)?;
                    let prompt_text = cmd
                        .prompt
                        .clone()
                        .unwrap_or_else(|| format!("Initial run turn for task {}", cmd.task_id));
                    let target_model = cmd
                        .model
                        .as_deref()
                        .filter(|m| !m.trim().is_empty())
                        .unwrap_or_else(|| model.provider_id());

                    let _ = launch.mark_launched("model", Some(target_model.to_string()));
                    run.metadata["launch_attempt_id"] = serde_json::json!(launch.id);
                    wrun.continuation_packet_ref = Some(format!("launch:{}", launch.id));

                    let req = ProviderRequest::new(
                        new_id("req"),
                        &cmd.task_id,
                        1,
                        prompt_text,
                        target_model,
                    );
                    let resp = model.generate(&req).await?;
                    run.metadata["output"] = serde_json::json!(resp.content);
                    run.metadata["tokens_used"] = serde_json::json!(resp.tokens_used);
                    run.metadata["model_id"] = serde_json::json!(resp.model_id);
                    turn_output = Some(resp.content);
                    if let Some(ref store) = self.run_store {
                        let _ = store.save_run(&run).await;
                        let _ = store.save_worker_run(&wrun).await;
                    }
                }
            }
            Ok::<Option<String>, DomainError>(turn_output)
        }
        .await;

        let output_text = match turn_result {
            Ok(out) => out,
            Err(e) => {
                if let Some(ref mut l) = lease {
                    let _ = l.release();
                }
                let _ = self.dispatcher.release_claim(&claim.id).await;
                return Err(e);
            }
        };

        if actual_mode == "model" && run.status == RunStatus::Active {
            run.transition(RunStatus::Completed)?;
            wrun.transition(RunStatus::Completed)?;
            run.metadata["completion_reason"] = serde_json::json!("model_turn_completed");
            if let Some(ref store) = self.run_store {
                let _ = store.save_run(&run).await;
                let _ = store.save_worker_run(&wrun).await;
            }
        }

        if run.status.is_terminal() {
            if let Some(ref mut l) = lease {
                let _ = l.release();
            }
            let _ = self.dispatcher.release_claim(&claim.id).await;
        }

        let handle = RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
            completed_at: run.ended_at,
            output: output_text,
        };

        // 5. Register active executions
        self.active_runs.write().await.insert(run.id.clone(), run);
        self.active_workers
            .write()
            .await
            .insert(wrun.id.clone(), wrun);

        Ok(handle)
    }

    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError> {
        let run = {
            let mut runs = self.active_runs.write().await;
            if let Some(r) = runs.get_mut(run_id) {
                r.transition(RunStatus::Cancelled)?;
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let mut r = store
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.to_string(),
                    })?;
                r.transition(RunStatus::Cancelled)?;
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        if let Some(ref store) = self.run_store {
            let _ = store.save_run(&run).await;
        }

        if let Some(ref harness) = self.harness {
            let _ = harness.cancel_run(run_id).await;
        }

        if let Some(ref kernel) = self.kernel {
            let _ = kernel
                .transition_task(&run.task_id, TaskStatus::Cancelled)
                .await;
        }

        // Release active dispatch claim for this task
        if let Some(claim) = self.dispatcher.get_active_claim(&run.task_id, None).await {
            let _ = self.dispatcher.release_claim(&claim.id).await;
        }

        // Release workspace lease if allocated
        if let Some(lease_id) = run.metadata.get("lease_id").and_then(|v| v.as_str()) {
            if let Some(ref lm) = self.lease_manager {
                let _ = lm.release_lease(lease_id);
            }
        }

        let uncertain_count = if let Some(ref outbox) = self.outbox {
            let uncertain = outbox
                .list_by_status(OutboxStatus::Uncertain)
                .await
                .unwrap_or_default();
            let dispatching = outbox
                .list_by_status(OutboxStatus::Dispatching)
                .await
                .unwrap_or_default();
            (uncertain.len() + dispatching.len()) as u32
        } else {
            0
        };

        Ok(CancelReceipt {
            run_id: run_id.to_string(),
            cancelled_at: Utc::now(),
            reason: reason.to_string(),
            uncertain_effects_count: uncertain_count,
        })
    }

    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError> {
        let run = {
            let mut runs = self.active_runs.write().await;
            if let Some(r) = runs.get_mut(run_id) {
                if r.status == RunStatus::Suspended {
                    r.transition(RunStatus::Active)?;
                }
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let mut r = store
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.to_string(),
                    })?;
                if r.status == RunStatus::Suspended {
                    r.transition(RunStatus::Active)?;
                }
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        if let Some(ref store) = self.run_store {
            let _ = store.save_run(&run).await;
        }

        let handle_output = run
            .metadata
            .get("output")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        Ok(RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
            completed_at: run.ended_at,
            output: handle_output,
        })
    }

    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError> {
        let run = {
            let runs = self.active_runs.read().await;
            if let Some(r) = runs.get(run_id) {
                r.clone()
            } else if let Some(ref store) = self.run_store {
                store
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.to_string(),
                    })?
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

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
        let receipt = runtime
            .request_cancel(&handle.run_id, "user request")
            .await
            .unwrap();
        assert_eq!(receipt.run_id, handle.run_id);
        assert_eq!(receipt.reason, "user request");
    }

    #[tokio::test]
    async fn test_task_runtime_double_dispatch_fencing() {
        let runtime = TaskRuntime::new();
        let cmd1 = StartRunCommand::new("task_fence_01", "actor_alpha");
        let handle1 = runtime
            .start_run(cmd1)
            .await
            .expect("First run should start");

        // Second start_run on the SAME task before completion/cancellation must be refused!
        let cmd2 = StartRunCommand::new("task_fence_01", "actor_beta");
        let err2 = runtime
            .start_run(cmd2)
            .await
            .expect_err("Double dispatch must be refused");

        match err2 {
            DomainError::Conflict(msg) => {
                assert!(msg.contains("task_fence_01"));
                assert!(msg.contains("actor_alpha"));
            }
            other => panic!("Expected Conflict, got {:?}", other),
        }

        // After cancelling run 1, claiming should succeed again
        runtime
            .request_cancel(&handle1.run_id, "cancelled for test")
            .await
            .unwrap();
        let cmd3 = StartRunCommand::new("task_fence_01", "actor_beta");
        let handle3 = runtime
            .start_run(cmd3)
            .await
            .expect("Run should succeed after cancellation");
        assert_eq!(handle3.task_id, "task_fence_01");
    }

    #[tokio::test]
    async fn test_task_runtime_mode_negotiation_downgrade_metadata() {
        let runtime = TaskRuntime::new();
        let cmd = StartRunCommand::new("task_mode_01", "actor_alpha").with_preferred_mode("native");

        let handle = runtime.start_run(cmd).await.unwrap();
        let runs = runtime.active_runs.read().await;
        let run = runs.get(&handle.run_id).unwrap();

        // Since no harness was configured on this TaskRuntime, it recorded downgrade
        assert_eq!(run.metadata["requested_mode"], "native");
        assert_eq!(run.metadata["actual_mode"], "none");
        assert_eq!(run.metadata["mode_downgraded"], true);
    }

    struct TestMockModel;

    #[async_trait]
    impl ModelPort for TestMockModel {
        fn provider_id(&self) -> &str {
            "test_model"
        }

        async fn generate(
            &self,
            req: &ProviderRequest,
        ) -> Result<custos_provider::request::ModelResponse, DomainError> {
            Ok(custos_provider::request::ModelResponse::text(
                format!(
                    "Completed turn response for task {} with prompt: {}",
                    req.task_id, req.prompt
                ),
                "test_model",
                42,
            ))
        }
    }

    #[tokio::test]
    async fn test_task_runtime_model_turn_generates_and_completes() {
        let model = Arc::new(TestMockModel);
        let runtime = TaskRuntime::new().with_model(model);
        let cmd = StartRunCommand::new("task_chat_01", "actor_user")
            .with_preferred_mode("model")
            .with_prompt("Help me build the mobile simulator");

        let handle = runtime.start_run(cmd).await.unwrap();
        assert_eq!(handle.task_id, "task_chat_01");
        assert_eq!(handle.status, RunStatus::Completed);
        assert!(handle.completed_at.is_some());
        assert!(handle.output.is_some());
        let output = handle.output.unwrap();
        assert!(output.contains("Completed turn response for task task_chat_01"));
        assert!(output.contains("Help me build the mobile simulator"));

        let runs = runtime.active_runs.read().await;
        let run = runs.get(&handle.run_id).unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.metadata["actual_mode"], "model");
        assert_eq!(run.metadata["tokens_used"], 42);
        assert!(run.metadata["launch_attempt_id"].is_string());
        assert_eq!(run.metadata["completion_reason"], "model_turn_completed");
        drop(runs);

        // A completed model-only turn releases its claim, so a follow-up can reuse the task.
        let cmd2 = StartRunCommand::new("task_chat_01", "actor_user")
            .with_preferred_mode("model")
            .with_prompt("Second turn follow-up");
        let handle2 = runtime
            .start_run(cmd2)
            .await
            .expect("Follow-up model turn should succeed after claim release");
        assert_eq!(handle2.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn test_task_runtime_recovers_persisted_completed_chat_turn_claim() {
        use custos_core::contracts::kernel::TrustedKernel;
        use custos_core::TaskStore;
        use custos_domain::{DispatchClaim, Task};
        use custos_persistence::SqliteTaskStore;

        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let mut task = Task::new(
            "task_chat_recover_01".into(),
            "Recover completed chat turn".into(),
        );
        task.metadata = serde_json::json!({ "kind": "conversation_chat_turn" });
        store.save_task(&task).await.unwrap();

        let mut stale_run = Run::new(task.id.clone(), 1);
        stale_run.transition(RunStatus::Active).unwrap();
        stale_run.metadata["actual_mode"] = serde_json::json!("model");
        stale_run.metadata["output"] = serde_json::json!("already completed output");
        store.save_run(&stale_run).await.unwrap();

        let mut stale_worker = WorkerRun::new(task.id.clone(), "daemon_user".into(), 3);
        stale_worker.run_id = Some(stale_run.id.clone());
        stale_worker.transition(RunStatus::Active).unwrap();
        store.save_worker_run(&stale_worker).await.unwrap();

        let mut stale_claim = DispatchClaim::new(&task.id, "daemon_user", 1);
        stale_claim.mark_dispatched(&stale_worker.id).unwrap();
        store.claim_ready(&stale_claim).await.unwrap();

        let kernel = Arc::new(TrustedKernel::new(store.clone()));
        let model = Arc::new(TestMockModel);
        let runtime = TaskRuntime::new()
            .with_kernel(kernel)
            .with_model(model)
            .with_run_store(store.clone());

        let handle = runtime
            .start_run(
                StartRunCommand::new(&task.id, "actor_user")
                    .with_preferred_mode("model")
                    .with_prompt("follow-up after stale claim"),
            )
            .await
            .expect("stale completed model chat turn should be recovered before claiming");

        assert_eq!(handle.status, RunStatus::Completed);
        let stale_run_after = store.get_run(&stale_run.id).await.unwrap().unwrap();
        assert_eq!(stale_run_after.status, RunStatus::Completed);
        assert_eq!(
            stale_run_after.metadata["completion_reason"],
            "recovered_model_turn_completed"
        );
        let stale_worker_after = store
            .get_worker_run(&stale_worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stale_worker_after.status, RunStatus::Completed);
        assert!(store
            .get_active_claim(&task.id, None)
            .await
            .unwrap()
            .is_none());
    }
}
