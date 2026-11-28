//! Background Sweepers and Jobs (SADE R8 §6.8)

use custos_storage::SqliteTaskStore;
use std::sync::Arc;

/// Spawns startup credential initialization, the 5-minute OAuth sweeper, and the 60-second uncertain runs reconciler.
pub fn spawn_background_sweepers(
    credentials: Arc<crate::credentials::ProviderCredentials>,
    store: Arc<SqliteTaskStore>,
) {
    let startup_credentials = credentials.clone();
    tokio::spawn(async move {
        if startup_credentials.initialize().await.is_err() {
            tracing::warn!(
                "Provider secure storage needs attention; affected accounts remain blocked"
            );
        }
    });

    // Background OAuth Token Sweeper Job: runs every 5 minutes and refreshes expiring tokens
    let sweeper_credentials = credentials;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            let now_ms = chrono::Utc::now().timestamp_millis();
            // Check tokens expiring within the next 10 minutes (600 seconds)
            let threshold_ms = now_ms + (600 * 1000);
            if let Ok(expiring) = sweeper_credentials
                .list_expiring_oauth_tokens(threshold_ms)
                .await
            {
                for tok in expiring {
                    let settings = match custos_integrations::connectors::oauth::oauth_settings(
                        &tok.service_type,
                    ) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let manager = custos_integrations::connectors::oauth::OAuthPkceManager::new(
                        &settings.0,
                        &settings.1,
                    );
                    tracing::info!(provider_id = %tok.provider_id, "Sweeper job refreshing expiring OAuth token");
                    if let Ok(refreshed) = manager.refresh_token(&settings.2, &tok).await {
                        match sweeper_credentials.save_refreshed(&tok, &refreshed).await {
                            Ok(()) => {
                                tracing::info!(provider_id = %tok.provider_id, "Sweeper saved refreshed credential")
                            }
                            Err(_) => {
                                tracing::warn!(provider_id = %tok.provider_id, "Sweeper could not save refreshed credential")
                            }
                        }
                    }
                }
            }
        }
    });

    // Background Uncertain Runs Reconciler (Phase 1 / F04): polls uncertain runs every 60s
    let uncertain_store = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(uncertain_runs) = uncertain_store.list_uncertain_runs() {
                let now = chrono::Utc::now();
                for mut run in uncertain_runs {
                    let is_expired = (now - run.started_at).num_hours() >= 24;

                    if is_expired {
                        tracing::warn!(run_id = %run.id, "Uncertain run timed out after 24h; marking failed");
                        run.status = custos_contracts::RunStatus::Failed;
                        run.ended_at = Some(now);
                        let _ = uncertain_store.runs().save_run(&mut run);
                    }
                }
            }
        }
    });

    // Background Telemetry Retention Pruner: runs every hour and prunes historical spans/logs older than 30 days
    let pruning_store = store;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let cutoff_30d_ms = now_ms - (30 * 24 * 3600 * 1000);
            match pruning_store
                .runs()
                .prune_telemetry_before(cutoff_30d_ms, cutoff_30d_ms)
            {
                Ok(pruned) if pruned > 0 => {
                    tracing::info!(
                        pruned,
                        "Pruned historical telemetry records older than 30 days"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(error = %err, "Historical telemetry pruning failed");
                }
            }
        }
    });
}

/// Spawns the unified WorkflowScheduler ticker sharing the coordinator/API scheduler instance.
pub fn spawn_workflow_scheduler_ticker(
    scheduler: Arc<custos_engine::orchestration::WorkflowScheduler>,
    store: Arc<SqliteTaskStore>,
    task_service: Arc<custos_engine::TaskService>,
    workflow: Arc<dyn custos_contracts::WorkflowPort>,
    event_bus: crate::api::contexts::GlobalEventBus,
) {
    tokio::spawn(async move {
        if let Ok(jobs) = store.fleet_automation().list_headless_jobs() {
            for job in jobs {
                if let custos_contracts::HeadlessTrigger::Cron { ref schedule } = job.trigger {
                    scheduler
                        .schedule(
                            job.name.clone(),
                            custos_engine::orchestration::scheduler::JobRecurrence::Cron {
                                expression: schedule.clone(),
                            },
                            chrono::Utc::now(),
                            serde_json::json!({
                                "headless_job_id": job.id,
                                "prompt": job.spec.command_or_script,
                                "workspace_id": job.spec.workspace_id,
                            }),
                        )
                        .await;
                }
            }
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();
            if !scheduler.check_wake_settled(now.timestamp_millis()) {
                continue;
            }
            let due = scheduler.get_due_jobs(now).await;
            for job in due {
                let job_key = job.id.to_string();
                let lane = scheduler.lane_for(&job_key).await;
                let _guard = lane.lock().await;
                let _ = scheduler.mark_running(&job.id).await;

                let prompt = job
                    .contract_payload
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&job.name)
                    .to_string();
                let workspace_id = job
                    .contract_payload
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);

                let title = format!("Scheduled: {}", job.name);
                match task_service.create_task(title).await {
                    Ok(task) => {
                        let mut cmd = custos_contracts::StartRunCommand::new(
                            task.id.clone(),
                            format!("scheduler:{job_key}"),
                        )
                        .with_prompt(prompt);
                        if let Some(ws_id) = workspace_id {
                            cmd = cmd.with_workspace_id(ws_id);
                        }
                        match workflow.start_run_background(cmd).await {
                            Ok(_) => {
                                let _ = event_bus.send(
                                    crate::api::contexts::DaemonBroadcastEvent::AutomationJobDispatched {
                                        job_id: job_key,
                                        task_id: task.id.clone(),
                                    },
                                );
                                let _ = scheduler.mark_finished(&job.id, true, None).await;
                            }
                            Err(err) => {
                                let _ = scheduler
                                    .mark_finished(&job.id, false, Some(err.to_string()))
                                    .await;
                            }
                        }
                    }
                    Err(err) => {
                        let _ = scheduler
                            .mark_finished(&job.id, false, Some(err.to_string()))
                            .await;
                    }
                }
            }
        }
    });
}
