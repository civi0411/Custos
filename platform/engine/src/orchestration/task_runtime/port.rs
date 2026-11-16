use async_trait::async_trait;
use chrono::Utc;
use custos_core::contracts::storage::OutboxStatus;
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    CancelReceipt, ContinuationPacket, DomainError, RunHandle, RunStatus, StartRunCommand,
    TaskStatus,
};

use super::TaskRuntime;

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        self.dispatch_run(cmd, false).await
    }

    async fn start_run_background(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        self.dispatch_run(cmd, true).await
    }

    async fn live_run(
        &self,
        session_id: &str,
        run_id: &str,
        cursor: u64,
    ) -> Result<custos_domain::LiveRunReplay, DomainError> {
        match self.live.replay(session_id, run_id, cursor) {
            Ok(replay) => Ok(replay),
            Err(DomainError::NotFound { .. }) => {
                let run = self
                    .run_store
                    .as_ref()
                    .ok_or_else(|| DomainError::Validation("Run storage unavailable".into()))?
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.into(),
                    })?;
                if run.metadata.get("session_id").and_then(|v| v.as_str()) != Some(session_id) {
                    return Err(DomainError::Validation(
                        "Run does not belong to this session".into(),
                    ));
                }
                let (status_str, err_msg) = if run.status == RunStatus::Suspended {
                    let reason = run
                        .metadata
                        .get("pending_approval_reason")
                        .or_else(|| run.metadata.get("suspension_reason"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    ("suspended".to_string(), reason)
                } else if !run.status.is_terminal()
                    || (run.status == RunStatus::Failed
                        && run.metadata.get("completion_reason").is_none()
                        && run.metadata.get("failure_reason").is_none())
                {
                    (
                        "interrupted".to_string(),
                        Some(
                            "Run interrupted by daemon restart; send a new prompt to continue"
                                .to_string(),
                        ),
                    )
                } else {
                    (
                        run.status.to_string(),
                        run.metadata
                            .get("failure_reason")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                    )
                };
                Ok(custos_domain::LiveRunReplay {
                    events: vec![],
                    gap: true,
                    snapshot: custos_domain::LiveRunSnapshot {
                        cursor: 0,
                        run_id: run.id,
                        task_id: run.task_id,
                        session_id: session_id.into(),
                        turn_id: run
                            .metadata
                            .get("turn_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .into(),
                        status: status_str,
                        text: run
                            .metadata
                            .get("output")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .into(),
                        activities: vec![],
                        usage: run.metadata.get("usage").cloned(),
                        error: err_msg,
                    },
                })
            }
            Err(error) => Err(error),
        }
    }

    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError> {
        if self.live.cancel(run_id) {
            let task_id = self
                .active_runs
                .read()
                .await
                .get(run_id)
                .map(|run| run.task_id.clone())
                .unwrap_or_default();
            let harness_id = self.active_runs.read().await.get(run_id).and_then(|r| {
                r.metadata
                    .get("harness_id")
                    .and_then(|h| h.as_str().map(str::to_string))
            });
            if let Some(ref hid) = harness_id {
                if let Ok(harness) = self.resolve_harness(Some(hid)) {
                    let _ = harness.cancel_run(run_id).await;
                }
            }
            tracing::info!(run_id = run_id, task_id = %task_id, "Live run cancellation requested");
            let uncertain_effects_count = if let Some(outbox) = &self.outbox {
                let uncertain = outbox.list_by_status(OutboxStatus::Uncertain).await?;
                let dispatching = outbox.list_by_status(OutboxStatus::Dispatching).await?;
                u32::try_from(uncertain.len().saturating_add(dispatching.len()))
                    .unwrap_or(u32::MAX)
            } else {
                0
            };
            return Ok(CancelReceipt {
                run_id: run_id.into(),
                cancelled_at: Utc::now(),
                reason: reason.into(),
                uncertain_effects_count,
            });
        }
        let mut run = {
            let runs = self.active_runs.read().await;
            if let Some(r) = runs.get(run_id) {
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let r = store
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.to_string(),
                    })?;
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        run.transition(RunStatus::Cancelled)?;
        if let Some(ref store) = self.run_store {
            store.save_run(&mut run).await?;
            for mut w in store
                .list_worker_runs_for_run(run_id)
                .await?
                .into_iter()
                .filter(|w| w.status == RunStatus::Active)
            {
                w.transition(RunStatus::Cancelled)?;
                store.save_worker_run(&mut w).await?;
                self.active_workers.write().await.remove(&w.id);
            }
        }
        self.active_runs.write().await.remove(run_id);

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
        let mut run = {
            let runs = self.active_runs.read().await;
            if let Some(r) = runs.get(run_id) {
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let r = store
                    .get_run(run_id)
                    .await?
                    .ok_or_else(|| DomainError::NotFound {
                        kind: "Run".into(),
                        id: run_id.to_string(),
                    })?;
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        if run.status == RunStatus::Suspended {
            // Revalidate outbox approval if an attempt was recorded
            if let Some(attempt_id) = run
                .metadata
                .get("pending_approval_attempt_id")
                .and_then(|v| v.as_str())
            {
                if let Some(ref outbox) = self.outbox {
                    if let Ok(Some(entry)) = outbox.get(attempt_id).await {
                        match entry.status {
                            OutboxStatus::Failed => {
                                run.transition(RunStatus::Failed)?;
                                run.metadata["failure_reason"] = serde_json::json!(
                                    "Approval rejected by user or outbox execution failed"
                                );
                                if let Some(ref kernel) = self.kernel {
                                    let _ = kernel
                                        .transition_task(&run.task_id, TaskStatus::Failed)
                                        .await;
                                }
                                if let Some(ref store) = self.run_store {
                                    store.save_run(&mut run).await?;
                                }
                                self.live.emit(
                                    &run.id,
                                    "result",
                                    serde_json::json!({
                                        "status": "failed",
                                        "text": run.metadata.get("output").and_then(|v| v.as_str()).unwrap_or(""),
                                        "error": "Approval rejected by user or outbox execution failed"
                                    }),
                                );
                                return Ok(RunHandle {
                                    run_id: run.id.clone(),
                                    task_id: run.task_id.clone(),
                                    status: run.status,
                                    started_at: run.started_at,
                                    completed_at: run.ended_at,
                                    output: run
                                        .metadata
                                        .get("output")
                                        .and_then(|v| v.as_str().map(str::to_string)),
                                });
                            }
                            OutboxStatus::Pending => {
                                return Err(DomainError::Conflict(
                                    "Cannot resume run: approval is still pending".into(),
                                ));
                            }
                            OutboxStatus::Approved
                            | OutboxStatus::Dispatching
                            | OutboxStatus::Receipted
                            | OutboxStatus::Uncertain => {
                                // Approval recorded; proceed to reactivate run
                            }
                        }
                    }
                }
            }

            run.transition(RunStatus::Active)?;
            if let Some(obj) = run.metadata.as_object_mut() {
                obj.remove("pending_approval_reason");
            }
            run.metadata["resumed_at"] = serde_json::json!(Utc::now().to_rfc3339());
            if let Some(ref kernel) = self.kernel {
                let _ = kernel
                    .transition_task(&run.task_id, TaskStatus::Running)
                    .await;
            }
            if let Some(ref store) = self.run_store {
                store.save_run(&mut run).await?;
                if let Ok(workers) = store.list_worker_runs_for_run(run_id).await {
                    for mut wrun in workers
                        .into_iter()
                        .filter(|w| w.status == RunStatus::Suspended)
                    {
                        wrun.transition(RunStatus::Active)?;
                        let _ = store.save_worker_run(&mut wrun).await;
                        self.active_workers
                            .write()
                            .await
                            .insert(wrun.id.clone(), wrun);
                    }
                }
            }
            self.active_runs
                .write()
                .await
                .insert(run.id.clone(), run.clone());
            self.live
                .emit(&run.id, "status", serde_json::json!({ "status": "active" }));

            let runtime = self.clone();
            let resumed_run_id = run.id.clone();
            let resumed_task_id = run.task_id.clone();
            tokio::spawn(async move {
                runtime
                    .continue_resumed_run(&resumed_run_id, &resumed_task_id)
                    .await;
            });
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

    async fn execute_candidate_race(
        &self,
        spec: custos_domain::CandidateRaceSpec,
    ) -> Result<custos_domain::CandidateRaceResult, DomainError> {
        self.execute_candidate_race(spec).await
    }
}

