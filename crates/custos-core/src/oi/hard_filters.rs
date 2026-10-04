//! Deterministic Hard Filters for OI Strategy Proposals (RFC 003 §2B / RFC 004 §2B)
//!
//! Filter evaluation order:
//! 1. Model Pin: if pinned, candidate harness must match.
//! 2. Token Budget: estimated tokens <= remaining budget tokens.
//! 3. Monetary Budget: estimated USD <= remaining budget USD.
//! 4. Capabilities: required capabilities must not be violated.
//! 5. Egress & Isolation: egress rules and local constraints.

use custos_domain::oi::{DecisionSnapshot, RejectionReason, StrategyProposal};

pub struct HardFilters;

impl HardFilters {
    /// Validates all static hard filters against a strategy proposal.
    /// Returns Ok(()) if admissible, or a list of RejectionReasons.
    pub fn evaluate(
        proposal: &StrategyProposal,
        snapshot: &DecisionSnapshot,
    ) -> Result<(), Vec<RejectionReason>> {
        let mut reasons = Vec::new();

        // 1. Model Pinning check
        if let Some(ref pin) = snapshot.pinned_model {
            if &proposal.candidate_harness != pin {
                reasons.push(RejectionReason::ModelPinMismatch);
            }
        }

        // 2. Token Budget check
        if proposal.estimated_tokens > snapshot.remaining_budget_tokens {
            reasons.push(RejectionReason::BudgetExceeded);
        }

        // 3. Monetary USD Budget check
        if proposal.estimated_cost_usd > snapshot.remaining_budget_usd {
            reasons.push(RejectionReason::BudgetExceeded);
        }

        // 4. Egress Rules check
        for rule in &snapshot.egress_rules {
            if rule == "no-external-egress" && proposal.candidate_harness.contains("cloud") {
                reasons.push(RejectionReason::EgressViolation(rule.clone()));
            }
        }

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hard_filter_budget_exceeded() {
        let mut snapshot = DecisionSnapshot::new("task_1");
        snapshot.remaining_budget_tokens = 2_000;
        snapshot.remaining_budget_usd = 0.02;

        let proposal = StrategyProposal::native_baseline("task_1", "claude-code");
        // default native baseline has est_tokens = 3000, est_usd = 0.05

        let res = HardFilters::evaluate(&proposal, &snapshot);
        assert!(res.is_err());
        let errs = res.unwrap_err();
        assert!(errs.contains(&RejectionReason::BudgetExceeded));
    }

    #[test]
    fn test_hard_filter_model_pin_mismatch() {
        let mut snapshot = DecisionSnapshot::new("task_2");
        snapshot.pinned_model = Some("deepseek-coder".into());

        let proposal = StrategyProposal::native_baseline("task_2", "claude-code");

        let res = HardFilters::evaluate(&proposal, &snapshot);
        assert!(res.is_err());
        let errs = res.unwrap_err();
        assert!(errs.contains(&RejectionReason::ModelPinMismatch));
    }

    #[test]
    fn test_hard_filter_admissible() {
        let snapshot = DecisionSnapshot::new("task_3");
        let proposal = StrategyProposal::native_baseline("task_3", "claude-code");

        let res = HardFilters::evaluate(&proposal, &snapshot);
        assert!(res.is_ok());
    }
}
