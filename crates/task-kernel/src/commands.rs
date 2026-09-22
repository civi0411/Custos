//! Task Commands (CQRS Write Side)
//!
//! Encapsulates user or system intentions to mutate task state.

use custos_core_domain::TaskStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub metadata: Option<serde_json::Value>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTask {
    pub task_id: String,
    pub expected_epoch: u64,
    pub reason: String,
}
