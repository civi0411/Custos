//! Policy Evaluation Contracts & Evaluators
//!
//! Evaluates policy rules before granting permits.

use async_trait::async_trait;
use custos_core_domain::{Action, DomainError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireApproval { reason: String },
    Deny { reason: String },
}

#[async_trait]
pub trait PolicyEvaluator: Send + Sync {
    async fn evaluate(&self, task_id: &str, action: &Action)
        -> Result<PolicyDecision, DomainError>;
}

/// Default policy evaluator based on action risk level
#[derive(Debug, Default)]
pub struct DefaultPolicyEvaluator;

#[async_trait]
impl PolicyEvaluator for DefaultPolicyEvaluator {
    async fn evaluate(
        &self,
        _task_id: &str,
        action: &Action,
    ) -> Result<PolicyDecision, DomainError> {
        match action.risk_level {
            custos_core_domain::RiskLevel::Low | custos_core_domain::RiskLevel::Medium => {
                Ok(PolicyDecision::Allow)
            }
            custos_core_domain::RiskLevel::High => Ok(PolicyDecision::RequireApproval {
                reason: format!(
                    "High-risk action '{}' requires explicit human confirmation",
                    action.name
                ),
            }),
            custos_core_domain::RiskLevel::Critical => Ok(PolicyDecision::Deny {
                reason: format!(
                    "Critical action '{}' is blocked by default policy",
                    action.name
                ),
            }),
        }
    }
}
