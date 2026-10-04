//! Decision Snapshot extracted by Kernel before OI deliberates (RFC 003 §2A / RFC 004)
//!
//! An immutable, zero-I/O summary of current task state, budget constraints,
//! active effects, and security policies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionSnapshot {
    pub task_id: String,
    pub task_version: u64,
    pub policy_version: String,
    pub remaining_budget_tokens: u64,
    pub remaining_budget_usd: f32,
    pub pending_effects_count: usize,
    pub uncertain_effects_count: usize,
    pub active_worker_runs_count: usize,
    pub criteria: Vec<String>,

    // ────────── V2 Hard-Filter Fields (RFC 004 §2A) ──────────
    #[serde(default)]
    pub pinned_model: Option<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub egress_rules: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

impl DecisionSnapshot {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            task_version: 1,
            policy_version: "custos.policy.v1".into(),
            remaining_budget_tokens: 100_000,
            remaining_budget_usd: 10.0,
            pending_effects_count: 0,
            uncertain_effects_count: 0,
            active_worker_runs_count: 0,
            criteria: Vec::new(),
            pinned_model: None,
            required_capabilities: Vec::new(),
            egress_rules: Vec::new(),
            unknowns: Vec::new(),
        }
    }
}
