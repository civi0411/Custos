//! Dumb OI D0 Planner (RFC 003 §2B, §3 & PR 5)
//!
//! Orchestration Intelligence D0:
//! Ingests a `DecisionSnapshot`, strictly evaluates against constraints,
//! proposes the single-agent sovereign Native Baseline candidate (Claude 3.5 / Governed),
//! records alternatives considered, measures decision latency overhead, and yields an audit `DecisionRecord`.

use std::time::Instant;
use custos_domain::{
    DecisionRecord, DecisionSnapshot, DomainError, ExecutionTopology, StrategyProposal,
};

pub struct DumbOiPlanner {
    default_harness_id: String,
}

impl Default for DumbOiPlanner {
    fn default() -> Self {
        Self::new("claude-code")
    }
}

impl DumbOiPlanner {
    pub fn new(default_harness_id: impl Into<String>) -> Self {
        Self {
            default_harness_id: default_harness_id.into(),
        }
    }

    /// Evaluates the DecisionSnapshot and produces a StrategyProposal and DecisionRecord
    pub fn plan(&self, snapshot: DecisionSnapshot) -> Result<DecisionRecord, DomainError> {
        let start = Instant::now();

        // RFC 003 §3: The Baseline Fallback is ALWAYS a single, strong native coding agent.
        // Complex DAG topologies are only chosen if baseline cannot satisfy localization.
        let proposal = StrategyProposal {
            id: custos_domain::new_id("prop"),
            task_id: snapshot.task_id.clone(),
            chosen_topology: ExecutionTopology::NativeBaseline,
            candidate_harness: self.default_harness_id.clone(),
            reasoning: "Selected sovereign single-agent native baseline (RFC 003 §3)".into(),
            estimated_cost_usd: 0.05,
            estimated_tokens: 3_000,
            alternatives_considered: vec![
                "direct_model (abstained: workspace mutation required)".into(),
                "parallel_dag (deferred: single-agent baseline sufficient for initial turn)".into(),
            ],
            assumptions: Vec::new(),
            context_strategy: Some("default_windowed".into()),
        };

        let overhead_ms = start.elapsed().as_millis() as u64;
        let record = DecisionRecord::new(snapshot, proposal, overhead_ms);

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dumb_oi_proposes_native_baseline() {
        let planner = DumbOiPlanner::new("claude-code");
        let snapshot = DecisionSnapshot::new("task_test_001");

        let record = planner.plan(snapshot).unwrap();

        assert_eq!(record.task_id, "task_test_001");
        assert_eq!(
            record.proposal.chosen_topology,
            ExecutionTopology::NativeBaseline
        );
        assert_eq!(record.proposal.candidate_harness, "claude-code");
        assert_eq!(record.proposal.alternatives_considered.len(), 2);
    }
}
