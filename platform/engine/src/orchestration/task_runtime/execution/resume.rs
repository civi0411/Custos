use super::super::*;

impl TaskRuntime {
    pub(crate) async fn continue_resumed_run(&self, run_id: &str, task_id: &str) {
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
                } else {
                    // F11 fix: Do not synthesize a fake Success receipt!
                    // If an action is Approved, Dispatching, or Uncertain but has no real receipt,
                    // we must not transition the Run to Completed or Task to Succeeded.
                    run.metadata["outbox_status"] =
                        serde_json::to_value(entry.status).unwrap_or_default();
                    run.metadata["completion_reason"] =
                        serde_json::json!("awaiting_execution_receipt");
                    if let Some(ref store) = self.run_store {
                        let _ = store.save_run(&mut run).await;
                    }
                    self.live.emit(
                        run_id,
                        "status",
                        serde_json::json!({
                            "status": "awaiting_receipt",
                            "outbox_status": format!("{:?}", entry.status),
                        }),
                    );
                    return;
                }
            }
        }

        if let Some(receipt) = outbox_receipt {
            run.metadata["approved_action_receipt"] =
                serde_json::to_value(&receipt).unwrap_or_default();
            run.metadata["completion_reason"] = serde_json::json!("approval_resumed_and_executed");
            let _ = run.transition(RunStatus::Completed);
            if let Some(ref store) = self.run_store {
                // Receipt persistence failure must not emit completed
                if let Err(e) = store.save_run(&mut run).await {
                    eprintln!("[Runtime] Failed to persist completed run: {}", e);
                    return;
                }
                if let Ok(workers) = store.list_worker_runs_for_run(run_id).await {
                    for mut wrun in workers
                        .into_iter()
                        .filter(|w| w.status == RunStatus::Active)
                    {
                        let _ = wrun.transition(RunStatus::Completed);
                        let _ = store.save_worker_run(&mut wrun).await;
                        self.active_workers.write().await.remove(&wrun.id);
                    }
                }
            }
            if let Some(ref kernel) = self.kernel {
                let _ = kernel.transition_task(task_id, TaskStatus::Succeeded).await;
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
            self.live.emit(
                run_id,
                "status",
                serde_json::json!({ "status": "completed" }),
            );
        }
    }
}
