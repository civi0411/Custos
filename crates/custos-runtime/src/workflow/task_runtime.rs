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
use custos_core::SessionStore;
use custos_domain::{
    new_id, Assurance, CancelReceipt, ContextPack, ContinuationPacket, DomainError, Run, RunHandle,
    RunStatus, SessionId, StartRunCommand, TaskStatus, WorkerRun,
};
use custos_provider::request::ProviderRequest;
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
    outbox: Option<Arc<dyn OutboxPort>>,
    effect_ledger: Option<Arc<dyn EffectLedgerPort>>,
    run_store: Option<Arc<dyn RunPort>>,
    session_store: Option<Arc<dyn SessionStore>>,
    dispatcher: Arc<super::dispatcher::WorkflowDispatcher>,
    lease_manager: Option<Arc<super::lease::WorkspaceLeaseManager>>,
    workspace_coordinator: Option<Arc<crate::workspace::WorkspaceCoordinator>>,
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
            outbox: None,
            effect_ledger: None,
            run_store: None,
            session_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            workspace_coordinator: None,
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
            outbox: None,
            effect_ledger: None,
            run_store: None,
            session_store: None,
            dispatcher: Arc::new(super::dispatcher::WorkflowDispatcher::new()),
            lease_manager: None,
            workspace_coordinator: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_journal_manager(mut self, manager: Arc<crate::session::SessionManager>) -> Self { self.journal_manager = Some(manager); self }

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
        lease_manager: Arc<super::lease::WorkspaceLeaseManager>,
    ) -> Self {
        self.lease_manager = Some(lease_manager);
        self
    }

    pub fn lease_manager(&self) -> Option<&Arc<super::lease::WorkspaceLeaseManager>> {
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
                store.save_run(&run).await?;
                store.save_worker_run(&worker).await?;
                recovered_any = true;
            }
        }
        drop(active_runs_guard);

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


impl TaskRuntime {
    async fn dispatch_run(&self, cmd: StartRunCommand, background: bool) -> Result<RunHandle, DomainError> {

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

        if background && !matches!(actual_mode,"model"|"native") {
            return Err(DomainError::Validation("No executable provider is configured for this background mode".into()));
        }

        self.recover_completed_model_chat_turn(&cmd.task_id).await?;

        // 1. Atomic claim via WorkflowDispatcher (Gate A Fencing & double-dispatch prevention)
        let claim = self
            .dispatcher
            .claim_ready_task(&cmd.task_id, None, &cmd.actor, 1)
            .await?;

        let mut provider_account_id = serde_json::Value::Null;
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

            provider_account_id = task.metadata.get("provider_account_id").cloned().unwrap_or(serde_json::Value::Null);
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

        // 4. Instantiate and activate Run with mode recording
        let mut run = Run::new(cmd.task_id.clone(), 1);
        run.workflow_revision = cmd.workflow_revision.clone();
        run.transition(RunStatus::Active)?;

        run.metadata["provider_account_id"] = provider_account_id;
        run.metadata["background"] = serde_json::json!(background);
        run.metadata["requested_model"] = serde_json::json!(cmd.model);
        run.metadata["reasoning_effort"] = serde_json::json!(cmd.reasoning_effort);
        if let Some(account) = &cmd.provider_account_id { run.metadata["provider_account_id"] = serde_json::json!(account); }
        run.metadata["session_id"] = serde_json::json!(cmd.session_id);
        run.metadata["turn_id"] = serde_json::json!(cmd.turn_id.clone().unwrap_or_else(||new_id("turn")));
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
        let lease = if let Some(ref lm) = self.lease_manager {
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

        if let Some(ref ws_id) = cmd.workspace_id {
            run.metadata["workspace_id"] = serde_json::json!(ws_id);
        }
        if let Some(ref ws_root) = cmd.workspace_root {
            run.metadata["workspace_root"] = serde_json::json!(ws_root);
        }

        let token = match self.live.register(&run, &cmd) {
            Ok(token)=>token,
            Err(error)=>{
                if let Some(mut lease)=lease {if let Err(cleanup)=lease.release() {tracing::warn!(run_id=%run.id,error=%cleanup,"Lease cleanup failed after live admission error");}}
                self.dispatcher.release_claim(&claim.id).await?;
                run.transition(RunStatus::Failed)?;wrun.transition(RunStatus::Failed)?;
                run.metadata["failure_reason"]=serde_json::json!(error.to_string());
                if let Some(store)=&self.run_store {store.save_run(&run).await?;store.save_worker_run(&wrun).await?;}
                return Err(error);
            }
        };
        self.active_runs.write().await.insert(run.id.clone(), run.clone());
        self.active_workers.write().await.insert(wrun.id.clone(), wrun.clone());
        let handle = RunHandle { run_id: run.id.clone(), task_id: run.task_id.clone(), status: run.status, started_at: run.started_at, completed_at: None, output: None };
        self.live.emit(&run.id, "status", serde_json::json!({"status":"queued"}));
        let actual_mode = actual_mode.to_string(); let requested_mode = requested_mode.to_string();
        if background {
            let runtime = self.clone();
            let live_run_id = run.id.clone();
            let live_session = cmd.session_id.clone().unwrap_or_default();
            tokio::spawn(async move {
                if let Err(error) = runtime.execute_registered(cmd, run, wrun, claim.id, lease, actual_mode, requested_mode, token, true).await {
                    tracing::warn!(run_id = %live_run_id, error = %error, "Background run failed");
                    let partial = runtime.live.replay(&live_session, &live_run_id, 0).ok().map(|r|r.snapshot.text).unwrap_or_default();
                    runtime.live.emit(&live_run_id,"result",serde_json::json!({"status":"failed","text":partial,"error":error.to_string()}));
                }
            });
            Ok(handle)
        } else { self.execute_registered(cmd, run, wrun, claim.id, lease, actual_mode, requested_mode, token, false).await }
    }

    async fn persist_live_journal(&self, cmd: &StartRunCommand, run: &mut Run, text: &str) -> Result<(), DomainError> {
        if let (Some(session), Some(manager)) = (&cmd.session_id, &self.journal_manager) {
            let message = if text.is_empty() { format!("Run {} {}", run.id, run.status) } else { text.to_string() };
            let session_id=SessionId(session.clone());
            let entry=manager.append_assistant_entry(&session_id, &message).await?;
            run.metadata["journal_occurred_at"]=serde_json::json!(entry.occurred_at);
            if let Some(store)=&self.run_store {store.save_run(run).await?;}
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_registered(&self, cmd: StartRunCommand, mut run: Run, mut wrun: WorkerRun, claim_id: String, mut lease: Option<super::lease::WorkspaceLease>, actual_mode: String, requested_mode: String, token: tokio_util::sync::CancellationToken, background: bool) -> Result<RunHandle, DomainError> {
        tracing::info!(run_id = %run.id, task_id = %run.task_id, "Executing registered run");
        self.live.emit(&run.id, "status", serde_json::json!({"status":"running"}));
        let observer = self.live.observer(&run.id, token.clone());
        // Execute governed agent turn based on actual_mode
        let turn_result = {
        let turn_result = async {
            let mut turn_output: Option<String> = None;
            if actual_mode == "native" {
                let harness = self.resolve_harness(cmd.harness_id.as_deref())?;
                let profile = harness.profile();
                let harness_id = profile.harness_id.clone();

                let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, &requested_mode)?;
                if let Some(ref store) = self.run_store {
                    store.save_launch_attempt(&launch).await?;
                }
                run.metadata["launch_attempt_id"] = serde_json::json!(launch.id);
                run.metadata["harness_id"] = serde_json::json!(harness_id);
                run.metadata["tool_mediation"] =
                    serde_json::json!(profile.tool_mediation.to_string());
                wrun.continuation_packet_ref = Some(format!("launch:{}", launch.id));

                // Resolve execution workspace & effective directory
                let (base_workspace_dir, maybe_resolved_ws_id) = if let Some(ref ws_coord) = self.workspace_coordinator {
                    if let Some(ref ws_id_str) = cmd.workspace_id {
                        let ws_id = custos_domain::WorkspaceId::new(ws_id_str);
                        if let Ok(Some(ws)) = ws_coord.get_workspace(&ws_id).await {
                            (std::path::PathBuf::from(ws.path), Some(ws_id))
                        } else if let Some(ref root) = cmd.workspace_root {
                            (std::path::PathBuf::from(root), Some(ws_id))
                        } else {
                            (std::path::PathBuf::from("."), Some(ws_id))
                        }
                    } else if let Some(ref root) = cmd.workspace_root {
                        let matched = if let Ok(list) = ws_coord.list_workspaces().await {
                            list.into_iter().find(|w| w.path == *root).map(|w| w.id)
                        } else {
                            None
                        };
                        (std::path::PathBuf::from(root), matched)
                    } else {
                        (std::path::PathBuf::from("."), None)
                    }
                } else if let Some(ref root) = cmd.workspace_root {
                    (std::path::PathBuf::from(root), None)
                } else {
                    (std::path::PathBuf::from("."), None)
                };

                let (effective_dir, isolation_level) = if let Some(ref l) = lease {
                    (l.lease_path().to_path_buf(), "isolated_worktree_lease")
                } else {
                    (base_workspace_dir.clone(), "shared_live_checkout")
                };

                if let Some(ref ws_id) = maybe_resolved_ws_id {
                    run.metadata["workspace_id"] = serde_json::json!(ws_id.to_string());
                }
                run.metadata["isolation_level"] = serde_json::json!(isolation_level);
                run.metadata["effective_workspace_path"] = serde_json::json!(effective_dir.display().to_string());

                // Capture baseline commit before execution if workspace coordinator is available
                if let (Some(ref ws_coord), Some(ref ws_id)) = (&self.workspace_coordinator, &maybe_resolved_ws_id) {
                    if let Ok(dirty) = ws_coord.inspect_dirty(ws_id).await {
                        if let Some(head) = dirty.head_commit {
                            run.metadata["base_commit_hash"] = serde_json::json!(head);
                        }
                    }
                }

                let intents = if let Some(prompt_text) = cmd.prompt.as_deref() {
                    let native_result = harness
                        .run_native_observed(prompt_text, &effective_dir, observer.clone())
                        .await;
                    let execution = match native_result {
                        Ok(result) => {
                            launch.mark_launched("native", Some(harness_id.clone()))?;
                            if let Some(ref store) = self.run_store {
                                store.save_launch_attempt(&launch).await?;
                            }
                            result
                        }
                        Err(err) => {
                            launch.mark_failed(err.to_string())?;
                            if let Some(ref store) = self.run_store {
                                store.save_launch_attempt(&launch).await?;
                            }
                            return Err(err);
                        }
                    };

                    for intent in &execution.observed_effects {
                        profile.validate_intent_assurance(intent)?;
                    }

                    run.metadata["exit_code"] = serde_json::json!(execution.exit_code);
                    run.metadata["native_success"] = serde_json::json!(execution.success);
                    run.metadata["stderr"] = serde_json::json!(execution.stderr.clone());
                    run.metadata["observed_effect_count"] =
                        serde_json::json!(execution.observed_effects.len());
                    let combined_output = if execution.stdout.trim().is_empty() {
                        execution.stderr.clone()
                    } else {
                        execution.stdout.clone()
                    };
                    run.metadata["output"] = serde_json::json!(combined_output);
                    turn_output = Some(combined_output);

                    // Post-execution: Capture Git dirty status and unified diff
                    if let Some(ref l) = lease {
                        if let Ok(files) = l.collect_modified_files() {
                            let rel_strs: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
                            run.metadata["isolated_modified_files"] = serde_json::json!(rel_strs);
                        }
                        let diff_out = std::process::Command::new("git")
                            .current_dir(l.lease_path())
                            .args(["diff", "HEAD"])
                            .output();
                        if let Ok(out) = diff_out {
                            let diff_str = String::from_utf8_lossy(&out.stdout).to_string();
                            if !diff_str.is_empty() {
                                run.metadata["isolated_worktree_diff"] = serde_json::json!(diff_str);
                            }
                        }
                    } else if let (Some(ref ws_coord), Some(ref ws_id)) = (&self.workspace_coordinator, &maybe_resolved_ws_id) {
                        if let Ok(dirty) = ws_coord.inspect_dirty(ws_id).await {
                            run.metadata["dirty_manifest"] = serde_json::to_value(&dirty).unwrap_or_default();
                            run.metadata["diff_stat"] = serde_json::json!({
                                "is_dirty": dirty.is_dirty,
                                "modified_count": dirty.modified_files.len(),
                                "untracked_count": dirty.untracked_files.len(),
                                "deleted_count": dirty.deleted_files.len(),
                                "modified_files": dirty.modified_files,
                                "untracked_files": dirty.untracked_files,
                                "deleted_files": dirty.deleted_files,
                            });
                        }
                        if let Ok(diff_patch) = ws_coord.diff(ws_id).await {
                            if !diff_patch.is_empty() {
                                run.metadata["diff_patch"] = serde_json::json!(diff_patch);
                            }
                        }
                    }

                    execution.observed_effects
                } else {
                    launch.mark_launched("native", Some(harness_id.clone()))?;
                    if let Some(ref store) = self.run_store {
                        store.save_launch_attempt(&launch).await?;
                    }
                    let context_pack =
                        ContextPack::new(new_id("ctx"), Vec::new(), 0, "sha256:empty".into());
                    harness.execute_turn(&wrun, &context_pack).await?
                };

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
                        // Native/provider-governed effects have no Custos dispatch receipt.
                        // Keep them uncertain until a post-read verifier reconciles the effect.
                        let idempotency_key = intent
                            .idempotency_key
                            .clone()
                            .unwrap_or_else(|| intent.id.clone());
                        let mut effect = custos_domain::EffectAttempt::new(
                            wrun.id.clone(),
                            "native-unmediated",
                            idempotency_key,
                        );
                        effect.status = custos_domain::EffectStatus::Uncertain;
                        let _ = ledger.record_effect(&effect).await;
                    }
                }
            } else if actual_mode == "model" {
                if let Some(ref model) = self.model {
                    let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, &requested_mode)?;
                    let mut prompt_text = cmd
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

                    if let (Some(session_id), Some(session_store)) =
                        (cmd.session_id.as_ref(), self.session_store.as_ref())
                    {
                        let session_id = SessionId::new(session_id.clone());
                        if session_store.get_session(&session_id).await?.is_none() {
                            return Err(DomainError::Validation(format!(
                                "Unknown session '{}' for model run",
                                session_id
                            )));
                        }
                        let journal = session_store.get_journal(&session_id).await?;
                        let compiled =
                            crate::context::compile_session_history(&journal, &prompt_text, 12_000);
                        run.metadata["session_context"] = serde_json::json!({
                            "compiler_version": "journal-recent-v1",
                            "session_id": session_id.0,
                            "included_entry_ids": compiled.included_entry_ids,
                            "omitted_entry_ids": compiled.omitted_entry_ids,
                            "estimated_tokens": compiled.estimated_tokens,
                            "destination_model": target_model,
                        });
                        prompt_text = compiled.prompt;
                    }

                    let mut req = ProviderRequest::new(
                        new_id("req"),
                        &cmd.task_id,
                        1,
                        prompt_text,
                        target_model,
                    );
                    req.reasoning_effort = run.metadata.get("reasoning_effort").and_then(|v| v.as_str()).map(str::to_string);
                    req.provider_account_id = run.metadata.get("provider_account_id").and_then(|v| v.as_str()).map(str::to_string);
                    let mut receiver = model.stream(&req).await?;
                    let mut content = String::new();
                    let mut usage = custos_provider::TokenUsage::default();
                    let mut completed = false;
                    let mut actual_model = target_model.to_string();
                    let mut tokens_used = None;
                    while let Some(event) = receiver.recv().await {
                        use custos_provider::ProviderEventType;
                        match event.event_type {
                            ProviderEventType::Chunk => {
                                if let Some(text) = event.text {
                                    if content.len().saturating_add(text.len()) > 2 * 1024 * 1024 { return Err(DomainError::Validation("Model output exceeded live limit".into())); }
                                    observer.emit("status", serde_json::json!({"status":"running"}));
                                    content.push_str(&text);
                                    observer.emit("text_delta", serde_json::json!({"text":text}));
                                }
                            },
                            ProviderEventType::Thinking => observer.emit("status", serde_json::json!({"status":"thinking"})),
                            ProviderEventType::ReasoningSummary => {
                                observer.emit("status", serde_json::json!({"status":"thinking"}));
                                observer.emit("reasoning_summary", serde_json::json!({"text":event.text}));
                            },
                            ProviderEventType::ToolCall => {
                                observer.emit("status", serde_json::json!({"status":"tool-running"}));
                                observer.emit("tool_input", serde_json::json!({"calls":event.tool_calls}));
                            },
                            ProviderEventType::ToolOutput => observer.emit("tool_output", serde_json::json!({"text":event.text})),
                            ProviderEventType::Usage => {},
                            ProviderEventType::Completed => {
                                if let Some(text) = event.text { content = text; }
                                if let Some(model) = event.model_id { actual_model = model; }
                                tokens_used = event.tokens_used;
                                completed = true;
                            },
                            ProviderEventType::Failed => return Err(DomainError::Validation(event.error.unwrap_or_else(|| "Provider stream failed".into()))),
                        }
                        if let Some(measured) = event.usage {
                            if measured.input_tokens.is_some() { usage.input_tokens = measured.input_tokens; }
                            if measured.output_tokens.is_some() { usage.output_tokens = measured.output_tokens; }
                            observer.emit("usage",serde_json::json!({"input_tokens":usage.input_tokens,"output_tokens":usage.output_tokens,"total_tokens":usage.input_tokens.zip(usage.output_tokens).map(|(a,b)|a.saturating_add(b))}));
                        }
                        if completed { break; }
                    }
                    if !completed { return Err(DomainError::Validation("Provider stream ended without completion".into())); }
                    let known_total=usage.input_tokens.zip(usage.output_tokens).map(|(input,output)|input.saturating_add(output));
                    let resp = custos_provider::ModelResponse::text(content, actual_model, tokens_used.or(known_total).or(usage.output_tokens).unwrap_or(0)).with_usage(usage.input_tokens, usage.output_tokens, None);
                    observer.emit("usage", serde_json::json!({"input_tokens":resp.input_tokens,"output_tokens":resp.output_tokens,"total_tokens":resp.total_tokens}));
                    run.metadata["usage"] = serde_json::json!({ "input_tokens": resp.input_tokens, "output_tokens": resp.output_tokens, "total_tokens": resp.total_tokens });
                    run.metadata["output"] = serde_json::json!(resp.content);
                    run.metadata["tokens_used"] = serde_json::json!(resp.tokens_used);
                    run.metadata["model_id"] = serde_json::json!(resp.model_id);
                    turn_output = Some(resp.content);
                    if let Some(ref store) = self.run_store {
                        store.save_run(&run).await?;
                        let _ = store.save_worker_run(&wrun).await;
                    }
                }
            }
            Ok::<Option<String>, DomainError>(turn_output)
        }
        ;
        tokio::pin!(turn_result);
        let turn_result = tokio::select! {
            result = &mut turn_result => result,
            _ = token.cancelled() => {
                if actual_mode == "native" {
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), &mut turn_result).await;
                }
                Err(DomainError::Validation("Run cancelled".into()))
            },
        };

        turn_result
        };

        if let Ok(replay)=self.live.replay(cmd.session_id.as_deref().unwrap_or(""),&run.id,0) {
            if let Some(usage)=replay.snapshot.usage {run.metadata["usage"]=usage;}
        }
        let turn_result = if token.is_cancelled() { Err(DomainError::Validation("Run cancelled".into())) } else { turn_result };
        let output_text = match turn_result {
            Ok(out) => out,
            Err(e) => {
                if let Some(ref mut l) = lease {
                    let _ = l.release();
                }
                let terminal = if token.is_cancelled() { RunStatus::Cancelled } else { RunStatus::Failed };
                if token.is_cancelled() { if let Some(kernel)=&self.kernel { kernel.transition_task(&run.task_id,TaskStatus::Cancelled).await?; } }
                let _ = run.transition(terminal);
                let _ = wrun.transition(terminal);
                run.metadata["failure_reason"] = serde_json::json!(e.to_string());
                if let Some(ref store) = self.run_store {
                    let _ = store.save_run(&run).await;
                    let _ = store.save_worker_run(&wrun).await;
                }
                let _ = self.dispatcher.release_claim(&claim_id).await;
                let partial = self.live.replay(cmd.session_id.as_deref().unwrap_or(""), &run.id, 0).ok().map(|r| r.snapshot.text).unwrap_or_default();
                run.metadata["output"] = serde_json::json!(partial);
                if let Some(store) = &self.run_store { store.save_run(&run).await?; }
                self.active_runs.write().await.insert(run.id.clone(), run.clone());
                self.active_workers.write().await.insert(wrun.id.clone(), wrun.clone());
                if background { self.persist_live_journal(&cmd, &mut run, &partial).await?; }
                self.live.emit(&run.id, "result", serde_json::json!({"status":run.status.to_string(),"text":partial,"error":e.to_string()}));
                return Err(e);
            }
        };

        let is_chat_turn = cmd.session_id.is_some()
            || cmd.turn_id.is_some()
            || cmd.actor == "chat"
            || cmd.actor == "actor_user";

        if is_chat_turn && actual_mode == "model" && run.status == RunStatus::Active {
            run.transition(RunStatus::Completed)?;
            wrun.transition(RunStatus::Completed)?;
            run.metadata["completion_reason"] = serde_json::json!("model_turn_completed");
            if let Some(ref store) = self.run_store {
                store.save_run(&run).await?;
                store.save_worker_run(&wrun).await?;
            }
        }

        if actual_mode == "native" && run.status == RunStatus::Active {
            if let Some(native_succeeded) = run
                .metadata
                .get("native_success")
                .and_then(|value| value.as_bool())
            {
                if !native_succeeded {
                    run.metadata["failure_reason"] =
                        serde_json::json!("Native process failed; see the captured output");
                }
                run.transition(if native_succeeded {
                    RunStatus::Completed
                } else {
                    RunStatus::Failed
                })?;
                wrun.transition(if native_succeeded {
                    RunStatus::Completed
                } else {
                    RunStatus::Failed
                })?;
                run.metadata["completion_reason"] = serde_json::json!(if native_succeeded {
                    "native_turn_completed"
                } else {
                    "native_process_failed"
                });
                run.metadata["turn_completion"] = serde_json::json!("process_exit");
                run.metadata["task_criteria_verified"] = serde_json::json!(false);
                run.metadata["verification_status"] = serde_json::json!("unverified_external_effects_require_evidence");

                if let Some(ref outbox) = self.outbox {
                    if let Ok(uncertain) = outbox.list_by_status(OutboxStatus::Uncertain).await {
                        run.metadata["uncertain_effects_count"] = serde_json::json!(uncertain.len());
                    }
                } else if let Some(count) = run.metadata.get("observed_effect_count") {
                    run.metadata["uncertain_effects_count"] = count.clone();
                }

                if let Some(ref store) = self.run_store {
                    store.save_run(&run).await?;
                    store.save_worker_run(&wrun).await?;
                }
            }
        }

        if run.status.is_terminal() {
            if let Some(ref mut l) = lease {
                let _ = l.release();
            }
            let _ = self.dispatcher.release_claim(&claim_id).await;
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
        self.active_runs.write().await.insert(run.id.clone(), run.clone());
        self.active_workers
            .write()
            .await
            .insert(wrun.id.clone(), wrun);

        if let Some(store)=&self.run_store {store.save_run(&run).await?;}
        if background { self.persist_live_journal(&cmd, &mut run, handle.output.as_deref().unwrap_or("")).await?; }
        self.live.emit(&handle.run_id, "result", serde_json::json!({"status":handle.status.to_string(),"text":handle.output,"error":if handle.status==RunStatus::Failed {Some("Native process failed; see the captured output")} else {None}}));
        Ok(handle)
    }
}

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> { self.dispatch_run(cmd, false).await }
    async fn start_run_background(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> { self.dispatch_run(cmd, true).await }
    async fn live_run(&self, session_id: &str, run_id: &str, cursor: u64) -> Result<custos_domain::LiveRunReplay, DomainError> {
        match self.live.replay(session_id, run_id, cursor) {
            Ok(replay) => Ok(replay),
            Err(DomainError::NotFound { .. }) => {
                let run = self.run_store.as_ref().ok_or_else(|| DomainError::Validation("Run storage unavailable".into()))?.get_run(run_id).await?.ok_or_else(|| DomainError::NotFound { kind:"Run".into(), id:run_id.into() })?;
                if run.metadata.get("session_id").and_then(|v|v.as_str()) != Some(session_id) { return Err(DomainError::Validation("Run does not belong to this session".into())); }
                Ok(custos_domain::LiveRunReplay { events: vec![], gap: true, snapshot: custos_domain::LiveRunSnapshot { cursor:0, run_id:run.id, task_id:run.task_id, session_id:session_id.into(), turn_id:run.metadata.get("turn_id").and_then(|v|v.as_str()).unwrap_or("").into(), status:if !run.status.is_terminal() || (run.status==RunStatus::Failed && run.metadata.get("completion_reason").is_none() && run.metadata.get("failure_reason").is_none()) { "interrupted".into() } else { run.status.to_string() }, text:run.metadata.get("output").and_then(|v|v.as_str()).unwrap_or("").into(), activities:vec![], usage:run.metadata.get("usage").cloned(), error:run.metadata.get("failure_reason").and_then(|v|v.as_str()).map(str::to_string).or_else(||if !run.status.is_terminal() || (run.status==RunStatus::Failed && run.metadata.get("completion_reason").is_none()) {Some("Run interrupted by daemon restart; send a new prompt to continue".into())} else {None}) } })
            },
            Err(error) => Err(error),
        }
    }

    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError> {
        if self.live.cancel(run_id) {
            let task_id = self.active_runs.read().await.get(run_id).map(|run| run.task_id.clone()).unwrap_or_default();
            let harness_id = self.active_runs.read().await.get(run_id).and_then(|r| r.metadata.get("harness_id").and_then(|h| h.as_str().map(str::to_string)));
            if let Some(ref hid) = harness_id {
                if let Ok(harness) = self.resolve_harness(Some(hid)) {
                    let _ = harness.cancel_run(run_id).await;
                }
            }
            tracing::info!(run_id = run_id, task_id = %task_id, "Live run cancellation requested");
            let uncertain_effects_count = if let Some(outbox) = &self.outbox {
                let uncertain = outbox.list_by_status(OutboxStatus::Uncertain).await?;
                let dispatching = outbox.list_by_status(OutboxStatus::Dispatching).await?;
                u32::try_from(uncertain.len().saturating_add(dispatching.len())).unwrap_or(u32::MAX)
            } else { 0 };
            return Ok(CancelReceipt { run_id: run_id.into(), cancelled_at: Utc::now(), reason: reason.into(), uncertain_effects_count });
        }
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
            if let Ok(wruns) = store.list_worker_runs_for_task(&run.task_id).await {
                for mut w in wruns.into_iter().filter(|w| w.status == RunStatus::Active) {
                    if w.transition(RunStatus::Cancelled).is_ok() {
                        let _ = store.save_worker_run(&w).await;
                    }
                }
            }
        }

        let selected_harness = run
            .metadata
            .get("harness_id")
            .and_then(|value| value.as_str())
            .and_then(|id| self.harnesses.get(id).cloned())
            .or_else(|| self.harness.clone());
        if let Some(harness) = selected_harness {
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
    use custos_core::contracts::harness::{
        HarnessExecutionResult, HarnessProfile, ToolMediationLevel,
    };
    use custos_domain::{ActionIntent, ContextPack};
    use std::path::Path;
    use std::sync::Mutex;

    struct RecordingHarness {
        id: String,
        calls: Arc<Mutex<Vec<(String, String)>>>,
    }

    #[async_trait]
    impl AgentRuntimePort for RecordingHarness {
        fn harness_id(&self) -> &str {
            &self.id
        }

        fn profile(&self) -> HarnessProfile {
            HarnessProfile::new(&self.id, ToolMediationLevel::ProviderGoverned)
        }

        async fn execute_turn(
            &self,
            _worker_run: &WorkerRun,
            _context_pack: &ContextPack,
        ) -> Result<Vec<ActionIntent>, DomainError> {
            Ok(Vec::new())
        }

        async fn run_native(
            &self,
            instruction: &str,
            cwd: &Path,
        ) -> Result<HarnessExecutionResult, DomainError> {
            self.calls
                .lock()
                .map_err(|_| DomainError::Validation("recording harness lock poisoned".into()))?
                .push((instruction.to_string(), cwd.display().to_string()));
            Ok(HarnessExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: format!("{} completed", self.id),
                stderr: String::new(),
                observed_effects: Vec::new(),
                mediation_level: ToolMediationLevel::ProviderGoverned,
            })
        }
    }

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

    struct SlowStreamModel;
    #[async_trait]
    impl ModelPort for SlowStreamModel {
        fn provider_id(&self)->&str {"slow"}
        async fn generate(&self,_:&ProviderRequest)->Result<custos_provider::ModelResponse,DomainError> {Err(DomainError::Validation("Streaming only".into()))}
        async fn stream(&self,req:&ProviderRequest)->Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>,DomainError> {
            let (tx,rx)=tokio::sync::mpsc::channel(4);let id=req.request_id.clone();
            tokio::spawn(async move {
                tx.send(custos_provider::ProviderEvent::chunk(&id,"partial")).await.unwrap();
                tokio::select! {_=tx.closed()=>{},_=tokio::time::sleep(std::time::Duration::from_secs(30))=>{}}
            });Ok(rx)
        }
    }
    #[tokio::test]
    async fn background_returns_early_and_stop_preserves_partial() {
        let runtime=TaskRuntime::new().with_model(Arc::new(SlowStreamModel));
        let cmd=StartRunCommand::new("background-task","chat").with_preferred_mode("model").with_prompt("hello").with_session_id("session").with_turn_id("turn");
        let handle=tokio::time::timeout(std::time::Duration::from_secs(1),runtime.start_run_background(cmd)).await.unwrap().unwrap();
        assert_eq!(handle.status,RunStatus::Active);
        for _ in 0..100 {if runtime.live_run("session",&handle.run_id,0).await.unwrap().snapshot.text=="partial" {break;}tokio::time::sleep(std::time::Duration::from_millis(5)).await;}
        assert_eq!(runtime.live_run("session",&handle.run_id,0).await.unwrap().snapshot.text,"partial");
        runtime.request_cancel(&handle.run_id,"test").await.unwrap();
        for _ in 0..100 {if runtime.live_run("session",&handle.run_id,0).await.unwrap().snapshot.status=="cancelled" {break;}tokio::time::sleep(std::time::Duration::from_millis(5)).await;}
        let snapshot=runtime.live_run("session",&handle.run_id,0).await.unwrap().snapshot;assert_eq!(snapshot.status,"cancelled");assert_eq!(snapshot.text,"partial");
    }

    #[tokio::test]
    async fn durable_background_result_restores_without_duplicate_journal() {
        use custos_core::TaskStore;
        use custos_core::contracts::storage::RunPort;
        let store=Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task=custos_domain::Task::new("durable-live-task".into(),"Durable live".into());store.save_task(&task).await.unwrap();
        let sessions=Arc::new(crate::session::SessionManager::with_store(store.clone()));
        let session=sessions.create_session(custos_domain::SessionMode::Assisted).await;
        let runtime=TaskRuntime::new().with_run_store(store.clone()).with_model(Arc::new(TestMockModel)).with_journal_manager(sessions.clone());
        let cmd=StartRunCommand::new(&task.id,"chat").with_preferred_mode("model").with_prompt("hello").with_session_id(session.id.to_string()).with_turn_id("durable-turn");
        let handle=runtime.start_run_background(cmd).await.unwrap();
        for _ in 0..100 {if runtime.live_run(&session.id.to_string(),&handle.run_id,0).await.unwrap().snapshot.status=="completed" {break;}tokio::time::sleep(std::time::Duration::from_millis(5)).await;}
        let run=store.get_run(&handle.run_id).await.unwrap().unwrap();assert_eq!(run.status,RunStatus::Completed);assert_eq!(run.metadata["background"],true);assert!(run.metadata["journal_occurred_at"].is_string());
        let restarted=TaskRuntime::new().with_run_store(store.clone());
        let snapshot=restarted.live_run(&session.id.to_string(),&handle.run_id,99).await.unwrap();assert!(snapshot.gap);assert_eq!(snapshot.snapshot.status,"completed");assert!(snapshot.snapshot.text.contains("Completed turn response"));
        assert_eq!(sessions.get_journal(&session.id).await.unwrap().entries.iter().filter(|e|e.entry_type=="assistant_message").count(),1);
        let mut interrupted=custos_domain::Run::new(task.id.clone(),2);interrupted.transition(RunStatus::Active).unwrap();interrupted.metadata=serde_json::json!({"session_id":session.id,"background":true});store.save_run(&interrupted).await.unwrap();
        assert_eq!(restarted.live_run(&session.id.to_string(),&interrupted.id,0).await.unwrap().snapshot.status,"interrupted");
    }

    #[tokio::test]
    async fn live_capacity_rejection_releases_dispatch_claim() {
        let runtime=TaskRuntime::new().with_model(Arc::new(TestMockModel));
        for _ in 0..256 {
            let run=Run::new("retained".into(),1);
            runtime.live.register(&run,&StartRunCommand::new("retained","chat")).unwrap();
        }
        assert!(runtime.start_run_background(StartRunCommand::new("capacity-task","chat").with_preferred_mode("model").with_prompt("hello")).await.is_err());
        assert!(runtime.dispatcher.get_active_claim("capacity-task",None).await.is_none());
    }

    #[tokio::test]
    async fn legacy_worker_without_chat_identity_remains_active() {
        let runtime=TaskRuntime::new().with_model(Arc::new(TestMockModel));
        let handle=runtime.start_run(StartRunCommand::new("legacy-worker","worker").with_preferred_mode("model").with_prompt("hello")).await.unwrap();
        assert_eq!(handle.status,RunStatus::Active);
        runtime.request_cancel(&handle.run_id,"test cleanup").await.unwrap();
    }

    #[tokio::test]
    async fn rejected_background_mode_does_not_persist_a_phantom_run() {
        use custos_core::{TaskStore,contracts::storage::RunPort};
        let store=Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let task=custos_domain::Task::new("unconfigured-task".into(),"Unconfigured".into());store.save_task(&task).await.unwrap();
        let runtime=TaskRuntime::new().with_run_store(store.clone());
        assert!(runtime.start_run_background(StartRunCommand::new(&task.id,"chat").with_preferred_mode("model")).await.is_err());
        assert!(store.list_runs_for_task(&task.id).await.unwrap().is_empty());
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

    struct MeasuredAccountModel;
    #[async_trait]
    impl ModelPort for MeasuredAccountModel {
        fn provider_id(&self) -> &str { "test" }
        async fn generate(&self, req: &ProviderRequest) -> Result<custos_provider::ModelResponse,DomainError> {
            assert_eq!(req.provider_account_id.as_deref(),Some("account-A"));
            assert_eq!(req.reasoning_effort.as_deref(),Some("ultra"));
            Ok(custos_provider::ModelResponse::text("answer","ollama/test",10).with_usage(Some(7),Some(3),Some(10)))
        }
    }
    #[tokio::test]
    async fn measured_usage_and_account_are_persisted_with_model_run() {
        use custos_core::contracts::kernel::TrustedKernel;
        use custos_core::TaskStore;
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let mut task = custos_domain::Task::new("usage-task".into(),"Measured usage".into());
        task.metadata=serde_json::json!({"provider_account_id":"account-A"});
        store.save_task(&task).await.unwrap();
        let runtime=TaskRuntime::new().with_kernel(Arc::new(TrustedKernel::new(store.clone()))).with_model(Arc::new(MeasuredAccountModel)).with_run_store(store.clone());
        let mut command=StartRunCommand::new(&task.id,"actor").with_preferred_mode("model").with_model("ollama/test").with_prompt("hello");
        command.reasoning_effort=Some("ultra".into());
        command.provider_account_id=Some("account-A".into());
        let handle=runtime.start_run(command).await.unwrap();
        drop(runtime);
        let run=store.get_run(&handle.run_id).await.unwrap().unwrap();
        assert_eq!(run.metadata["provider_account_id"],"account-A");
        assert_eq!(run.metadata["usage"]["total_tokens"],10);
        assert_eq!(run.metadata["usage"]["input_tokens"],7);
        assert_eq!(run.metadata["requested_model"],"ollama/test");
        assert_eq!(run.metadata["reasoning_effort"],"ultra");
    }

    struct TestFailingModel;

    #[async_trait]
    impl custos_provider::ModelPort for TestFailingModel {
        fn provider_id(&self) -> &str {
            "test-fail"
        }

        async fn generate(
            &self,
            _req: &ProviderRequest,
        ) -> Result<custos_provider::request::ModelResponse, DomainError> {
            Err(DomainError::Validation(
                "Simulated model provider failure".into(),
            ))
        }
    }

    #[tokio::test]
    async fn test_task_runtime_failed_model_turn_transitions_to_failed_and_releases_lock() {
        use custos_core::contracts::kernel::TrustedKernel;
        use custos_core::TaskStore;
        use custos_domain::Task;
        use custos_persistence::SqliteTaskStore;

        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task = Task::new("task_fail_recover_01".into(), "Fail and retry task".into());
        store.save_task(&task).await.unwrap();

        let kernel = Arc::new(TrustedKernel::new(store.clone()));
        let model = Arc::new(TestFailingModel);
        let runtime = TaskRuntime::new()
            .with_kernel(kernel)
            .with_model(model)
            .with_run_store(store.clone());

        // First run fails due to model error
        let err = runtime
            .start_run(
                StartRunCommand::new(&task.id, "actor_user")
                    .with_preferred_mode("model")
                    .with_prompt("failing prompt"),
            )
            .await
            .expect_err("Should fail with model error");
        assert!(matches!(err, DomainError::Validation(_)));

        // Verify worker run in store is Failed, not left Active
        let wruns = store.list_worker_runs_for_task(&task.id).await.unwrap();
        assert_eq!(wruns.len(), 1);
        assert_eq!(wruns[0].status, RunStatus::Failed);

        let runs = store.list_runs_for_task(&task.id).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Failed);

        // Second run can immediately proceed without Conflict error
        let working_model = Arc::new(TestMockModel);
        let runtime2 = TaskRuntime::new()
            .with_model(working_model)
            .with_run_store(store.clone());

        let handle2 = runtime2
            .start_run(
                StartRunCommand::new(&task.id, "actor_user")
                    .with_preferred_mode("model")
                    .with_prompt("retry prompt"),
            )
            .await
            .expect("Retry turn should succeed without Conflict error");
        assert_eq!(handle2.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn native_run_uses_requested_harness_and_workspace() {
        let alpha_calls = Arc::new(Mutex::new(Vec::new()));
        let beta_calls = Arc::new(Mutex::new(Vec::new()));
        let mut harnesses: HashMap<String, Arc<dyn AgentRuntimePort>> = HashMap::new();
        harnesses.insert(
            "alpha".into(),
            Arc::new(RecordingHarness {
                id: "alpha".into(),
                calls: alpha_calls.clone(),
            }),
        );
        harnesses.insert(
            "beta".into(),
            Arc::new(RecordingHarness {
                id: "beta".into(),
                calls: beta_calls.clone(),
            }),
        );

        let runtime = TaskRuntime::new().with_harnesses(harnesses, Some("alpha"));
        let command = StartRunCommand::new("task_native", "actor_user")
            .with_preferred_mode("native")
            .with_harness_id("beta")
            .with_workspace_root("/tmp/custos-native-test")
            .with_prompt("inspect the repository");

        let handle = runtime.start_run(command).await.unwrap();
        assert_eq!(handle.status, RunStatus::Completed);
        assert_eq!(handle.output.as_deref(), Some("beta completed"));
        assert!(alpha_calls.lock().unwrap().is_empty());
        assert_eq!(
            beta_calls.lock().unwrap().as_slice(),
            &[(
                "inspect the repository".to_string(),
                "/tmp/custos-native-test".to_string()
            )]
        );
    }
}
