//! WorkPacket and WorkerResult contracts (RFC 003 §2C / RFC 005)
//!
//! Typed execution unit dispatched to a worker, containing strictly bounded capabilities,
//! budget slices, and read/write sets.

use crate::ids::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPacket {
    pub packet_id: String,
    pub task_id: String,
    pub node_id: String,
    pub allocated_budget_tokens: u64,
    pub allowed_capabilities: Vec<String>,
    pub instructions: String,
    pub read_set: Vec<String>,
    pub write_set: Vec<String>,
}

impl WorkPacket {
    pub fn new(
        task_id: impl Into<String>,
        node_id: impl Into<String>,
        instructions: impl Into<String>,
        allocated_budget_tokens: u64,
    ) -> Self {
        Self {
            packet_id: new_id("wpk"),
            task_id: task_id.into(),
            node_id: node_id.into(),
            allocated_budget_tokens,
            allowed_capabilities: Vec::new(),
            instructions: instructions.into(),
            read_set: Vec::new(),
            write_set: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Goal achieved sufficiently
    Sufficient,
    /// Partial progress achieved, subsequent steps required
    Partial,
    /// Worker abstained from execution (e.g. uncertain or out of scope)
    Abstain,
    /// Execution failed with error
    Failed,
    /// Cancelled by kernel or timeout
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub packet_id: String,
    pub status: WorkerStatus,
    pub payload: String,
    pub tokens_used: u64,
    pub cost_usd: f32,
    pub evidence_references: Vec<String>,
}

impl WorkerResult {
    pub fn success(packet_id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            packet_id: packet_id.into(),
            status: WorkerStatus::Sufficient,
            payload: payload.into(),
            tokens_used: 0,
            cost_usd: 0.0,
            evidence_references: Vec::new(),
        }
    }

    pub fn failed(packet_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            packet_id: packet_id.into(),
            status: WorkerStatus::Failed,
            payload: error.into(),
            tokens_used: 0,
            cost_usd: 0.0,
            evidence_references: Vec::new(),
        }
    }

    pub fn abstain(packet_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            packet_id: packet_id.into(),
            status: WorkerStatus::Abstain,
            payload: reason.into(),
            tokens_used: 0,
            cost_usd: 0.0,
            evidence_references: Vec::new(),
        }
    }
}
