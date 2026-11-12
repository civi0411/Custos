//! Durable Decision Record appended to the ledger (RFC 003 §2D)

use super::proposal::StrategyProposal;
use super::snapshot::DecisionSnapshot;
use crate::ids::new_id;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub task_id: String,
    pub created_at: DateTime<Utc>,
    pub snapshot: DecisionSnapshot,
    pub proposal: StrategyProposal,
    pub overhead_ms: u64,
}

impl DecisionRecord {
    pub fn new(snapshot: DecisionSnapshot, proposal: StrategyProposal, overhead_ms: u64) -> Self {
        Self {
            id: new_id("drec"),
            task_id: snapshot.task_id.clone(),
            created_at: Utc::now(),
            snapshot,
            proposal,
            overhead_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oi::ExecutionTopology;

    #[test]
    fn test_decision_record_lifecycle() {
        let snapshot = DecisionSnapshot::new("task_123");
        let proposal = StrategyProposal::native_baseline("task_123", "claude-code");
        let record = DecisionRecord::new(snapshot.clone(), proposal.clone(), 12);

        assert_eq!(record.task_id, "task_123");
        assert_eq!(
            record.proposal.chosen_topology,
            ExecutionTopology::NativeBaseline
        );
        assert_eq!(record.proposal.candidate_harness, "claude-code");
        assert_eq!(record.overhead_ms, 12);
        assert_eq!(record.proposal.alternatives_considered.len(), 2);
    }
}
