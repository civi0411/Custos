//! Admissibility Gate for Strategy Proposals (RFC 004 §2B)
//!
//! Validates proposed topologies and candidates before passing to the compiler.

use super::hard_filters::HardFilters;
use custos_domain::oi::{DecisionSnapshot, RejectionReason, StrategyProposal};

#[derive(Debug, Clone, PartialEq)]
pub enum AdmissibilityResult {
    /// Strategy proposal meets all invariants and constraints
    Admitted(StrategyProposal),
    /// Proposal rejected with specific, auditable reasons
    Rejected(Vec<RejectionReason>),
}

pub struct AdmissibilityEvaluator;

impl AdmissibilityEvaluator {
    /// Evaluates a proposal against the current decision snapshot.
    pub fn evaluate(
        proposal: StrategyProposal,
        snapshot: &DecisionSnapshot,
    ) -> AdmissibilityResult {
        match HardFilters::evaluate(&proposal, snapshot) {
            Ok(()) => AdmissibilityResult::Admitted(proposal),
            Err(reasons) => AdmissibilityResult::Rejected(reasons),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admissibility_evaluation_flow() {
        let snapshot = DecisionSnapshot::new("task_adm_1");
        let proposal = StrategyProposal::native_baseline("task_adm_1", "claude-code");

        let result = AdmissibilityEvaluator::evaluate(proposal.clone(), &snapshot);
        match result {
            AdmissibilityResult::Admitted(admitted) => {
                assert_eq!(admitted.task_id, "task_adm_1");
            }
            AdmissibilityResult::Rejected(reasons) => {
                panic!("Expected admitted, got rejected: {:?}", reasons);
            }
        }
    }
}
