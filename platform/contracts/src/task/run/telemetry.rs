//! Execution supervision telemetry filters, observation sinks, and query ports.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::finding::{ErrorGroupSummary, SupervisionFinding, SupervisionSeverity};
use super::step_span::{LogSeverity, RunStepSpan, StepError, SupervisionLogEntry};
use super::watchdog::{SupervisionProfile, SupervisionRunRecord};
use crate::identity::error::DomainError;

/// Non-blocking observer port used by TaskRuntime, ToolExecutor, and Harness adapters.
pub trait SupervisionTraceSink: Send + Sync {
    fn notify_run_state(
        &self,
        run_id: &str,
        workbench_id: Option<&str>,
        agent: &str,
        active: bool,
        started_at_ms: i64,
        ended_at_ms: Option<i64>,
    );

    fn record_span(&self, span: RunStepSpan);

    fn record_log(&self, entry: SupervisionLogEntry);

    fn record_error(&self, error: StepError);
}

/// Zero-overhead NullObject implementation for lite tests or disabled supervision paths.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSupervisionSink;

impl SupervisionTraceSink for NoopSupervisionSink {
    fn notify_run_state(
        &self,
        _run_id: &str,
        _workbench_id: Option<&str>,
        _agent: &str,
        _active: bool,
        _started_at_ms: i64,
        _ended_at_ms: Option<i64>,
    ) {
    }

    fn record_span(&self, _span: RunStepSpan) {}
    fn record_log(&self, _entry: SupervisionLogEntry) {}
    fn record_error(&self, _error: StepError) {}
}

/// Combined overview statistics across supervised runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisionOverview {
    pub active_runs: usize,
    pub total_runs: usize,
    pub failed_runs: usize,
    pub open_findings: usize,
    pub critical_findings: usize,
    pub dropped_events: u64,
    pub registered_profiles: Vec<String>,
}

/// Filter parameters for listing supervised runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisionRunFilter {
    pub workbench_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

/// Filter parameters for querying structured logs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisionLogFilter {
    pub run_id: Option<String>,
    pub level: Option<LogSeverity>,
    pub span_id: Option<String>,
    pub limit: Option<usize>,
}

/// Filter parameters for querying anomaly findings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisionFindingFilter {
    pub run_id: Option<String>,
    pub workbench_id: Option<String>,
    pub severity: Option<SupervisionSeverity>,
    pub unresolved_only: bool,
    pub limit: Option<usize>,
}

/// Complete waterfall and diagnostic bundle for a single supervised run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWaterfallView {
    pub run: SupervisionRunRecord,
    pub spans: Vec<RunStepSpan>,
    pub logs: Vec<SupervisionLogEntry>,
    pub errors: Vec<StepError>,
    pub error_groups: Vec<ErrorGroupSummary>,
    pub findings: Vec<SupervisionFinding>,
    pub applied_profile: SupervisionProfile,
}

/// Normalized batch accepted by external OTLP or harness adapters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NormalizedTelemetryBatch {
    pub runs: Vec<SupervisionRunRecord>,
    pub spans: Vec<RunStepSpan>,
    pub errors: Vec<StepError>,
    pub logs: Vec<SupervisionLogEntry>,
}

/// Protocol ingest contract implemented by the runtime workflow watchdog coordinator.
#[async_trait]
pub trait SupervisionIngestPort: Send + Sync {
    async fn ingest_batch(&self, batch: NormalizedTelemetryBatch) -> Result<usize, DomainError>;
}

/// Read-only query contract for UI Dashboard/Studio, CLI diagnostics, and RPC endpoints.
#[async_trait]
pub trait SupervisionQueryPort: Send + Sync {
    async fn overview(&self) -> Result<SupervisionOverview, DomainError>;
    async fn list_runs(
        &self,
        filter: SupervisionRunFilter,
    ) -> Result<Vec<SupervisionRunRecord>, DomainError>;
    async fn get_run_waterfall(&self, run_id: &str) -> Result<Option<RunWaterfallView>, DomainError>;
    async fn list_logs(
        &self,
        filter: SupervisionLogFilter,
    ) -> Result<Vec<SupervisionLogEntry>, DomainError>;
    async fn list_findings(
        &self,
        filter: SupervisionFindingFilter,
    ) -> Result<Vec<SupervisionFinding>, DomainError>;
    async fn list_error_groups(
        &self,
        workbench_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ErrorGroupSummary>, DomainError>;
    async fn list_profiles(&self) -> Result<Vec<SupervisionProfile>, DomainError>;
}

