//! Supervised Agent Runner (Decorator Pattern)
//!
//! Wraps any [`AgentRuntimePort`] (whether an internal governed LLM loop or an
//! external subprocess CLI agent like Claude Code, Codex, Goose) with
//! Zero-Scatter Telemetry, StepSpan emission, and SupervisionTraceSink recording.

use async_trait::async_trait;
use custos_core::contracts::harness::{
    AgentRuntimePort, HarnessExecutionResult, HarnessProfile, RunObserver,
};
use custos_core::contracts::workflow::SupervisionTraceSink;
use custos_domain::{
    ActionIntent, ContextPack, DomainError, LogSeverity, RunStepSpan, StepError, StepSpanKind,
    StepSpanStatus, SupervisionLogEntry, WorkerRun,
};
use std::path::Path;
use std::sync::Arc;

/// A Decorator that instruments any AgentRuntimePort with Zero-Scatter telemetry spans.
pub struct SupervisedAgentRunner {
    inner: Arc<dyn AgentRuntimePort>,
    trace_sink: Option<Arc<dyn SupervisionTraceSink>>,
    workbench_id: Option<String>,
}

impl SupervisedAgentRunner {
    pub fn new(inner: Arc<dyn AgentRuntimePort>, trace_sink: Arc<dyn SupervisionTraceSink>) -> Self {
        Self {
            inner,
            trace_sink: Some(trace_sink),
            workbench_id: None,
        }
    }

    pub fn with_workbench(mut self, workbench_id: impl Into<String>) -> Self {
        self.workbench_id = Some(workbench_id.into());
        self
    }

    pub fn inner(&self) -> &Arc<dyn AgentRuntimePort> {
        &self.inner
    }
}

/// An observer wrapper that intercepts events from native CLI agents and records
/// them into the SupervisionTraceSink.
struct SupervisedObserver {
    inner: Arc<dyn RunObserver>,
    trace_sink: Option<Arc<dyn SupervisionTraceSink>>,
    run_id: String,
    span_id: String,
    agent_id: String,
}

impl RunObserver for SupervisedObserver {
    fn emit(&self, kind: &str, payload: serde_json::Value) {
        self.inner.emit(kind, payload.clone());
        if let Some(ref sink) = self.trace_sink {
            let now = chrono::Utc::now().timestamp_millis();
            let msg = payload.to_string();
            if kind == "error" {
                sink.record_error(StepError {
                    id: format!("err_{}", uuid::Uuid::new_v4().simple()),
                    run_id: self.run_id.clone(),
                    worker_run_id: None,
                    span_id: Some(self.span_id.clone()),
                    error_type: "native_cli_error".into(),
                    message: msg.clone(),
                    signature_hash: format!("{:x}", md5_or_simple_hash(&msg)),
                    stack: None,
                    agent: self.agent_id.clone(),
                    timestamp_ms: now,
                });
            } else {
                sink.record_log(SupervisionLogEntry {
                    id: format!("log_{}", uuid::Uuid::new_v4().simple()),
                    run_id: self.run_id.clone(),
                    worker_run_id: None,
                    span_id: Some(self.span_id.clone()),
                    timestamp_ms: now,
                    level: LogSeverity::Info,
                    message: format!("{kind}: {msg}"),
                    metadata: serde_json::json!({ "agent": self.agent_id, "kind": kind }),
                });
            }
        }
    }

    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn run_id(&self) -> Option<&str> {
        Some(&self.run_id)
    }
}

fn md5_or_simple_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[async_trait]
impl AgentRuntimePort for SupervisedAgentRunner {
    fn harness_id(&self) -> &str {
        self.inner.harness_id()
    }

    fn profile(&self) -> HarnessProfile {
        self.inner.profile()
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let run_id = worker_run.run_id.as_deref().unwrap_or(&worker_run.id);
        let span_id = format!("turn_{}", uuid::Uuid::new_v4().simple());
        let started_at_ms = chrono::Utc::now().timestamp_millis();
        let agent_id = self.harness_id().to_string();

        if let Some(ref sink) = self.trace_sink {
            sink.notify_run_state(
                run_id,
                self.workbench_id.as_deref(),
                &agent_id,
                true,
                started_at_ms,
                None,
            );
            let mut span = RunStepSpan::new(
                &span_id,
                run_id,
                StepSpanKind::Agent,
                format!("turn:{}", agent_id),
                started_at_ms,
            );
            span.worker_run_id = Some(worker_run.id.clone());
            sink.record_span(span);
        }

        let result = self.inner.execute_turn(worker_run, context_pack).await;
        let ended_at_ms = chrono::Utc::now().timestamp_millis();

        if let Some(ref sink) = self.trace_sink {
            sink.notify_run_state(
                run_id,
                self.workbench_id.as_deref(),
                &agent_id,
                false,
                started_at_ms,
                Some(ended_at_ms),
            );
            let status = if result.is_ok() {
                StepSpanStatus::Success
            } else {
                StepSpanStatus::Error
            };
            let mut span = RunStepSpan::new(
                &span_id,
                run_id,
                StepSpanKind::Agent,
                format!("turn:{}", agent_id),
                started_at_ms,
            );
            span.worker_run_id = Some(worker_run.id.clone());
            span.status = status;
            span.ended_at_ms = Some(ended_at_ms);
            span.duration_ms = Some(ended_at_ms.saturating_sub(started_at_ms));
            sink.record_span(span);
        }

        result
    }

    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        observer: Arc<dyn RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        let run_id = observer.run_id().unwrap_or("native-run").to_string();
        let span_id = format!("cli_{}", uuid::Uuid::new_v4().simple());
        let started_at_ms = chrono::Utc::now().timestamp_millis();
        let agent_id = self.harness_id().to_string();

        if let Some(ref sink) = self.trace_sink {
            sink.notify_run_state(
                &run_id,
                self.workbench_id.as_deref(),
                &agent_id,
                true,
                started_at_ms,
                None,
            );
            let span = RunStepSpan::new(
                &span_id,
                &run_id,
                StepSpanKind::Harness,
                format!("cli:{}", agent_id),
                started_at_ms,
            );
            sink.record_span(span);
        }

        let supervised_observer = Arc::new(SupervisedObserver {
            inner: observer,
            trace_sink: self.trace_sink.clone(),
            run_id: run_id.clone(),
            span_id: span_id.clone(),
            agent_id: agent_id.clone(),
        });

        let result = self.inner.run_native_observed(instruction, cwd, supervised_observer).await;
        let ended_at_ms = chrono::Utc::now().timestamp_millis();

        if let Some(ref sink) = self.trace_sink {
            sink.notify_run_state(
                &run_id,
                self.workbench_id.as_deref(),
                &agent_id,
                false,
                started_at_ms,
                Some(ended_at_ms),
            );
            let status = if result.is_ok() {
                StepSpanStatus::Success
            } else {
                StepSpanStatus::Error
            };
            let mut span = RunStepSpan::new(
                &span_id,
                &run_id,
                StepSpanKind::Harness,
                format!("cli:{}", agent_id),
                started_at_ms,
            );
            span.status = status;
            span.ended_at_ms = Some(ended_at_ms);
            span.duration_ms = Some(ended_at_ms.saturating_sub(started_at_ms));
            sink.record_span(span);
        }

        result
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        self.inner.cancel_run(run_id).await
    }

    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        self.inner.steer_run(run_id, guidance).await
    }

    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.inner.run_native(instruction, cwd).await
    }
}
