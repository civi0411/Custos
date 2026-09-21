use async_trait::async_trait;
use custos_core_domain::{Action, DomainError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    Denied { reason: String },
    NeedsApproval { prompt: String },
}

#[async_trait]
pub trait Policy: Send + Sync {
    async fn evaluate(&self, action: &Action) -> Result<PolicyDecision, DomainError>;
}
