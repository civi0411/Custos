use async_trait::async_trait;
use custos_core_domain::{Action, DomainError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub evidence: Option<String>,
}

#[async_trait]
pub trait ToolGate: Send + Sync {
    async fn dispatch(&self, action: &Action) -> Result<ExecutionResult, DomainError>;
}
