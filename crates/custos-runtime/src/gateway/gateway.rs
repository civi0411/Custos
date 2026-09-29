use crate::gateway::budget::{BudgetCheck, BudgetGuard, BudgetTracker};
use crate::gateway::dispatch::{A2ADispatcher, ExternalDispatcher, HumanDispatcher, LocalDispatcher};
use crate::gateway::observability::{GatewayMetrics, GatewayTracer};
use crate::gateway::policy::SessionQuotaLimiter;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::cognitive::{HumanGate, ReasoningTier, RouteDecision, RoutingPolicy, RoutingSignals};
use custos_core_domain::{SessionId, TaskId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub session_id: SessionId,
    pub task_id: Option<TaskId>,
    pub signals: RoutingSignals,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub decision: RouteDecision,
    pub result: String,
    pub tokens_used: u64,
    pub cost_usd: f32,
    pub latency_ms: u64,
}

#[async_trait]
pub trait AgentGateway: Send + Sync {
    async fn route_and_execute(&self, request: RouteRequest) -> Result<GatewayResponse>;
    async fn check_budget(
        &self,
        session_id: &SessionId,
        task_id: Option<&TaskId>,
        estimated_tokens: u64,
        estimated_cost_usd: f32,
    ) -> Result<BudgetCheck>;
}

pub struct CustosGateway {
    router: RoutingPolicy,
    budget_guard: BudgetGuard,
    budget_tracker: BudgetTracker,
    quota_limiter: SessionQuotaLimiter,
    human_gate: HumanGate,
    human_dispatcher: HumanDispatcher,
    metrics: Arc<GatewayMetrics>,
    tracer: GatewayTracer,
}

impl Default for CustosGateway {
    fn default() -> Self {
        let human_gate = HumanGate::new();
        let human_dispatcher = HumanDispatcher::new(human_gate.clone());
        Self {
            router: RoutingPolicy::default(),
            budget_guard: BudgetGuard::default(),
            budget_tracker: BudgetTracker::new(),
            quota_limiter: SessionQuotaLimiter::new(120), // 120 reqs/min
            human_gate,
            human_dispatcher,
            metrics: Arc::new(GatewayMetrics::new()),
            tracer: GatewayTracer::new(),
        }
    }
}

impl CustosGateway {
    pub fn new(
        router: RoutingPolicy,
        budget_guard: BudgetGuard,
        quota_limiter: SessionQuotaLimiter,
        human_gate: HumanGate,
    ) -> Self {
        let human_dispatcher = HumanDispatcher::new(human_gate.clone());
        Self {
            router,
            budget_guard,
            budget_tracker: BudgetTracker::new(),
            quota_limiter,
            human_gate,
            human_dispatcher,
            metrics: Arc::new(GatewayMetrics::new()),
            tracer: GatewayTracer::new(),
        }
    }

    pub fn metrics(&self) -> Arc<GatewayMetrics> {
        self.metrics.clone()
    }

    pub fn tracer(&self) -> GatewayTracer {
        self.tracer.clone()
    }

    pub fn human_gate(&self) -> &HumanGate {
        &self.human_gate
    }
}

#[async_trait]
impl AgentGateway for CustosGateway {
    async fn check_budget(
        &self,
        session_id: &SessionId,
        task_id: Option<&TaskId>,
        estimated_tokens: u64,
        estimated_cost_usd: f32,
    ) -> Result<BudgetCheck> {
        let session_usage = self
            .budget_tracker
            .get_session_usage(&session_id.to_string())
            .await;
        let task_usage = if let Some(t) = task_id {
            self.budget_tracker.get_task_usage(t).await
        } else {
            Default::default()
        };

        self.budget_guard.check(
            task_usage.total_tokens.max(session_usage.total_tokens),
            task_usage.total_cost_usd.max(session_usage.total_cost_usd),
            estimated_tokens,
            estimated_cost_usd,
        )
    }

    async fn route_and_execute(&self, request: RouteRequest) -> Result<GatewayResponse> {
        let start = Instant::now();

        // 1. Quota Check
        if !self
            .quota_limiter
            .check_and_record(&request.session_id.to_string())
            .await
        {
            return Err(anyhow!(
                "Rate limit exceeded for session: {}",
                request.session_id
            ));
        }

        // 2. 9Router Decision
        let decision = self.router.decide(&request.signals)?;
        let tier_name = match decision.tier {
            ReasoningTier::TierZero => "tier_zero",
            ReasoningTier::SystemOne => "system_one",
            ReasoningTier::SystemTwo => "system_two",
            ReasoningTier::Abstain => "abstain",
        };
        self.metrics.record_tier(tier_name);

        if decision.tier == ReasoningTier::Abstain {
            return Err(anyhow!(
                "Gateway abstained from routing: {}",
                decision.rationale
            ));
        }

        // 3. Budget Check
        let est_tokens = decision.reserved_token_budget;
        let est_cost = decision.cost_ceiling_usd.unwrap_or(0.0);
        let budget_check = self
            .check_budget(
                &request.session_id,
                request.task_id.as_ref(),
                est_tokens,
                est_cost,
            )
            .await?;

        if !budget_check.allowed {
            return Err(anyhow!(
                "Budget check rejected execution: {}",
                budget_check.reason.unwrap_or_default()
            ));
        }

        // 4. Human Approval Gate
        if decision.require_human_approval {
            self.metrics.record_human_gate();
            let outcome = self
                .human_dispatcher
                .dispatch(
                    request.session_id.clone(),
                    request.task_id.clone(),
                    format!("Approve execution for prompt: {}", request.prompt),
                    std::time::Duration::from_secs(60),
                )
                .await?;

            if !matches!(outcome, crate::cognitive::HumanGateOutcome::Approved(_)) {
                return Err(anyhow!("Execution denied by human gate: {:?}", outcome));
            }
        }

        // 5. Tier Dispatch
        let result_text = if decision.a2a_target.is_some() {
            A2ADispatcher::dispatch(&decision, &request.prompt).await?
        } else if decision.provider_id == "deterministic" || decision.provider_id == "local" {
            LocalDispatcher::dispatch(&decision, &request.prompt).await?
        } else {
            ExternalDispatcher::dispatch(&decision, &request.prompt).await?
        };

        let latency_ms = start.elapsed().as_millis() as u64;
        let tokens_used = decision.reserved_token_budget.min(256); // Mocked token actuals
        let cost_usd = decision.cost_ceiling_usd.unwrap_or(0.0) * 0.5;

        // 6. Record Budget & Observability
        self.budget_tracker
            .record(
                request.task_id.as_deref(),
                &request.session_id.to_string(),
                tokens_used,
                cost_usd,
            )
            .await;

        self.metrics.record_latency(tier_name, latency_ms).await;

        self.tracer
            .record_span(
                decision.trace_id.clone(),
                request.session_id.to_string(),
                request.task_id.map(|t| t.to_string()),
                tier_name.to_string(),
                decision.provider_id.clone(),
                decision.model_id.clone(),
                latency_ms,
                tokens_used,
                cost_usd,
            )
            .await;

        Ok(GatewayResponse {
            decision,
            result: result_text,
            tokens_used,
            cost_usd,
            latency_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::RiskLevel;

    fn sample_signals() -> RoutingSignals {
        RoutingSignals {
            deterministic_solution_available: false,
            ambiguity: 0.1,
            uncertainty: 0.1,
            context_sufficient: true,
            reversible: true,
            risk: RiskLevel::Low,
            evidence_required: false,
            budget_available: true,
            system_one_success_probability: 0.85,
            system_two_success_probability: 0.90,
            system_one_tokens: 1_000,
            system_two_tokens: 8_000,
            token_cost_weight: 0.000_01,
        }
    }

    #[tokio::test]
    async fn test_gateway_route_and_execute_system_one() {
        let gateway = CustosGateway::default();
        let session_id = SessionId::generate();

        let req = RouteRequest {
            session_id: session_id.clone(),
            task_id: None,
            signals: sample_signals(),
            prompt: "Refactor function xyz".into(),
        };

        let res = gateway.route_and_execute(req).await.unwrap();
        assert_eq!(res.decision.tier, ReasoningTier::SystemOne);
        assert!(!res.result.is_empty());
        assert_eq!(
            gateway
                .metrics()
                .total_system_one
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn test_gateway_budget_check() {
        let gateway = CustosGateway::default();
        let session_id = SessionId::generate();

        let check = gateway
            .check_budget(&session_id, None, 500, 0.01)
            .await
            .unwrap();
        assert!(check.allowed);
    }
}
