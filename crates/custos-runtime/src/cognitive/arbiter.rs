use crate::{RouteDecision, RoutingPolicy, RoutingSignals};
use custos_domain::DomainError;

/// Routes work to a low-cost or deliberate reasoning tier using explicit signals.
pub struct CognitiveArbiter {
    policy: RoutingPolicy,
}

impl CognitiveArbiter {
    pub fn new() -> Self {
        Self {
            policy: RoutingPolicy::default(),
        }
    }

    pub fn with_policy(policy: RoutingPolicy) -> Self {
        Self { policy }
    }

    pub fn route(&self, signals: &RoutingSignals) -> Result<RouteDecision, DomainError> {
        self.policy.decide(signals)
    }
}
impl Default for CognitiveArbiter {
    fn default() -> Self {
        Self::new()
    }
}
