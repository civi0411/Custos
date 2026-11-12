//! Background Sweepers and Jobs (SADE R8 §6.8)

use std::sync::Arc;
use custos_persistence::SqliteTaskStore;

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
                    let settings = match custos_adapters::connectors::oauth::oauth_settings(
                        &tok.service_type,
                    ) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
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
    let uncertain_store = store;
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
                        run.status = custos_domain::RunStatus::Failed;
                        run.ended_at = Some(now);
                        let _ = uncertain_store.runs().save_run(&mut run);
                    }
                }
            }
        }
    });
}
