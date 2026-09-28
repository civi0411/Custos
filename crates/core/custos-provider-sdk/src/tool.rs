use async_trait::async_trait;
use custos_core_domain::DomainError;
use serde::{Deserialize, Serialize};

pub use crate::events::ToolCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[async_trait]
pub trait ToolPort: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, call: ToolCall) -> Result<ToolResult, DomainError>;
}
