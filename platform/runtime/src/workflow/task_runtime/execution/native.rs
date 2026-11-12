use super::super::*;
use chrono::Utc;
use custos_core::contracts::harness::RunObserver;
use custos_core::contracts::storage::{OutboxEntry, OutboxStatus};
use custos_domain::{new_id, Assurance, ContextPack, StartRunCommand};

impl TaskRuntime {
    pub(super) async fn execute_native_turn(
        &self,
        cmd: &StartRunCommand,
        run: &mut Run,
        wrun: &mut WorkerRun,
        lease: &Option<crate::workspace::lease::WorkspaceLease>,
        requested_mode: &str,
        observer: &Arc<dyn RunObserver>,
    ) -> Result<Option<String>, DomainError> {
        let mut turn_output: Option<String> = None;
        let harness = self.resolve_harness(cmd.harness_id.as_deref())?;
        let profile = harness.profile();
        let harness_id = profile.harness_id.clone();

        let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, requested_mode)?;
        if let Some(ref store) = self.run_store {
            store.save_launch_attempt(&launch).await?;
        }
        run.metadata["launch_attempt_id"] = serde_json::json!(launch.id);
        run.metadata["harness_id"] = serde_json::json!(harness_id);
        run.metadata["tool_mediation"] = serde_json::json!(profile.tool_mediation.to_string());
        wrun.continuation_packet_ref = Some(format!("launch:{}", launch.id));

        // Resolve execution workspace & effective directory
        let (base_workspace_dir, maybe_resolved_ws_id) =
            if let Some(ref ws_coord) = self.workspace_coordinator {
                let ws_id_str = cmd.workspace_id.as_deref().ok_or_else(|| {
                    DomainError::Validation("Native run requires a registered workspace_id".into())
                })?;
                let ws_id = custos_domain::WorkspaceId::new(ws_id_str);
                let ws = ws_coord.get_workspace(&ws_id).await?.ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Native workspace '{ws_id_str}' no longer exists"
                    ))
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
        run.metadata["effective_workspace_path"] =
            serde_json::json!(effective_dir.display().to_string());

        // Capture baseline commit before execution if workspace coordinator is available
        if let (Some(ref ws_coord), Some(ref ws_id)) =
            (&self.workspace_coordinator, &maybe_resolved_ws_id)
        {
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
                    let rel_strs: Vec<String> =
                        files.iter().map(|f| f.display().to_string()).collect();
                    run.metadata["isolated_modified_files"] = serde_json::json!(rel_strs);
                }
                if let Some(ref mgr) = self.lease_manager {
                    if let Ok(diff_str) = mgr.diff_lease_head(l) {
                        if !diff_str.is_empty() {
                            run.metadata["isolated_worktree_diff"] = serde_json::json!(diff_str);
                        }
                    }
                }
            } else if let (Some(ref ws_coord), Some(ref ws_id)) =
                (&self.workspace_coordinator, &maybe_resolved_ws_id)
            {
                if let Ok(dirty) = ws_coord.inspect_dirty(ws_id).await {
                    run.metadata["dirty_manifest"] =
                        serde_json::to_value(&dirty).unwrap_or_default();
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
            let context_pack = ContextPack::new(new_id("ctx"), Vec::new(), 0, "sha256:empty".into());
            harness.execute_turn(wrun, &context_pack).await?
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
                                    idempotency_key: Some(
                                        intent
                                            .idempotency_key
                                            .clone()
                                            .unwrap_or_else(|| intent.id.clone()),
                                    ),
                                    status: OutboxStatus::Pending,
                                    created_at: Utc::now(),
                                    receipt: None,
                                    recipient: None,
                                    payload_json: Some(intent.parameters.to_string()),
                                    error_message: Some(reason.clone()),
                                };
                                outbox.enqueue(outbox_entry).await?;
                            }
                            run.metadata["pending_approval_attempt_id"] =
                                serde_json::json!(attempt_id);
                            run.metadata["pending_approval_reason"] = serde_json::json!(reason);
                            run.transition(RunStatus::Suspended)?;
                            wrun.transition(RunStatus::Suspended)?;
                            if let Some(ref kernel) = self.kernel {
                                let _ = kernel
                                    .transition_task(&cmd.task_id, TaskStatus::Blocked)
                                    .await;
                            }
                            self.live.emit(
                                &run.id,
                                "status",
                                serde_json::json!({
                                    "status": "suspended",
                                    "reason": reason,
                                    "attempt_id": attempt_id,
                                }),
                            );
                            if let Some(ref store) = self.run_store {
                                store.save_run(run).await?;
                                store.save_worker_run(wrun).await?;
                            }
                            break;
                        }
                        Err(e) => return Err(e),
                    };
                    let burned_permit = kernel.consume_permit(&permit.id, &intent).await?;

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
                                        let _ = ledger
                                            .update_effect_status(
                                                &eff_id,
                                                custos_domain::EffectStatus::Uncertain,
                                                None,
                                            )
                                            .await;
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

        Ok(turn_output)
    }
}
