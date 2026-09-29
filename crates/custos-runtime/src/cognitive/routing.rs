use custos_core_domain::{DomainError, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningTier {
    TierZero,
    SystemOne,
    SystemTwo,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSignals {
    pub deterministic_solution_available: bool,
    pub ambiguity: f32,
    pub uncertainty: f32,
    pub context_sufficient: bool,
    pub reversible: bool,
    pub risk: RiskLevel,
    pub evidence_required: bool,
    pub budget_available: bool,
    pub system_one_success_probability: f32,
    pub system_two_success_probability: f32,
    pub system_one_tokens: u64,
    pub system_two_tokens: u64,
    pub token_cost_weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
    Full,
    Summarized,
    NexusOnly,
    Bare,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2ATarget {
    pub agent_id: String,
    pub delegation_mode: DelegationMode,
    pub trust_score: f32,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationMode {
    FireAndForget,
    Await,
    Stream,
    Collaborate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FallbackRoute {
    pub provider_id: String,
    pub model_id: String,
    pub max_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteDecision {
    // D1: Cognitive Tier
    pub tier: ReasoningTier,
    pub rationale: String,
    pub reserved_token_budget: u64,
    pub require_human_approval: bool,

    // D2: Provider ID
    pub provider_id: String,
    // D3: Model ID
    pub model_id: String,
    // D4: Budget & Cost Ceiling
    pub cost_ceiling_usd: Option<f32>,
    pub latency_sla_ms: Option<u32>,
    // D5: Pack ID
    pub pack_id: Option<String>,
    // D6: Tool Permit Set
    pub tool_set: Vec<String>,
    // D7: A2A Target
    pub a2a_target: Option<A2ATarget>,
    // D8: Context Mode
    pub context_mode: ContextMode,
    // D9: Fallback Chain
    pub fallback_chain: Vec<FallbackRoute>,

    // Observability
    pub trace_id: String,
}

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    pub ambiguity_escalation: f32,
    pub uncertainty_escalation: f32,
    pub minimum_quality_uplift: f32,
    pub system_two_token_cap: u64,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            ambiguity_escalation: 0.65,
            uncertainty_escalation: 0.55,
            minimum_quality_uplift: 0.12,
            system_two_token_cap: 32_000,
        }
    }
}

impl RoutingPolicy {
    pub fn decide(&self, signals: &RoutingSignals) -> Result<RouteDecision, DomainError> {
        self.validate(signals)?;
        if !signals.budget_available {
            return Ok(self.decision(ReasoningTier::Abstain, "budget_exhausted", 0, false));
        }
        if signals.deterministic_solution_available
            && signals.ambiguity <= 0.1
            && signals.uncertainty <= 0.1
            && signals.risk == RiskLevel::Low
            && signals.reversible
            && !signals.evidence_required
        {
            return Ok(self.decision(ReasoningTier::TierZero, "deterministic_low_risk", 0, false));
        }

        let hard_escalation = signals.ambiguity >= self.ambiguity_escalation
            || signals.uncertainty >= self.uncertainty_escalation
            || !signals.context_sufficient
            || !signals.reversible
            || matches!(signals.risk, RiskLevel::High | RiskLevel::Critical)
            || signals.evidence_required;
        let uplift =
            signals.system_two_success_probability - signals.system_one_success_probability;
        let extra_tokens = signals
            .system_two_tokens
            .saturating_sub(signals.system_one_tokens);
        let cost_penalty = extra_tokens as f32 * signals.token_cost_weight;
        let expected_gain = uplift - cost_penalty;
        let choose_system_two = hard_escalation || expected_gain >= self.minimum_quality_uplift;

        if choose_system_two && signals.system_two_tokens <= self.system_two_token_cap {
            let needs_approval = matches!(signals.risk, RiskLevel::High | RiskLevel::Critical);
            return Ok(self.decision(
                ReasoningTier::SystemTwo,
                if hard_escalation {
                    "risk_or_uncertainty_requires_deliberation"
                } else {
                    "verified_uplift_justifies_cost"
                },
                signals.system_two_tokens,
                needs_approval,
            ));
        }
        if choose_system_two {
            return Ok(self.decision(
                ReasoningTier::Abstain,
                "system_two_budget_exceeded",
                0,
                false,
            ));
        }

        let reason = if expected_gain < 0.0 {
            "system_two_cost_exceeds_expected_gain"
        } else {
            "system_one_sufficient"
        };
        Ok(self.decision(
            ReasoningTier::SystemOne,
            reason,
            signals.system_one_tokens,
            false,
        ))
    }

    fn validate(&self, signals: &RoutingSignals) -> Result<(), DomainError> {
        let probabilities = [
            signals.ambiguity,
            signals.uncertainty,
            signals.system_one_success_probability,
            signals.system_two_success_probability,
        ];
        if probabilities
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || !signals.token_cost_weight.is_finite()
            || signals.token_cost_weight < 0.0
        {
            return Err(DomainError::Validation("routing probabilities must be in [0,1] and token cost weight must be finite and non-negative".into()));
        }
        Ok(())
    }

    fn decision(
        &self,
        tier: ReasoningTier,
        rationale: &'static str,
        reserved_token_budget: u64,
        require_human_approval: bool,
    ) -> RouteDecision {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let rat = rationale.to_string();
        match tier {
            ReasoningTier::TierZero => RouteDecision {
                tier,
                rationale: rat,
                reserved_token_budget,
                require_human_approval,
                provider_id: "deterministic".into(),
                model_id: "system-0-exact".into(),
                cost_ceiling_usd: Some(0.0),
                latency_sla_ms: Some(10),
                pack_id: None,
                tool_set: vec![],
                a2a_target: None,
                context_mode: ContextMode::Bare,
                fallback_chain: vec![],
                trace_id,
            },
            ReasoningTier::SystemOne => RouteDecision {
                tier,
                rationale: rat,
                reserved_token_budget,
                require_human_approval,
                provider_id: "claude".into(),
                model_id: "claude-3-5-haiku".into(),
                cost_ceiling_usd: Some(0.05),
                latency_sla_ms: Some(2500),
                pack_id: Some("assistant".into()),
                tool_set: vec!["file_read".into(), "grep_search".into()],
                a2a_target: None,
                context_mode: ContextMode::Summarized,
                fallback_chain: vec![FallbackRoute {
                    provider_id: "codex".into(),
                    model_id: "gpt-4o-mini".into(),
                    max_tokens: reserved_token_budget,
                }],
                trace_id,
            },
            ReasoningTier::SystemTwo => RouteDecision {
                tier,
                rationale: rat,
                reserved_token_budget,
                require_human_approval,
                provider_id: "claude".into(),
                model_id: "claude-3-5-sonnet".into(),
                cost_ceiling_usd: Some(0.50),
                latency_sla_ms: Some(15000),
                pack_id: Some("engineering".into()),
                tool_set: vec!["all".into()],
                a2a_target: None,
                context_mode: ContextMode::Full,
                fallback_chain: vec![FallbackRoute {
                    provider_id: "codex".into(),
                    model_id: "o3-mini".into(),
                    max_tokens: reserved_token_budget,
                }],
                trace_id,
            },
            ReasoningTier::Abstain => RouteDecision {
                tier,
                rationale: rat,
                reserved_token_budget,
                require_human_approval,
                provider_id: "none".into(),
                model_id: "none".into(),
                cost_ceiling_usd: Some(0.0),
                latency_sla_ms: None,
                pack_id: None,
                tool_set: vec![],
                a2a_target: None,
                context_mode: ContextMode::Bare,
                fallback_chain: vec![],
                trace_id,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals() -> RoutingSignals {
        RoutingSignals {
            deterministic_solution_available: false,
            ambiguity: 0.1,
            uncertainty: 0.1,
            context_sufficient: true,
            reversible: true,
            risk: RiskLevel::Low,
            evidence_required: false,
            budget_available: true,
            system_one_success_probability: 0.8,
            system_two_success_probability: 0.9,
            system_one_tokens: 2_000,
            system_two_tokens: 12_000,
            token_cost_weight: 0.000_01,
        }
    }

    #[test]
    fn uses_tier_zero_for_exact_safe_work() {
        let mut request = signals();
        request.deterministic_solution_available = true;
        let decision = RoutingPolicy::default().decide(&request).unwrap();
        assert_eq!(decision.tier, ReasoningTier::TierZero);
        assert_eq!(decision.reserved_token_budget, 0);
    }

    #[test]
    fn escalates_high_risk_and_marks_approval_requirement() {
        let mut request = signals();
        request.risk = RiskLevel::High;
        let decision = RoutingPolicy::default().decide(&request).unwrap();
        assert_eq!(decision.tier, ReasoningTier::SystemTwo);
        assert!(decision.require_human_approval);
    }

    #[test]
    fn keeps_simple_work_on_system_one_when_extra_cost_is_not_justified() {
        let decision = RoutingPolicy::default().decide(&signals()).unwrap();
        assert_eq!(decision.tier, ReasoningTier::SystemOne);
    }

    #[test]
    fn abstains_when_budget_is_exhausted_or_system_two_exceeds_cap() {
        let mut request = signals();
        request.budget_available = false;
        assert_eq!(
            RoutingPolicy::default().decide(&request).unwrap().tier,
            ReasoningTier::Abstain
        );

        let mut request = signals();
        request.ambiguity = 0.9;
        request.system_two_tokens = 40_000;
        assert_eq!(
            RoutingPolicy::default().decide(&request).unwrap().tier,
            ReasoningTier::Abstain
        );
    }

    #[test]
    fn rejects_non_finite_routing_inputs() {
        let mut request = signals();
        request.uncertainty = f32::NAN;
        assert!(RoutingPolicy::default().decide(&request).is_err());
    }

    #[test]
    fn test_nine_router_dimensions_populated() {
        let policy = RoutingPolicy::default();
        let s = signals();
        let d = policy.decide(&s).unwrap();

        // D1: Tier
        assert_eq!(d.tier, ReasoningTier::SystemOne);
        // D2: Provider
        assert_eq!(d.provider_id, "claude");
        // D3: Model
        assert_eq!(d.model_id, "claude-3-5-haiku");
        // D4: Budget & Cost
        assert!(d.reserved_token_budget > 0);
        assert!(d.cost_ceiling_usd.is_some());
        // D5: Pack
        assert_eq!(d.pack_id.as_deref(), Some("assistant"));
        // D6: Tool Set
        assert!(!d.tool_set.is_empty());
        // D7: A2A
        assert!(d.a2a_target.is_none());
        // D8: Context Mode
        assert_eq!(d.context_mode, ContextMode::Summarized);
        // D9: Fallback Chain
        assert_eq!(d.fallback_chain.len(), 1);
        assert_eq!(d.fallback_chain[0].provider_id, "codex");

        // Trace ID
        assert!(!d.trace_id.is_empty());
    }
}
