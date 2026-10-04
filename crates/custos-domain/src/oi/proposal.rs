//! Strategy Proposal produced by OI (RFC 003 §2B / RFC 004)

use super::topology::ExecutionTopology;
use crate::ids::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyProposal {
    pub id: String,
    pub task_id: String,
    pub chosen_topology: ExecutionTopology,
    pub candidate_harness: String,
    pub reasoning: String,
    pub estimated_cost_usd: f32,
    pub estimated_tokens: u64,
    pub alternatives_considered: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub context_strategy: Option<String>,
}

impl StrategyProposal {
    pub fn native_baseline(task_id: impl Into<String>, harness_id: impl Into<String>) -> Self {
        Self {
            id: new_id("prop"),
            task_id: task_id.into(),
            chosen_topology: ExecutionTopology::NativeBaseline,
            candidate_harness: harness_id.into(),
            reasoning: "Selected sovereign single-agent native baseline (RFC 003 §3)".into(),
            estimated_cost_usd: 0.05,
            estimated_tokens: 3_000,
            alternatives_considered: vec!["direct_model".into(), "parallel_dag".into()],
            assumptions: Vec::new(),
            context_strategy: Some("default_windowed".into()),
        }
    }
}
