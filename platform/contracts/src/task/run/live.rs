use serde::{Deserialize, Serialize};

/// Ephemeral presentation event; it never grants authority for an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRunEvent {
    pub id: u64,
    pub session_id: String,
    pub turn_id: String,
    pub task_id: String,
    pub run_id: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRunSnapshot {
    pub cursor: u64,
    pub run_id: String,
    pub task_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub status: String,
    pub text: String,
    pub activities: Vec<serde_json::Value>,
    pub usage: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRunReplay {
    pub events: Vec<LiveRunEvent>,
    pub snapshot: LiveRunSnapshot,
    pub gap: bool,
}

/// Summary of entities reconciled during startup / crash recovery (G13).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    pub outbox_reconciled: usize,
    pub runs_reconciled: usize,
    pub attempts_reconciled: usize,
}
