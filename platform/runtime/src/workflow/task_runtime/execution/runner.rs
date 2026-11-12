use super::*;
use custos_core::contracts::storage::OutboxStatus;
use custos_domain::{RunHandle, SessionId, StartRunCommand};

impl TaskRuntime {
    async fn persist_live_journal(
        &self,
        cmd: &StartRunCommand,
        run: &mut Run,
        text: &str,
    ) -> Result<(), DomainError> {
        if let (Some(session), Some(manager)) = (&cmd.session_id, &self.journal_manager) {
            let message = if text.is_empty() {
                format!("Run {} {}", run.id, run.status)
            } else {
                text.to_string()
            };
            let session_id = SessionId::new(session.clone());
            let entry = manager
                .append_assistant_entry(&session_id, &message)
                .await?;
            run.metadata["journal_occurred_at"] = serde_json::json!(entry.occurred_at);
            if let Some(store) = &self.run_store {
                store.save_run(run).await?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_registered(
        &self,
        cmd: StartRunCommand,
        mut run: Run,
        mut wrun: WorkerRun,
        claim_id: String,
        mut lease: Option<crate::workspace::lease::WorkspaceLease>,
        actual_mode: String,
        requested_mode: String,
        token: tokio_util::sync::CancellationToken,
        background: bool,
    ) -> Result<RunHandle, DomainError> {
        tracing::info!(run_id = %run.id, task_id = %run.task_id, "Executing registered run");
        let wb_id = run
            .metadata
            .get("workbench_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let run_started_ms = run.started_at.timestamp_millis();
        let root_span_id = custos_domain::new_id("step");
        let root_kind = if actual_mode == "native" {
            custos_domain::supervision::StepSpanKind::Harness
        } else {
            custos_domain::supervision::StepSpanKind::Llm
        };
        let mut root_span = custos_domain::supervision::RunStepSpan {
            id: root_span_id.clone(),
            run_id: run.id.clone(),
            worker_run_id: Some(wrun.id.clone()),
            parent_id: None,
            kind: root_kind,
            name: format!("{actual_mode}_turn"),
            status: custos_domain::supervision::StepSpanStatus::Running,
            started_at_ms: run_started_ms,
            ended_at_ms: None,
            duration_ms: None,
            metadata: serde_json::json!({ "task_id": run.task_id, "mode": actual_mode }),
        };
        self.supervision.notify_run_state(
            &run.id,
            wb_id.as_deref(),
            &cmd.actor,
            true,
            run_started_ms,
            None,
        );
        self.supervision.record_span(root_span.clone());
        self.live
            .emit(&run.id, "status", serde_json::json!({"status":"running"}));
        let observer = self.live.observer(&run.id, token.clone());

        // Execute governed agent turn based on actual_mode
        let turn_result = {
            let turn_result = async {
                if actual_mode == "native" {
                    self.execute_native_turn(
                        &cmd,
                        &mut run,
                        &mut wrun,
                        &lease,
                        &requested_mode,
                        &observer,
                    )
                    .await
                } else if actual_mode == "model" {
                    self.execute_model_turn(
                        &cmd,
                        &mut run,
                        &mut wrun,
                        &requested_mode,
                        &observer,
                    )
                    .await
                } else {
                    Ok(None)
                }
            };
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

        if let Ok(replay) = self
            .live
            .replay(cmd.session_id.as_deref().unwrap_or(""), &run.id, 0)
        {
            if let Some(usage) = replay.snapshot.usage {
                run.metadata["usage"] = usage;
            }
        }
        let turn_result = if token.is_cancelled() {
            Err(DomainError::Validation("Run cancelled".into()))
        } else {
            turn_result
        };
        let output_text = match turn_result {
            Ok(out) => out,
            Err(e) => {
                if let Some(ref mut l) = lease {
                    let _ = l.release();
                }
                let terminal = if token.is_cancelled() {
                    RunStatus::Cancelled
                } else {
                    RunStatus::Failed
                };
                if token.is_cancelled() {
                    if let Some(kernel) = &self.kernel {
                        kernel
                            .transition_task(&run.task_id, TaskStatus::Cancelled)
                            .await?;
                    }
                }
                let _ = run.transition(terminal);
                let _ = wrun.transition(terminal);
                run.metadata["failure_reason"] = serde_json::json!(e.to_string());
                if let Some(ref store) = self.run_store {
                    let _ = store.save_run(&mut run).await;
                    let _ = store.save_worker_run(&mut wrun).await;
                }
                let _ = self.dispatcher.release_claim(&claim_id).await;
                let partial = self
                    .live
                    .replay(cmd.session_id.as_deref().unwrap_or(""), &run.id, 0)
                    .ok()
                    .map(|r| r.snapshot.text)
                    .unwrap_or_default();
                run.metadata["output"] = serde_json::json!(partial);
                if let Some(store) = &self.run_store {
                    store.save_run(&mut run).await?;
                }
                if self.run_store.is_some() {
                    self.active_runs.write().await.remove(&run.id);
                    self.active_workers.write().await.remove(&wrun.id);
                } else {
                    self.active_runs
                        .write()
                        .await
                        .insert(run.id.clone(), run.clone());
                    self.active_workers
                        .write()
                        .await
                        .insert(wrun.id.clone(), wrun.clone());
                }
                if background {
                    self.persist_live_journal(&cmd, &mut run, &partial).await?;
                }
                self.live.emit(
                    &run.id,
                    "result",
                    serde_json::json!({
                        "status": run.status.to_string(),
                        "text": partial,
                        "error": e.to_string()
                    }),
                );
                let err_ms = chrono::Utc::now().timestamp_millis();
                root_span.finish(err_ms, custos_domain::supervision::StepSpanStatus::Error);
                self.supervision.record_span(root_span.clone());
                let err_msg = e.to_string();
                let (_, sig_hash) =
                    custos_domain::supervision::compute_error_signature("ExecutionError", &err_msg);
                self.supervision.record_error(custos_domain::supervision::StepError {
                    id: custos_domain::new_id("serr"),
                    run_id: run.id.clone(),
                    worker_run_id: Some(wrun.id.clone()),
                    span_id: Some(root_span_id),
                    error_type: "ExecutionError".into(),
                    message: err_msg,
                    stack: None,
                    agent: cmd.actor.clone(),
                    signature_hash: sig_hash,
                    timestamp_ms: err_ms,
                });
                self.supervision.notify_run_state(
                    &run.id,
                    wb_id.as_deref(),
                    &cmd.actor,
                    false,
                    run_started_ms,
                    Some(err_ms),
                );
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
                run.metadata["verification_status"] =
                    serde_json::json!("unverified_external_effects_require_evidence");

                if let Some(ref outbox) = self.outbox {
                    if let Ok(uncertain) = outbox.list_by_status(OutboxStatus::Uncertain).await {
                        run.metadata["uncertain_effects_count"] =
                            serde_json::json!(uncertain.len());
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

        if let Some(store) = &self.run_store {
            store.save_run(&mut run).await?;
        }

        // 5. Evict terminal executions from active maps if durably persisted, otherwise retain
        if run.status.is_terminal() && self.run_store.is_some() {
            self.active_runs.write().await.remove(&run.id);
            self.active_workers.write().await.remove(&wrun.id);
        } else {
            self.active_runs
                .write()
                .await
                .insert(run.id.clone(), run.clone());
            self.active_workers
                .write()
                .await
                .insert(wrun.id.clone(), wrun);
        }
        if background {
            self.persist_live_journal(&cmd, &mut run, handle.output.as_deref().unwrap_or(""))
                .await?;
        }
        let result_err = if handle.status == RunStatus::Suspended {
            run.metadata
                .get("pending_approval_reason")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        } else if handle.status == RunStatus::Failed {
            run.metadata
                .get("failure_reason")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| Some("Native process failed; see the captured output".into()))
        } else {
            None
        };
        self.live.emit(
            &handle.run_id,
            "result",
            serde_json::json!({
                "status": handle.status.to_string(),
                "text": handle.output,
                "error": result_err
            }),
        );
        let done_ms = chrono::Utc::now().timestamp_millis();
        let done_status = if handle.status == RunStatus::Failed {
            custos_domain::supervision::StepSpanStatus::Error
        } else {
            custos_domain::supervision::StepSpanStatus::Success
        };
        root_span.finish(done_ms, done_status);
        self.supervision.record_span(root_span);
        if handle.status.is_terminal() {
            self.supervision.notify_run_state(
                &handle.run_id,
                wb_id.as_deref(),
                &cmd.actor,
                false,
                run_started_ms,
                Some(done_ms),
            );
        }
        Ok(handle)
    }
}
