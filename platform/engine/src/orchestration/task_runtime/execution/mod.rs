mod candidate_race;
mod model;
mod native;
mod resume;
mod runner;

use super::*;
use custos_domain::{new_id, RunHandle, StartRunCommand};

impl TaskRuntime {
    pub(super) async fn dispatch_run(
        &self,
        cmd: StartRunCommand,
        background: bool,
    ) -> Result<RunHandle, DomainError> {
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

        if background && !matches!(actual_mode, "model" | "native") {
            return Err(DomainError::Validation(
                "No executable provider is configured for this background mode".into(),
            ));
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

            provider_account_id = task
                .metadata
                .get("provider_account_id")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
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
        if let Some(account) = &cmd.provider_account_id {
            run.metadata["provider_account_id"] = serde_json::json!(account);
        }
        run.metadata["session_id"] = serde_json::json!(cmd.session_id);
        run.metadata["turn_id"] =
            serde_json::json!(cmd.turn_id.clone().unwrap_or_else(|| new_id("turn")));
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
            Ok(token) => token,
            Err(error) => {
                if let Some(mut lease) = lease {
                    if let Err(cleanup) = lease.release() {
                        tracing::warn!(run_id=%run.id,error=%cleanup,"Lease cleanup failed after live admission error");
                    }
                }
                self.dispatcher.release_claim(&claim.id).await?;
                run.transition(RunStatus::Failed)?;
                wrun.transition(RunStatus::Failed)?;
                run.metadata["failure_reason"] = serde_json::json!(error.to_string());
                if let Some(store) = &self.run_store {
                    store.save_run(&mut run).await?;
                    store.save_worker_run(&mut wrun).await?;
                }
                return Err(error);
            }
        };
        self.active_runs
            .write()
            .await
            .insert(run.id.clone(), run.clone());
        self.active_workers
            .write()
            .await
            .insert(wrun.id.clone(), wrun.clone());
        let handle = RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
            completed_at: None,
            output: None,
        };
        self.live
            .emit(&run.id, "status", serde_json::json!({"status":"queued"}));
        let actual_mode = actual_mode.to_string();
        let requested_mode = requested_mode.to_string();
        if background {
            let runtime = self.clone();
            let live_run_id = run.id.clone();
            let live_session = cmd.session_id.clone().unwrap_or_default();
            tokio::spawn(async move {
                if let Err(error) = runtime
                    .execute_registered(
                        cmd,
                        run,
                        wrun,
                        claim.id,
                        lease,
                        actual_mode,
                        requested_mode,
                        token,
                        true,
                    )
                    .await
                {
                    tracing::warn!(run_id = %live_run_id, error = %error, "Background run failed");
                    let partial = runtime
                        .live
                        .replay(&live_session, &live_run_id, 0)
                        .ok()
                        .map(|r| r.snapshot.text)
                        .unwrap_or_default();
                    runtime.live.emit(
                        &live_run_id,
                        "result",
                        serde_json::json!({
                            "status": "failed",
                            "text": partial,
                            "error": error.to_string()
                        }),
                    );
                }
            });
            Ok(handle)
        } else {
            self.execute_registered(
                cmd,
                run,
                wrun,
                claim.id,
                lease,
                actual_mode,
                requested_mode,
                token,
                false,
            )
            .await
        }
    }
}
