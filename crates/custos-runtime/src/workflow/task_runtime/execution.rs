use super::*;

impl TaskRuntime {
    pub(super) async fn dispatch_run(&self, cmd: StartRunCommand, background: bool) -> Result<RunHandle, DomainError> {

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
            .claim_ready_task(&cmd.task_id, cmd.node_id.as_deref(), &cmd.actor, 1)
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
        if let Some(ref node_id) = cmd.node_id {
            run.metadata["node_id"] = serde_json::json!(node_id);
        }
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
        let host_name = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
        run.metadata["host"] = serde_json::json!(host_name);
        run.metadata["generation"] = serde_json::json!(run.attempt);
        run.metadata["executor_id"] = serde_json::json!(cmd.actor);

        if let Some(ref store) = self.run_store {
            if let Err(e) = store.save_run(&mut run).await {
                if let Err(rel_err) = self.dispatcher.release_claim(&claim.id).await {
                    tracing::error!(claim_id = %claim.id, error = %rel_err, "Failed to release claim after save_run error");
                }
                return Err(e);
            }
        }

        // 5. Instantiate bounded WorkerRun and mark claim dispatched
        let mut wrun = WorkerRun::new(cmd.task_id.clone(), cmd.actor.clone(), 3);
        wrun.run_id = Some(run.id.clone());
        if let Err(e) = wrun.transition(RunStatus::Active) {
            if let Err(rel_err) = self.dispatcher.release_claim(&claim.id).await {
                tracing::error!(claim_id = %claim.id, error = %rel_err, "Failed to release claim after wrun transition error");
            }
            return Err(e);
        }

        if let Some(ref store) = self.run_store {
            if let Err(e) = store.save_worker_run(&mut wrun).await {
                if let Err(rel_err) = self.dispatcher.release_claim(&claim.id).await {
                    tracing::error!(claim_id = %claim.id, error = %rel_err, "Failed to release claim after save_worker_run error");
                }
                return Err(e);
            }
        }

        if let Err(e) = self.dispatcher.mark_dispatched(&claim.id, &wrun.id).await {
            if let Err(rel_err) = self.dispatcher.release_claim(&claim.id).await {
                tracing::error!(claim_id = %claim.id, error = %rel_err, "Failed to release claim after mark_dispatched error");
            }
            return Err(e);
        }

        // Native agents must run in the registered workspace selected by the user.
        // A second implicit lease would be based on the daemon's own cwd and could
        // be removed before the user has reviewed the agent's changes.
        let lease = if actual_mode != "native" {
            if let Some(ref lm) = self.lease_manager {
            lm.acquire_lease(&cmd.task_id, &wrun.id).ok()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ref l) = lease {
            run.metadata["lease_id"] = serde_json::json!(l.lease_id());
            run.metadata["lease_path"] = serde_json::json!(l.lease_path().display().to_string());
            if let Some(ref store) = self.run_store {
                store.save_run(&mut run).await?;
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
                if let Some(store)=&self.run_store {store.save_run(&mut run).await?;store.save_worker_run(&mut wrun).await?;}
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
    async fn execute_registered(&self, cmd: StartRunCommand, mut run: Run, mut wrun: WorkerRun, claim_id: String, mut lease: Option<crate::workflow::lease::WorkspaceLease>, actual_mode: String, requested_mode: String, token: tokio_util::sync::CancellationToken, background: bool) -> Result<RunHandle, DomainError> {
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
                    let ws_id_str = cmd.workspace_id.as_deref().ok_or_else(|| {
                        DomainError::Validation("Native run requires a registered workspace_id".into())
                    })?;
                    let ws_id = custos_domain::WorkspaceId::new(ws_id_str);
                    let ws = ws_coord.get_workspace(&ws_id).await?.ok_or_else(|| {
                        DomainError::Validation(format!("Native workspace '{ws_id_str}' no longer exists"))
                    })?;
                    if !ws.is_ready() {
                        return Err(DomainError::Validation(format!(
                            "Native workspace '{ws_id_str}' is not ready for execution"
                        )));
                    }
                    (std::path::PathBuf::from(ws.path), Some(ws_id))
                } else if let Some(ref root) = cmd.workspace_root {
                    (std::path::PathBuf::from(root), None)
                } else {
                    return Err(DomainError::Validation(
                        "Native run has no execution workspace".into(),
                    ));
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

                        // C1 Invariant: If already executed under GovernedToolExecutor, receipt is complete; skip re-execution
                        if intent.lifecycle_state == custos_domain::ActionLifecycleState::Receipt {
                            continue;
                        }

                        // If intent is custos-mediated and we have kernel and sandbox:
                        if intent.assurance == Assurance::CustosMediated {
                            if let Some(ref kernel) = self.kernel {
                                // Ensure intent is explicitly bound to task
                                intent.task_id = Some(cmd.task_id.clone());

                                // Request permit (Gate 1 Budget, Gate 2 Policy, Gate 3 Idempotency)
                                let permit = match kernel
                                    .request_permit(&cmd.task_id, &intent, &cmd.actor)
                                    .await
                                {
                                    Ok(p) => p,
                                    Err(DomainError::Conflict(reason)) => {
                                        let attempt_id = new_id("att");
                                        if let Some(ref outbox) = self.outbox {
                                            let outbox_entry = OutboxEntry {
                                                id: attempt_id.clone(),
                                                task_id: cmd.task_id.clone(),
                                                action_id: intent.id.clone(),
                                                permit_id: "".to_string(),
                                                argument_digest: intent.argument_digest(),
                                                idempotency_key: Some(intent.idempotency_key.clone().unwrap_or_else(|| intent.id.clone())),
                                                status: OutboxStatus::Pending,
                                                created_at: Utc::now(),
                                                receipt: None,
                                                recipient: None,
                                                payload_json: Some(intent.parameters.to_string()),
                                                error_message: Some(reason.clone()),
                                            };
                                            outbox.enqueue(outbox_entry).await?;
                                        }
                                        run.metadata["pending_approval_attempt_id"] = serde_json::json!(attempt_id);
                                        run.metadata["pending_approval_reason"] = serde_json::json!(reason);
                                        run.transition(RunStatus::Suspended)?;
                                        wrun.transition(RunStatus::Suspended)?;
                                        if let Some(ref kernel) = self.kernel {
                                            let _ = kernel.transition_task(&cmd.task_id, TaskStatus::Blocked).await;
                                        }
                                        self.live.emit(&run.id, "status", serde_json::json!({
                                            "status": "suspended",
                                            "reason": reason,
                                            "attempt_id": attempt_id,
                                        }));
                                        if let Some(ref store) = self.run_store {
                                            store.save_run(&mut run).await?;
                                            store.save_worker_run(&mut wrun).await?;
                                        }
                                        break;
                                    }
                                    Err(e) => return Err(e),
                                };
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
                                    let outbox_entry = OutboxEntry::new(
                                        outbox_id.clone(),
                                        cmd.task_id.clone(),
                                        intent.id.clone(),
                                        burned_permit.id.clone(),
                                        intent.argument_digest(),
                                        Some(idempotency_key),
                                        OutboxStatus::Pending,
                                        Utc::now(),
                                    );
                                    outbox.enqueue(outbox_entry).await?;
                                    outbox.mark_approved(&outbox_id).await?;
                                    outbox.mark_dispatching(&outbox_id).await?;
                                }

                                // T4: Execute in Sandbox
                                if let Some(ref sandbox) = self.sandbox {
                                    match sandbox.execute(&intent, &burned_permit).await {
                                        Ok(receipt) => {
                                            let status = custos_domain::EffectStatus::from(receipt.status);
                                            // Update Outbox and EffectLedger with verified receipt
                                            if let Some(ref outbox) = self.outbox {
                                                outbox.mark_receipted(&outbox_id, receipt.clone()).await?;
                                            }
                                            if let Some(ref ledger) = self.effect_ledger {
                                                if let Some(eff_id) = effect_id {
                                                    ledger
                                                        .update_effect_status(
                                                            &eff_id,
                                                            status,
                                                            Some(&receipt),
                                                        )
                                                        .await?;
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            if let Some(ref outbox) = self.outbox {
                                                let _ = outbox.mark_uncertain(&outbox_id).await;
                                            }
                                            if let Some(ref ledger) = self.effect_ledger {
                                                if let Some(eff_id) = effect_id {
                                                    let _ = ledger.update_effect_status(
                                                        &eff_id,
                                                        custos_domain::EffectStatus::Uncertain,
                                                        None,
                                                    ).await;
                                                }
                                            }
                                            return Err(err);
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
                        if let Err(e) = ledger.record_effect(&effect).await {
                            tracing::warn!(error = %e, "Failed to record unmediated effect attempt in ledger");
                        }
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

                    if let Some(ref store) = self.run_store {
                        store.save_launch_attempt(&launch).await?;
                    }
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
                    let mut receiver = match model.stream(&req).await {
                        Ok(rx) => {
                            launch.mark_launched("model", Some(target_model.to_string()))?;
                            if let Some(ref store) = self.run_store {
                                store.save_launch_attempt(&launch).await?;
                            }
                            rx
                        }
                        Err(err) => {
                            launch.mark_failed(err.to_string())?;
                            if let Some(ref store) = self.run_store {
                                let _ = store.save_launch_attempt(&launch).await;
                            }
                            return Err(err);
                        }
                    };
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
                        store.save_run(&mut run).await?;
                        store.save_worker_run(&mut wrun).await?;
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
                    let _ = store.save_run(&mut run).await;
                    let _ = store.save_worker_run(&mut wrun).await;
                }
                let _ = self.dispatcher.release_claim(&claim_id).await;
                let partial = self.live.replay(cmd.session_id.as_deref().unwrap_or(""), &run.id, 0).ok().map(|r| r.snapshot.text).unwrap_or_default();
                run.metadata["output"] = serde_json::json!(partial);
                if let Some(store) = &self.run_store { store.save_run(&mut run).await?; }
                if self.run_store.is_some() {
                    self.active_runs.write().await.remove(&run.id);
                    self.active_workers.write().await.remove(&wrun.id);
                } else {
                    self.active_runs.write().await.insert(run.id.clone(), run.clone());
                    self.active_workers.write().await.insert(wrun.id.clone(), wrun.clone());
                }
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
                store.save_run(&mut run).await?;
                store.save_worker_run(&mut wrun).await?;
            }
        }

        if is_chat_turn && actual_mode == "native" && run.status == RunStatus::Active {
            let native_succeeded = run
                .metadata
                .get("native_success")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);
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
                    store.save_run(&mut run).await?;
                    store.save_worker_run(&mut wrun).await?;
                }
            }
        }

        if run.status.is_terminal() {
            if let Some(ref mut l) = lease {
                let _ = l.release();
            }
            self.dispatcher.release_claim(&claim_id).await?;
        }

        let handle = RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
            completed_at: run.ended_at,
            output: output_text,
        };

        if let Some(store)=&self.run_store {store.save_run(&mut run).await?;}

        // 5. Evict terminal executions from active maps if durably persisted, otherwise retain
        if run.status.is_terminal() && self.run_store.is_some() {
            self.active_runs.write().await.remove(&run.id);
            self.active_workers.write().await.remove(&wrun.id);
        } else {
            self.active_runs.write().await.insert(run.id.clone(), run.clone());
            self.active_workers
                .write()
                .await
                .insert(wrun.id.clone(), wrun);
        }
        if background { self.persist_live_journal(&cmd, &mut run, handle.output.as_deref().unwrap_or("")).await?; }
        let result_err = if handle.status == RunStatus::Suspended {
            run.metadata.get("pending_approval_reason").and_then(|v| v.as_str()).map(str::to_string)
        } else if handle.status == RunStatus::Failed {
            run.metadata.get("failure_reason").and_then(|v| v.as_str()).map(str::to_string).or_else(|| Some("Native process failed; see the captured output".into()))
        } else {
            None
        };
        self.live.emit(&handle.run_id, "result", serde_json::json!({"status":handle.status.to_string(),"text":handle.output,"error":result_err}));
        Ok(handle)
    }

    pub(super) async fn continue_resumed_run(&self, run_id: &str, task_id: &str) {
        let mut run = match self.active_runs.read().await.get(run_id).cloned() {
            Some(r) => r,
            None => {
                if let Some(ref store) = self.run_store {
                    match store.get_run(run_id).await {
                        Ok(Some(r)) => r,
                        _ => return,
                    }
                } else {
                    return;
                }
            }
        };

        let attempt_id = run
            .metadata
            .get("pending_approval_attempt_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let mut outbox_receipt = None;
        if let (Some(ref outbox), Some(ref att_id)) = (&self.outbox, &attempt_id) {
            if let Ok(Some(entry)) = outbox.get(att_id).await {
                if let Some(r) = entry.receipt {
                    outbox_receipt = Some(r);
                } else if entry.status == OutboxStatus::Approved || entry.status == OutboxStatus::Dispatching {
                    let rcpt = custos_domain::ExecutionReceipt {
                        receipt_id: new_id("rcpt"),
                        permit_id: entry.permit_id.clone(),
                        action_id: entry.action_id.clone(),
                        status: custos_domain::ReceiptStatus::Success,
                        output_digest: format!(
                            "sha256:{}",
                            custos_domain::digest(entry.payload_json.as_deref().unwrap_or("").as_bytes())
                        ),
                        output_data: entry.payload_json.as_deref().and_then(|p| serde_json::from_str(p).ok()),
                        error_message: None,
                        duration_ms: Some(10),
                        executed_at: chrono::Utc::now(),
                        assurance: custos_domain::Assurance::CustosMediated,
                    };
                    let _ = outbox.mark_receipted(att_id, rcpt.clone()).await;
                    outbox_receipt = Some(rcpt);
                }
            }
        }

        if let Some(receipt) = outbox_receipt {
            run.metadata["approved_action_receipt"] = serde_json::to_value(&receipt).unwrap_or_default();
            run.metadata["completion_reason"] = serde_json::json!("approval_resumed_and_executed");
            let _ = run.transition(RunStatus::Completed);
            if let Some(ref kernel) = self.kernel {
                let _ = kernel.transition_task(task_id, TaskStatus::Succeeded).await;
            }
            if let Some(ref store) = self.run_store {
                let _ = store.save_run(&mut run).await;
                if let Ok(workers) = store.list_worker_runs_for_run(run_id).await {
                    for mut wrun in workers.into_iter().filter(|w| w.status == RunStatus::Active) {
                        let _ = wrun.transition(RunStatus::Completed);
                        let _ = store.save_worker_run(&mut wrun).await;
                        self.active_workers.write().await.remove(&wrun.id);
                    }
                }
            }
            self.active_runs.write().await.remove(run_id);
            let out_text = run
                .metadata
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("Action approved and completed successfully");
            self.live.emit(
                run_id,
                "result",
                serde_json::json!({
                    "status": "completed",
                    "text": out_text,
                    "receipt": receipt,
                }),
            );
            self.live.emit(run_id, "status", serde_json::json!({ "status": "completed" }));
        }
    }

    /// Executes an Orca multi-candidate race across isolated model candidates and optional Test Oracle.
    pub async fn execute_candidate_race(
        &self,
        spec: custos_domain::CandidateRaceSpec,
    ) -> Result<custos_domain::CandidateRaceResult, DomainError> {
        let race_id = spec.id.clone();
        let task_id = spec.task_id.clone();
        let candidates_count = spec.candidates.len();

        if candidates_count == 0 {
            return Err(DomainError::Validation(
                "Candidate race requires at least one candidate".into(),
            ));
        }
        if candidates_count > 4 {
            return Err(DomainError::Validation(format!(
                "Candidate race concurrency capped at 4 candidates, got {candidates_count}"
            )));
        }

        let mut futures = Vec::new();

        for candidate in spec.candidates {
            let runtime = self.clone();
            let task_id = task_id.clone();
            let base_workspace_id = spec.base_workspace_id.clone();
            let test_command = spec.test_command.clone();

            futures.push(async move {
                let start_time = std::time::Instant::now();
                let node_id = format!("candidate:{}", candidate.candidate_id);

                // 1. Resolve candidate workspace path if workspace_coordinator is available
                let target_ws_id = candidate
                    .workspace_id
                    .as_deref()
                    .unwrap_or(&base_workspace_id);
                let (worktree_path, workspace_id) = if let Some(ref ws_coord) = runtime.workspace_coordinator {
                    let ws_id = custos_domain::WorkspaceId::new(target_ws_id);
                    if let Ok(Some(ws)) = ws_coord.get_workspace(&ws_id).await {
                        (ws.path.clone(), ws.id.to_string())
                    } else {
                        (".".to_string(), target_ws_id.to_string())
                    }
                } else {
                    (".".to_string(), target_ws_id.to_string())
                };

                // 2. Dispatch the candidate run with scoped node_id
                let cmd = custos_domain::StartRunCommand::new(&task_id, &candidate.candidate_id)
                    .with_node_id(&node_id)
                    .with_model(&candidate.model_id)
                    .with_harness_id(&candidate.harness_id)
                    .with_workspace_id(&workspace_id);

                let run_res = runtime.dispatch_run(cmd, false).await;

                // 3. Optional Test Oracle verification
                let mut test_passed = run_res.is_ok();
                let mut tests_succeeded = if test_passed { 1 } else { 0 };
                let mut tests_failed = if test_passed { 0 } else { 1 };
                let mut test_output = String::new();

                if let Some(ref test_cmd) = test_command {
                    let test_timeout = std::time::Duration::from_secs(spec.timeout_seconds.max(5));
                    let exec_fut = async {
                        if let Ok(harness) = runtime.resolve_harness(Some(&candidate.harness_id)) {
                            harness
                                .run_native(test_cmd, std::path::Path::new(&worktree_path))
                                .await
                        } else {
                            let mut cmd = tokio::process::Command::new("sh");
                            cmd.arg("-c").arg(test_cmd).current_dir(&worktree_path);
                            match cmd.output().await {
                                Ok(out) => Ok(custos_core::contracts::harness::HarnessExecutionResult {
                                    success: out.status.success(),
                                    exit_code: out.status.code(),
                                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                                    observed_effects: Vec::new(),
                                    mediation_level: custos_core::contracts::harness::ToolMediationLevel::ObserveOnly,
                                }),
                                Err(e) => Err(DomainError::Validation(format!("Test execution failed: {e}"))),
                            }
                        }
                    };

                    match tokio::time::timeout(test_timeout, exec_fut).await {
                        Ok(Ok(exec_result)) => {
                            test_passed = exec_result.success;
                            test_output = exec_result.stdout + "\n" + &exec_result.stderr;
                            let (p, f) = parse_test_counts(&test_output);
                            if p > 0 || f > 0 {
                                tests_succeeded = p;
                                tests_failed = f;
                            } else if test_passed {
                                tests_succeeded = 1;
                                tests_failed = 0;
                            } else {
                                tests_succeeded = 0;
                                tests_failed = 1;
                            }
                        }
                        Ok(Err(e)) => {
                            test_passed = false;
                            tests_failed = 1;
                            test_output = format!("Failed to run test oracle: {e}");
                        }
                        Err(_) => {
                            test_passed = false;
                            tests_failed = 1;
                            test_output = format!("Test oracle timed out after {}s", spec.timeout_seconds);
                        }
                    }
                }

                // 4. Collect modified files from workspace if workspace_coordinator is available
                let mut modified_files = Vec::new();
                if let Some(ref ws_coord) = runtime.workspace_coordinator {
                    let ws_id = custos_domain::WorkspaceId::new(&workspace_id);
                    if let Ok(dirty) = ws_coord.inspect_dirty(&ws_id).await {
                        modified_files = dirty.modified_files;
                    }
                }

                custos_domain::CandidateExecutionReport {
                    candidate_id: candidate.candidate_id,
                    model_id: candidate.model_id,
                    workspace_id,
                    worktree_path,
                    test_passed,
                    tests_succeeded,
                    tests_failed,
                    test_output: test_output.trim().to_string(),
                    modified_files,
                    total_tokens: 0,
                    execution_duration_ms: start_time.elapsed().as_millis() as u64,
                }
            });
        }

        let candidate_reports = futures::future::join_all(futures).await;

        // 5. Select winner: candidate with test_passed == true, highest test count, and lowest execution duration
        let winner_candidate_id = candidate_reports
            .iter()
            .filter(|r| r.test_passed)
            .max_by(|a, b| {
                a.tests_succeeded
                    .cmp(&b.tests_succeeded)
                    .then_with(|| b.execution_duration_ms.cmp(&a.execution_duration_ms))
            })
            .map(|r| r.candidate_id.clone());

        Ok(custos_domain::CandidateRaceResult {
            race_id,
            task_id,
            winner_candidate_id,
            candidate_reports,
            completed_at: chrono::Utc::now(),
        })
    }
}

fn parse_test_counts(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("passed") || lower.contains("failed") {
            let tokens: Vec<&str> = lower.split_whitespace().collect();
            for i in 0..tokens.len() {
                if tokens[i].starts_with("passed") && i > 0 {
                    if let Ok(n) = tokens[i - 1].trim_matches(|c: char| !c.is_ascii_digit()).parse::<u32>() {
                        passed = passed.max(n);
                    }
                }
                if tokens[i].starts_with("failed") && i > 0 {
                    if let Ok(n) = tokens[i - 1].trim_matches(|c: char| !c.is_ascii_digit()).parse::<u32>() {
                        failed = failed.max(n);
                    }
                }
            }
        }
    }
    (passed, failed)
}

