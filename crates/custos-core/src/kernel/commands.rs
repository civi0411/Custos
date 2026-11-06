//! Task Commands (CQRS Write Side)
//!
//! Encapsulates user or system intentions to mutate task state.

use custos_domain::{TaskContract, TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub contract: Option<TaskContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceTask {
    pub task_id: String,
    pub expected_epoch: u64,
    pub next_status: TaskStatus,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTask {
    pub task_id: String,
    pub expected_epoch: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTask {
    pub task_id: String,
    pub expected_epoch: u64,
    pub summary: String,
    #[serde(default)]
    pub evidence_claims: Vec<custos_domain::VerificationClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTask {
    pub task_id: String,
    pub expected_epoch: u64,
    pub reason: String,
}

/// Typed specification for operation retry policy with exponential backoff
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u8,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f32,
}

impl Default for RetrySpec {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 250,
            max_delay_ms: 5000,
            backoff_factor: 2.0,
        }
    }
}

