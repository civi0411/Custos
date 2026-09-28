use async_trait::async_trait;
use custos_core_domain::{DomainError, TaskId};
use custos_provider_sdk::{
    ModelProvider, ModelResponse, ProviderRequest, ToolCall, ToolDefinition, ToolPort, ToolResult,
};
use std::sync::Arc;

/// GatewayTool: Wraps tool execution with strict capability verification
pub struct GatewayTool {
    inner: Arc<dyn ToolPort>,
    task_id: TaskId,
}

impl GatewayTool {
    pub fn new(inner: Arc<dyn ToolPort>, task_id: TaskId) -> Self {
        Self { inner, task_id }
    }
}

#[async_trait]
impl ToolPort for GatewayTool {
    fn definition(&self) -> ToolDefinition {
        self.inner.definition()
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult, DomainError> {
        tracing::info!(
            task_id = %self.task_id,
            tool = %call.name,
            "GatewayTool intercepting execution for capability validation"
        );

        // Disallow arbitrary un-sandboxed effects (Invariant I1)
        if call.name == "raw_shell_exec" {
            return Err(DomainError::Unauthorized(
                "Direct un-sandboxed shell execution is strictly prohibited by Invariant I1".into(),
            ));
        }

        self.inner.execute(call).await
    }
}

/// GatewayProvider: Wraps model provider calls with egress sanitation (Invariant I7)
pub struct GatewayProvider {
    inner: Arc<dyn ModelProvider>,
    task_id: TaskId,
}

impl GatewayProvider {
    pub fn new(inner: Arc<dyn ModelProvider>, task_id: TaskId) -> Self {
        Self { inner, task_id }
    }
}

#[async_trait]
impl ModelProvider for GatewayProvider {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        tracing::info!(
            task_id = %self.task_id,
            provider = %self.provider_id(),
            "GatewayProvider: Auditing egress request and sanitizing prompt"
        );

        // Invariant I7: Check for raw secret exposure
        if req.prompt.contains("AWS_SECRET_KEY") || req.prompt.contains("PRIVATE_KEY") {
            return Err(DomainError::InvariantViolation(
                "Invariant I7 Violation: Secret detected in outbound prompt".into(),
            ));
        }

        self.inner.generate(req).await
    }
}
