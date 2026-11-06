use serde::{Deserialize, Serialize};

/// Execution policy class for a worker process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerExecutionKind {
    /// One-shot bounded operation (parsers, backtests, single evaluations).
    /// Auto-terminated upon completion or timeout.
    #[default]
    OneShot,
    /// Stateful long-running engine (interactive notebook kernel, live PTY).
    /// Kept alive for session duration until explicit termination.
    StatefulKernel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerOperation {
    pub operation_id: String,
    pub component_id: String,
    pub version: String,
    pub operation_kind: String,
    #[serde(default)]
    pub execution_kind: WorkerExecutionKind,
    pub payload: serde_json::Value,
    pub resource_budget: WorkerBudget,
}

impl WorkerOperation {
    pub fn new(
        operation_id: impl Into<String>,
        component_id: impl Into<String>,
        version: impl Into<String>,
        operation_kind: impl Into<String>,
        payload: serde_json::Value,
        resource_budget: WorkerBudget,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            component_id: component_id.into(),
            version: version.into(),
            operation_kind: operation_kind.into(),
            execution_kind: WorkerExecutionKind::OneShot,
            payload,
            resource_budget,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerBudget {
    pub max_wall_secs: u64,
    pub max_output_bytes: u64,
    pub max_memory_bytes: Option<u64>,
}

impl Default for WorkerBudget {
    fn default() -> Self {
        Self {
            max_wall_secs: 30,
            max_output_bytes: 64 * 1024, // 64 KiB bounded wire buffer
            max_memory_bytes: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerUsage {
    pub wall_ms: u64,
    pub output_bytes: u64,
    pub memory_peak_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerObservation {
    pub operation_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub resource_usage: WorkerUsage,
}

impl WorkerObservation {
    pub fn success(
        operation_id: impl Into<String>,
        output: serde_json::Value,
        wall_ms: u64,
        output_bytes: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            success: true,
            output: Some(output),
            error: None,
            resource_usage: WorkerUsage {
                wall_ms,
                output_bytes,
                memory_peak_bytes: None,
            },
        }
    }

    pub fn failure(
        operation_id: impl Into<String>,
        error: impl Into<String>,
        wall_ms: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            success: false,
            output: None,
            error: Some(error.into()),
            resource_usage: WorkerUsage {
                wall_ms,
                output_bytes: 0,
                memory_peak_bytes: None,
            },
        }
    }
}
