//! Orchestration Intelligence (OI) Explanation Models
//!
//! Pure, deterministic data models explaining planner decisions:
//! candidate evaluations, rejection rationales, cost ranges, and compiled DAG preview.

use serde::{Deserialize, Serialize};

use super::topology::ExecutionTopology;

/// Detailed explanation for a single strategy candidate considered during planning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateExplanation {
    pub topology: ExecutionTopology,
    pub harness_id: String,
    pub estimated_tokens: u64,
    pub estimated_cost_usd_min: f32,
    pub estimated_cost_usd_max: f32,
    pub estimated_latency_ms: u64,
    pub admissible: bool,
    pub rejection_reasons: Option<Vec<String>>,
}

/// Comprehensive inspection report explaining the planner's decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainReport {
    pub task_id: String,
    pub chosen_topology: ExecutionTopology,
    pub chosen_harness_id: String,
    pub estimated_tokens: u64,
    pub estimated_cost_usd: f32,
    pub reasoning: String,
    pub cost_range_tokens: (u64, u64),
    pub cost_range_usd: (f32, f32),
    pub candidates: Vec<CandidateExplanation>,
    pub compiled_nodes_preview: Vec<String>,
    pub obligations: Vec<String>,
}
