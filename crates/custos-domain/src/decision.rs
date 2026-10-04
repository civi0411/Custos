//! Orchestration Intelligence (OI) Decision Types (RFC 003 / RFC 004)
//!
//! Re-exported from `crate::oi` for backward compatibility.

pub use crate::oi::{
    Candidate, DecisionRecord, DecisionSnapshot, ExecutionTopology, NodePlacement,
    RejectionReason, ReplanBrief, ReplanRecord, ReplanTrigger, StrategyProposal, WorkPacket,
    WorkerResult, WorkerStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

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
