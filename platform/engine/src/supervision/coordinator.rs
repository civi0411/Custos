//! Asynchronous SupervisionCoordinator: non-blocking channel facade, idle-zero sweeper lifecycle,
//! and port implementations (`SupervisionTraceSink`, `SupervisionIngestPort`, `SupervisionQueryPort`).

use super::processor::{ActiveRunWindow, SupervisionCommand};
use super::SupervisionProfileRegistry;
use crate::workflow::live::LiveRuns;
use async_trait::async_trait;
use custos_core::contracts::supervision::{
    NormalizedTelemetryBatch, RunWaterfallView, SupervisionFindingFilter, SupervisionIngestPort,
    SupervisionLogFilter, SupervisionOverview, SupervisionQueryPort, SupervisionRunFilter,
    SupervisionStorePort, SupervisionTraceSink,
};
use custos_domain::supervision::{
    default_rules, group_step_errors, ErrorGroupSummary, RunStepSpan, StepError,
    SupervisionFinding, SupervisionLogEntry, SupervisionProfile, SupervisionRule,
    SupervisionRunRecord,
};
use custos_domain::DomainError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify};

const CHANNEL_CAPACITY: usize = 4096;
const SWEEP_INTERVAL_SECS: u64 = 10;

/// Central execution supervision engine decoupling telemetry producers, deterministic rules,
/// SQLite persistence, and live SSE updates.
pub struct SupervisionCoordinator {
    tx: mpsc::Sender<SupervisionCommand>,
    rx: Mutex<Option<mpsc::Receiver<SupervisionCommand>>>,
    worker_started: AtomicBool,
    dropped_events: Arc<AtomicU64>,
    pub(super) store: Option<Arc<dyn SupervisionStorePort>>,
    pub(super) profiles: Arc<SupervisionProfileRegistry>,
    pub(super) rules: Arc<Vec<Box<dyn SupervisionRule>>>,
    pub(super) live: LiveRuns,
    pub(super) active_runs: Arc<Mutex<HashMap<String, ActiveRunWindow>>>,
    pub(super) sweeper_notify: Arc<Notify>,
}

impl Default for SupervisionCoordinator {
    fn default() -> Self {
        Self::new(None, LiveRuns::default())
    }
}

impl SupervisionCoordinator {
    pub fn new(store: Option<Arc<dyn SupervisionStorePort>>, live: LiveRuns) -> Self {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            worker_started: AtomicBool::new(false),
            dropped_events: Arc::new(AtomicU64::new(0)),
            store,
            profiles: Arc::new(SupervisionProfileRegistry::new()),
            rules: Arc::new(default_rules()),
            live,
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            sweeper_notify: Arc::new(Notify::new()),
        }
    }

    pub fn profiles(&self) -> &Arc<SupervisionProfileRegistry> {
        &self.profiles
    }

    /// Starts the background queue consumer and idle-zero sweeper when a Tokio runtime is active.
    pub fn ensure_started(self: &Arc<Self>) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        if self.worker_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let rx = match self.rx.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        let Some(mut rx) = rx else {
            return;
        };

        let coordinator = Arc::clone(self);
        tokio::spawn(async move {
            let mut batch: Vec<SupervisionCommand> = Vec::with_capacity(64);
            while let Some(first) = rx.recv().await {
                batch.push(first);
                while batch.len() < 64 {
                    match rx.try_recv() {
                        Ok(item) => batch.push(item),
                        Err(_) => break,
                    }
                }
                let drained = std::mem::take(&mut batch);
                let _ = coordinator.process_commands(drained).await;
            }
        });

        let sweeper = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                let has_active = sweeper
                    .active_runs
                    .lock()
                    .map(|g| !g.is_empty())
                    .unwrap_or(false);
                if !has_active {
                    // Event-driven sleep: 0% CPU and 0 DB queries while no active runs exist.
                    sweeper.sweeper_notify.notified().await;
                    continue;
                }
                tokio::time::sleep(std::time::Duration::from_secs(SWEEP_INTERVAL_SECS)).await;
                let _ = sweeper.sweep_active_runs().await;
            }
        });
    }

    fn enqueue(&self, cmd: SupervisionCommand) {
        if self.tx.try_send(cmd).is_err() {
            self.dropped_events.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl SupervisionTraceSink for SupervisionCoordinator {
    fn notify_run_state(
        &self,
        run_id: &str,
        workbench_id: Option<&str>,
        agent: &str,
        active: bool,
        started_at_ms: i64,
        ended_at_ms: Option<i64>,
    ) {
        self.enqueue(SupervisionCommand::RunState {
            run_id: run_id.to_string(),
            workbench_id: workbench_id.map(str::to_string),
            agent: agent.to_string(),
            active,
            started_at_ms,
            ended_at_ms,
        });
    }

    fn record_span(&self, span: RunStepSpan) {
        self.enqueue(SupervisionCommand::Span(span));
    }

    fn record_log(&self, entry: SupervisionLogEntry) {
        self.enqueue(SupervisionCommand::Log(entry));
    }

    fn record_error(&self, error: StepError) {
        self.enqueue(SupervisionCommand::Error(error));
    }
}

#[async_trait]
impl SupervisionIngestPort for SupervisionCoordinator {
    async fn ingest_batch(&self, batch: NormalizedTelemetryBatch) -> Result<usize, DomainError> {
        let count = batch.runs.len() + batch.spans.len() + batch.logs.len() + batch.errors.len();
        self.process_commands(vec![SupervisionCommand::Batch(batch)])
            .await?;
        Ok(count)
    }
}

#[async_trait]
impl SupervisionQueryPort for SupervisionCoordinator {
    async fn overview(&self) -> Result<SupervisionOverview, DomainError> {
        let (active_runs, total_runs, failed_runs, open_findings, critical_findings) =
            if let Some(ref store) = self.store {
                store.count_overview_stats().await?
            } else {
                let active_count = self.active_runs.lock().map(|g| g.len()).unwrap_or(0);
                (active_count, active_count, 0, 0, 0)
            };
        Ok(SupervisionOverview {
            active_runs,
            total_runs,
            failed_runs,
            open_findings,
            critical_findings,
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
            registered_profiles: self.profiles.profile_ids(),
        })
    }

    async fn list_runs(
        &self,
        filter: SupervisionRunFilter,
    ) -> Result<Vec<SupervisionRunRecord>, DomainError> {
        if let Some(ref store) = self.store {
            return store.list_runs(&filter).await;
        }
        let guard = self
            .active_runs
            .lock()
            .map_err(|_| DomainError::Validation("Supervision active lock unavailable".into()))?;
        Ok(guard.values().map(|w| w.record.clone()).collect())
    }

    async fn get_run_waterfall(&self, run_id: &str) -> Result<Option<RunWaterfallView>, DomainError> {
        if let Some(ref store) = self.store {
            let Some(run) = store.get_run_record(run_id).await? else {
                return Ok(None);
            };
            let spans = store.list_run_spans(run_id).await?;
            let logs = store
                .list_logs(&SupervisionLogFilter {
                    run_id: Some(run_id.to_string()),
                    limit: Some(500),
                    ..Default::default()
                })
                .await?;
            let errors = store.list_run_errors(run_id).await?;
            let error_groups = group_step_errors(&errors);
            let findings = store
                .list_findings(&SupervisionFindingFilter {
                    run_id: Some(run_id.to_string()),
                    limit: Some(200),
                    ..Default::default()
                })
                .await?;
            let applied_profile = self.profiles.resolve(run.workbench_id.as_deref());
            return Ok(Some(RunWaterfallView {
                run,
                spans,
                logs,
                errors,
                error_groups,
                findings,
                applied_profile,
            }));
        }
        Ok(None)
    }

    async fn list_logs(
        &self,
        filter: SupervisionLogFilter,
    ) -> Result<Vec<SupervisionLogEntry>, DomainError> {
        if let Some(ref store) = self.store {
            return store.list_logs(&filter).await;
        }
        Ok(Vec::new())
    }

    async fn list_findings(
        &self,
        filter: SupervisionFindingFilter,
    ) -> Result<Vec<SupervisionFinding>, DomainError> {
        if let Some(ref store) = self.store {
            return store.list_findings(&filter).await;
        }
        Ok(Vec::new())
    }

    async fn list_error_groups(
        &self,
        workbench_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ErrorGroupSummary>, DomainError> {
        if let Some(ref store) = self.store {
            let errors = store.list_errors(workbench_id, limit).await?;
            return Ok(group_step_errors(&errors));
        }
        Ok(Vec::new())
    }

    async fn list_profiles(&self) -> Result<Vec<SupervisionProfile>, DomainError> {
        Ok(self.profiles.list_profiles())
    }
}
