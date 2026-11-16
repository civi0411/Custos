//! Pure value objects for run-step execution telemetry, latency breakdowns, and logs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Kind of execution step inside a run or worker run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepSpanKind {
    Llm,
    Tool,
    Agent,
    Error,
    Harness,
    Session,
    Verification,
}

impl StepSpanKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Tool => "tool",
            Self::Agent => "agent",
            Self::Error => "error",
            Self::Harness => "harness",
            Self::Session => "session",
            Self::Verification => "verification",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "llm" | "model" | "gen_ai" => Self::Llm,
            "tool" => Self::Tool,
            "agent" | "worker" => Self::Agent,
            "error" | "exception" => Self::Error,
            "harness" | "native" => Self::Harness,
            "session" | "approval" | "hitl" => Self::Session,
            "verification" | "verifier" | "backtest" => Self::Verification,
            _ => Self::Tool,
        }
    }
}

/// Lifecycle state of an individual execution step span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepSpanStatus {
    Running,
    Success,
    Error,
}

impl StepSpanStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Error => "error",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "success" | "ok" | "completed" => Self::Success,
            "error" | "failed" => Self::Error,
            _ => Self::Running,
        }
    }
}

/// Severity of a structured runtime log entry captured under a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogSeverity {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "info" => Self::Info,
            "warn" | "warning" => Self::Warn,
            "error" => Self::Error,
            "fatal" | "critical" => Self::Fatal,
            _ => Self::Info,
        }
    }

    pub fn from_otlp_severity_number(num: i32) -> Self {
        match num {
            1..=4 => Self::Trace,
            5..=8 => Self::Debug,
            9..=12 => Self::Info,
            13..=16 => Self::Warn,
            17..=20 => Self::Error,
            21..=24 => Self::Fatal,
            _ => Self::Info,
        }
    }
}

/// Bounded step span within a run (OTLP child span or TaskRuntime internal step).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunStepSpan {
    pub id: String,
    pub run_id: String,
    pub worker_run_id: Option<String>,
    pub parent_id: Option<String>,
    pub kind: StepSpanKind,
    pub name: String,
    pub status: StepSpanStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub metadata: Value,
}

impl RunStepSpan {
    pub fn new(
        id: impl Into<String>,
        run_id: impl Into<String>,
        kind: StepSpanKind,
        name: impl Into<String>,
        started_at_ms: i64,
    ) -> Self {
        Self {
            id: id.into(),
            run_id: run_id.into(),
            worker_run_id: None,
            parent_id: None,
            kind,
            name: name.into(),
            status: StepSpanStatus::Running,
            started_at_ms,
            ended_at_ms: None,
            duration_ms: None,
            metadata: Value::Object(Default::default()),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn with_worker_run(mut self, worker_run_id: impl Into<String>) -> Self {
        self.worker_run_id = Some(worker_run_id.into());
        self
    }

    pub fn with_metadata(mut self, meta: Value) -> Self {
        self.metadata = meta;
        self
    }

    pub fn finish(&mut self, ended_at_ms: i64, status: StepSpanStatus) {
        self.ended_at_ms = Some(ended_at_ms);
        self.duration_ms = Some((ended_at_ms - self.started_at_ms).max(0));
        self.status = status;
    }
}

/// Normalized runtime error recorded during step execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepError {
    pub id: String,
    pub run_id: String,
    pub worker_run_id: Option<String>,
    pub span_id: Option<String>,
    pub error_type: String,
    pub message: String,
    pub signature_hash: String,
    pub stack: Option<String>,
    pub agent: String,
    pub timestamp_ms: i64,
}

/// Correlated structured log entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisionLogEntry {
    pub id: String,
    pub run_id: String,
    pub worker_run_id: Option<String>,
    pub span_id: Option<String>,
    pub timestamp_ms: i64,
    pub level: LogSeverity,
    pub message: String,
    #[serde(default)]
    pub metadata: Value,
}

/// Accurate latency breakdown across execution categories (in milliseconds).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LatencyBreakdown {
    pub llm_ms: i64,
    pub tool_ms: i64,
    pub harness_ms: i64,
    pub other_ms: i64,
    pub total_ms: i64,
}

impl LatencyBreakdown {
    pub fn compute(spans: &[RunStepSpan]) -> Self {
        let mut llm_ms = 0;
        let mut tool_ms = 0;
        let mut harness_ms = 0;
        let mut other_ms = 0;

        for span in spans {
            let dur = span.duration_ms.unwrap_or(0);
            match span.kind {
                StepSpanKind::Llm => llm_ms += dur,
                StepSpanKind::Tool => tool_ms += dur,
                StepSpanKind::Harness => harness_ms += dur,
                StepSpanKind::Agent | StepSpanKind::Session | StepSpanKind::Verification | StepSpanKind::Error => {
                    other_ms += dur;
                }
            }
        }

        let total_ms = (llm_ms + tool_ms + harness_ms + other_ms).max(0);
        Self {
            llm_ms,
            tool_ms,
            harness_ms,
            other_ms,
            total_ms,
        }
    }
}
