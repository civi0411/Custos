use crate::traits::{Policy, PolicyDecision};
use async_trait::async_trait;
use custos_core_domain::{Action, DomainError, RiskLevel};

pub struct DemoPolicy;

#[async_trait]
impl Policy for DemoPolicy {
    async fn evaluate(&self, action: &Action) -> Result<PolicyDecision, DomainError> {
        match action.risk_level {
            RiskLevel::Low => Ok(PolicyDecision::Allowed),
            RiskLevel::Medium => Ok(PolicyDecision::Allowed),
            RiskLevel::High => Ok(PolicyDecision::NeedsApproval {
                prompt: format!(
                    "High-risk action '{}' requires explicit confirmation",
                    action.name
                ),
            }),
            RiskLevel::Critical => Ok(PolicyDecision::Denied {
                reason: format!("Critical action '{}' is blocked by policy", action.name),
            }),
        }
    }
}
