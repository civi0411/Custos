use crate::traits::{ExecutionResult, ToolGate};
use async_trait::async_trait;
use custos_core_domain::{Action, DomainError};
use custos_policy_engine::{Policy, PolicyDecision};
use std::sync::Arc;

pub struct DeterministicGate {
    policy: Arc<dyn Policy>,
}

impl DeterministicGate {
    pub fn new(policy: Arc<dyn Policy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl ToolGate for DeterministicGate {
    async fn dispatch(&self, action: &Action) -> Result<ExecutionResult, DomainError> {
        let decision = self.policy.evaluate(action).await?;
        match decision {
            PolicyDecision::Allowed => {
                // In full implementation, dispatch to actual adapter
                Ok(ExecutionResult {
                    success: true,
                    output: serde_json::json!({"status": "executed", "action": action.name}),
                    evidence: Some("deterministic_hash_proof".into()),
                })
            }
            PolicyDecision::NeedsApproval { prompt } => Err(DomainError::Validation(format!(
                "Approval required: {prompt}"
            ))),
            PolicyDecision::Denied { reason } => {
                Err(DomainError::Validation(format!("Action denied: {reason}")))
            }
        }
    }
}
