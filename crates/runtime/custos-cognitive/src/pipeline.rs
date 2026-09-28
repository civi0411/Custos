//! End-to-End Cognitive 9Router Pipeline
//!
//! Orchestrates the full lifecycle:
//! Request → SignalExtractor → 9Router Arbiter → Human Gate → ProviderSelector / A2A Fallback → Execution.

use crate::arbiter::CognitiveArbiter;
use crate::human_gate::{HumanGate, HumanGateOutcome};
use crate::provider_selector::TierProviderMap;
use crate::registry::ProviderRegistry;
use crate::routing::{ReasoningTier, RouteDecision};
use crate::signal_extractor::SignalExtractor;
use async_trait::async_trait;
use custos_core_domain::{DomainError, SessionId};
use custos_provider_sdk::request::ProviderRequest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

#[async_trait]
pub trait A2ADelegationPort: Send + Sync {
    async fn delegate(&self, task: &str, budget_usd: f32) -> Result<String, DomainError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CognitiveExecutionResult {
    Executed {
        provider_id: String,
        model_id: String,
        response: String,
        tier: ReasoningTier,
        route_decision: RouteDecision,
    },
    A2ADelegated {
        peer_response: String,
        tier: ReasoningTier,
        route_decision: RouteDecision,
    },
    BlockedByHumanGate {
        reason: String,
        route_decision: RouteDecision,
    },
    Abstained {
        reason: String,
        route_decision: RouteDecision,
    },
}

pub struct CognitivePipeline {
    signal_extractor: SignalExtractor,
    arbiter: CognitiveArbiter,
    tier_prefs: TierProviderMap,
    provider_registry: Arc<RwLock<ProviderRegistry>>,
    human_gate: Arc<HumanGate>,
    a2a_port: Option<Arc<dyn A2ADelegationPort>>,
    human_gate_timeout: Duration,
}

impl CognitivePipeline {
    pub fn new(
        provider_registry: Arc<RwLock<ProviderRegistry>>,
        human_gate: Arc<HumanGate>,
    ) -> Self {
        Self {
            signal_extractor: SignalExtractor::new(),
            arbiter: CognitiveArbiter::new(),
            tier_prefs: TierProviderMap::default(),
            provider_registry,
            human_gate,
            a2a_port: None,
            human_gate_timeout: Duration::from_millis(50),
        }
    }

    pub fn with_a2a_port(mut self, port: Arc<dyn A2ADelegationPort>) -> Self {
        self.a2a_port = Some(port);
        self
    }

    pub fn with_tier_prefs(mut self, prefs: TierProviderMap) -> Self {
        self.tier_prefs = prefs;
        self
    }

    pub fn with_human_gate_timeout(mut self, timeout: Duration) -> Self {
        self.human_gate_timeout = timeout;
        self
    }

    /// Process an incoming user request through the full 9Router cognitive pipeline
    pub async fn process_request(
        &self,
        request_text: &str,
        user_budget_usd: f32,
    ) -> Result<CognitiveExecutionResult, DomainError> {
        // Step 1: Extract 9-dimension routing signals
        let budget_available = user_budget_usd > 0.0;
        let signals = self
            .signal_extractor
            .extract(request_text, false, budget_available);

        // Step 2: Cognitive Arbiter decisions (Resolve-Delegate-Check protocol)
        let decision = self.arbiter.route(&signals)?;

        // Step 3: Handle Abstain
        if decision.tier == ReasoningTier::Abstain {
            return Ok(CognitiveExecutionResult::Abstained {
                reason: decision.rationale.clone(),
                route_decision: decision,
            });
        }

        // Step 4: Human Gate check for high-risk actions
        if decision.require_human_approval {
            let session_id = SessionId::generate();
            let gate_res = self
                .human_gate
                .request_and_wait(
                    session_id,
                    None,
                    format!("Approval required for high risk action: {}", request_text),
                    None,
                    self.human_gate_timeout,
                )
                .await;

            match gate_res {
                Ok(HumanGateOutcome::Approved(_)) => {
                    // Approved by human, proceed with execution
                }
                Ok(HumanGateOutcome::Rejected { reason }) => {
                    return Ok(CognitiveExecutionResult::BlockedByHumanGate {
                        reason: format!("Human rejected execution: {}", reason),
                        route_decision: decision,
                    });
                }
                _ => {
                    return Ok(CognitiveExecutionResult::BlockedByHumanGate {
                        reason: "Human approval timed out or was not granted".into(),
                        route_decision: decision,
                    });
                }
            }
        }

        // Step 5: Fast path for TierZero (deterministic)
        if decision.tier == ReasoningTier::TierZero {
            return Ok(CognitiveExecutionResult::Executed {
                provider_id: "deterministic".into(),
                model_id: "system-0-exact".into(),
                response: format!("Deterministic resolution for: {}", request_text),
                tier: ReasoningTier::TierZero,
                route_decision: decision,
            });
        }

        // Step 6: Select provider from registry
        let selection_result = {
            let registry = self.provider_registry.read().await;
            registry.select_for_tier(decision.tier, &self.tier_prefs)
        };

        if let Some((provider, pref)) = selection_result {
            let req = ProviderRequest::new(
                Uuid::new_v4().to_string(),
                "cognitive_task",
                1,
                request_text,
                &pref.model_id,
            );
            let start = std::time::Instant::now();
            let response = provider.generate(&req).await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            match response {
                Ok(resp) => {
                    let mut registry = self.provider_registry.write().await;
                    registry.record_success(provider.provider_id(), elapsed_ms);

                    return Ok(CognitiveExecutionResult::Executed {
                        provider_id: provider.provider_id().to_string(),
                        model_id: pref.model_id.clone(),
                        response: resp.content,
                        tier: decision.tier,
                        route_decision: decision,
                    });
                }
                Err(err) => {
                    let mut registry = self.provider_registry.write().await;
                    registry.record_failure(provider.provider_id(), &err.to_string());
                    // Fallthrough to A2A fallback if available
                }
            }
        }

        // Step 7: A2A Fallback when local provider is unavailable
        if let Some(a2a) = &self.a2a_port {
            let peer_res = a2a.delegate(request_text, user_budget_usd).await?;
            return Ok(CognitiveExecutionResult::A2ADelegated {
                peer_response: peer_res,
                tier: decision.tier,
                route_decision: decision,
            });
        }

        Err(DomainError::NotFound {
            kind: "ModelProvider".into(),
            id: format!(
                "No operable provider or A2A peer available for tier {:?}",
                decision.tier
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_provider_sdk::port::ModelProvider;
    use custos_provider_sdk::request::ModelResponse;

    struct TestProvider {
        id: String,
        should_fail: bool,
    }

    #[async_trait]
    impl ModelProvider for TestProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            if self.should_fail {
                Err(DomainError::Validation("Provider upstream timeout".into()))
            } else {
                Ok(ModelResponse::text("LLM output response", "test-model", 25))
            }
        }
    }

    struct MockA2APeer;

    #[async_trait]
    impl A2ADelegationPort for MockA2APeer {
        async fn delegate(&self, task: &str, _budget: f32) -> Result<String, DomainError> {
            Ok(format!("A2A peer processed task: {}", task))
        }
    }

    #[tokio::test]
    async fn test_pipeline_system_zero_deterministic() {
        let registry = Arc::new(RwLock::new(ProviderRegistry::new()));
        let gate = Arc::new(HumanGate::new());
        let pipeline = CognitivePipeline::new(registry, gate);

        let res = pipeline
            .process_request("calculate 2 + 2", 10.0)
            .await
            .unwrap();

        match res {
            CognitiveExecutionResult::Executed {
                tier, provider_id, ..
            } => {
                assert_eq!(tier, ReasoningTier::TierZero);
                assert_eq!(provider_id, "deterministic");
            }
            _ => panic!("Expected Executed result for TierZero"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_system_one_standard_llm() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(TestProvider {
            id: "claude".into(),
            should_fail: false,
        }));

        let registry = Arc::new(RwLock::new(reg));
        let gate = Arc::new(HumanGate::new());
        let pipeline = CognitivePipeline::new(registry, gate);

        let res = pipeline
            .process_request("Summarize the README file in 2 sentences", 10.0)
            .await
            .unwrap();

        match res {
            CognitiveExecutionResult::Executed {
                tier,
                provider_id,
                response,
                ..
            } => {
                assert_eq!(tier, ReasoningTier::SystemOne);
                assert_eq!(provider_id, "claude");
                assert_eq!(response, "LLM output response");
            }
            _ => panic!("Expected SystemOne execution"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_high_risk_blocked_by_human_gate() {
        let registry = Arc::new(RwLock::new(ProviderRegistry::new()));
        let gate = Arc::new(HumanGate::new());
        let pipeline = CognitivePipeline::new(registry, gate);

        // High risk query triggers approval requirement (will time out in test)
        let res = pipeline
            .process_request("Delete production database credentials immediately", 10.0)
            .await
            .unwrap();

        match res {
            CognitiveExecutionResult::BlockedByHumanGate {
                reason,
                route_decision,
            } => {
                assert!(route_decision.require_human_approval);
                assert!(reason.contains("Human approval timed out"));
            }
            _ => panic!("Expected BlockedByHumanGate for high risk mutation"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_a2a_fallback_when_local_provider_fails() {
        let mut reg = ProviderRegistry::new();
        // Provider will fail
        reg.register(Arc::new(TestProvider {
            id: "claude".into(),
            should_fail: true,
        }));

        let registry = Arc::new(RwLock::new(reg));
        let gate = Arc::new(HumanGate::new());
        let pipeline = CognitivePipeline::new(registry, gate).with_a2a_port(Arc::new(MockA2APeer));

        let res = pipeline
            .process_request("Analyze code repository architecture", 10.0)
            .await
            .unwrap();

        match res {
            CognitiveExecutionResult::A2ADelegated { peer_response, .. } => {
                assert!(peer_response.contains("A2A peer processed task"));
            }
            _ => panic!("Expected A2A delegation fallback"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_abstains_on_zero_budget() {
        let registry = Arc::new(RwLock::new(ProviderRegistry::new()));
        let gate = Arc::new(HumanGate::new());
        let pipeline = CognitivePipeline::new(registry, gate);

        let res = pipeline
            .process_request("Complex refactoring of 500 files", 0.0)
            .await
            .unwrap();

        match res {
            CognitiveExecutionResult::Abstained {
                reason,
                route_decision,
            } => {
                assert_eq!(route_decision.tier, ReasoningTier::Abstain);
                assert!(reason.contains("budget_exhausted"));
            }
            _ => panic!("Expected Abstained due to zero budget"),
        }
    }
}
