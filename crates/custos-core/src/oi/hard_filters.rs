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

        // 4. Egress Rules & Local-Only Constraint check
        let is_local_only = snapshot
            .required_capabilities
            .iter()
            .any(|c| c.eq_ignore_ascii_case("LocalOnly") || c.eq_ignore_ascii_case("local-only"))
            || snapshot
                .egress_rules
                .iter()
                .any(|r| r.eq_ignore_ascii_case("LocalOnly") || r.eq_ignore_ascii_case("local-only"));

        if is_local_only {
            let is_candidate_local = proposal.candidate_harness.contains("local")
                || proposal.candidate_harness.contains("llama")
                || proposal.candidate_harness.contains("mlx")
                || proposal.candidate_harness.contains("ollama");
            if !is_candidate_local {
                reasons.push(RejectionReason::LocalOnlyConstraint);
            }
        }

        for rule in &snapshot.egress_rules {
            if rule == "no-external-egress"
                && (proposal.candidate_harness.contains("cloud")
                    || proposal.candidate_harness.contains("claude")
                    || proposal.candidate_harness.contains("openai"))
            {
                if !reasons.contains(&RejectionReason::EgressViolation(rule.clone())) {
                    reasons.push(RejectionReason::EgressViolation(rule.clone()));
                }
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

    #[test]
    fn test_hard_filter_local_only_constraint() {
        let mut snapshot = DecisionSnapshot::new("task_local");
        snapshot.required_capabilities = vec!["LocalOnly".into()];

        let cloud_proposal = StrategyProposal::native_baseline("task_local", "claude-code");
        let cloud_res = HardFilters::evaluate(&cloud_proposal, &snapshot);
        assert!(cloud_res.is_err());
        assert!(cloud_res.unwrap_err().contains(&RejectionReason::LocalOnlyConstraint));

        let local_proposal = StrategyProposal::native_baseline("task_local", "custos-local");
        let local_res = HardFilters::evaluate(&local_proposal, &snapshot);
        assert!(local_res.is_ok());
    }
}
