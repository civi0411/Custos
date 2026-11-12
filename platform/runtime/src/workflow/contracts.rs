//! Workflow contracts and execution context

use custos_domain::{DomainError, ExecutionReceipt, ReceiptStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionContext {
    pub task_id: String,
    pub run_id: String,
    pub node_id: String,
    pub worker_id: String,
}

impl ExecutionContext {
    pub fn new(
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        node_id: impl Into<String>,
        worker_id: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            run_id: run_id.into(),
            node_id: node_id.into(),
            worker_id: worker_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepObservation {
    ToolExecuted {
        attempt_id: String,
        receipt: ExecutionReceipt,
    },
    ToolPending {
        attempt_id: String,
    },
    ToolFailed {
        attempt_id: String,
        error: String,
    },
    ToolUncertain {
        attempt_id: String,
        error: String,
    },
    NodeComplete {
        node_id: String,
        output: serde_json::Value,
    },
}

impl StepObservation {
    /// Compatibility boundary for executors that still return JSON.
    pub fn from_output(node_id: &str, output: serde_json::Value) -> Result<Self, DomainError> {
        if output.get("receipt_id").is_some() {
            let receipt = serde_json::from_value::<ExecutionReceipt>(output)
                .map_err(|_| DomainError::Validation("Malformed execution receipt".into()))?;
            let attempt_id = receipt.action_id.clone();
            return Ok(match receipt.status {
                ReceiptStatus::Success => Self::ToolExecuted {
                    attempt_id,
                    receipt,
                },
                ReceiptStatus::Timeout => Self::ToolUncertain {
                    attempt_id,
                    error: receipt
                        .error_message
                        .unwrap_or_else(|| "Effect timed out; reconcile before retry".into()),
                },
                ReceiptStatus::Failure | ReceiptStatus::Rejected => Self::ToolFailed {
                    attempt_id,
                    error: receipt
                        .error_message
                        .unwrap_or_else(|| "Execution did not succeed".into()),
                },
            });
        }
        let status = output
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let attempt_id = output.get("attempt_id").and_then(|value| value.as_str());
        let error = || {
            output
                .get("error")
                .or_else(|| output.get("error_message"))
                .and_then(|value| value.as_str())
                .unwrap_or("Step execution did not succeed")
                .to_string()
        };
        Ok(match status {
            "pending_approval" => Self::ToolPending {
                attempt_id: attempt_id
                    .filter(|id| !id.trim().is_empty())
                    .ok_or_else(|| {
                        DomainError::Validation("Pending approval requires an attempt ID".into())
                    })?
                    .to_string(),
            },
            "uncertain" | "timeout" | "timed_out" => Self::ToolUncertain {
                attempt_id: attempt_id.unwrap_or(node_id).to_string(),
                error: error(),
            },
            "failure" | "failed" | "error" | "rejected" => Self::ToolFailed {
                attempt_id: attempt_id.unwrap_or(node_id).to_string(),
                error: error(),
            },
            _ => Self::NodeComplete {
                node_id: node_id.to_string(),
                output,
            },
        })
    }
}
