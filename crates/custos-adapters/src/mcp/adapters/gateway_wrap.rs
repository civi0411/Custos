use async_trait::async_trait;
use custos_domain::{DomainError, TaskId};
use custos_provider::{
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

        // This wrapper has no permit or durable dispatch claim. Forwarding any
        // tool call here would bypass the authority boundary, regardless of name.
        Err(DomainError::Unauthorized(format!(
            "Tool '{}' requires a durable Custos capability dispatch claim",
            call.name
        )))
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

/// McpCapabilityAdapter: Dispatches actions to MCP tools governed by ExecutionPermits
pub struct McpCapabilityAdapter {
    capability_name: String,
    inner_tool: Arc<dyn ToolPort>,
}

impl McpCapabilityAdapter {
    pub fn new(capability_name: impl Into<String>, inner_tool: Arc<dyn ToolPort>) -> Self {
        Self {
            capability_name: capability_name.into(),
            inner_tool,
        }
    }
}

#[async_trait]
impl custos_provider::CapabilityPort for McpCapabilityAdapter {
    fn capability_name(&self) -> &str {
        &self.capability_name
    }

    async fn dispatch(
        &self,
        action: &custos_domain::ActionIntent,
        permit: &custos_domain::ExecutionPermit,
    ) -> Result<custos_domain::ExecutionReceipt, DomainError> {
        // A copied Permit can be replayed after a crash. Until the adapter is
        // supplied with an atomically claimed, durable attempt, deny dispatch.
        let registered_tool = self.inner_tool.definition().name;
        Err(DomainError::Unauthorized(format!(
            "MCP dispatch of '{}' through '{}' requires a durable, single-use attempt claim (permit '{}')",
            action.name, registered_tool, permit.id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{Action, Permit, RiskClass, RiskLevel};
    use custos_provider::CapabilityPort;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockTool {
        name: String,
        output: String,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolPort for MockTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: self.name.clone(),
                description: "mock tool".to_string(),
                parameters: serde_json::json!({}),
            }
        }

        async fn execute(&self, _call: ToolCall) -> Result<ToolResult, DomainError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                call_id: "test_call".into(),
                output: self.output.clone(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_gateway_tool_never_forwards_without_claim() {
        let calls = Arc::new(AtomicUsize::new(0));
        let tool = Arc::new(MockTool {
            name: "read_file".into(),
            output: "secret".into(),
            calls: calls.clone(),
        });
        let gateway = GatewayTool::new(tool, "task_1".into());
        let result = gateway
            .execute(ToolCall {
                call_id: "call_1".into(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "secret.txt"}),
            })
            .await;

        assert!(matches!(result, Err(DomainError::Unauthorized(_))));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_mcp_capability_adapter_denies_unclaimed_permit() {
        let tool = Arc::new(MockTool {
            name: "read_file".into(),
            output: "hello world".into(),
            calls: Arc::new(AtomicUsize::new(0)),
        });
        let adapter = McpCapabilityAdapter::new("fs_read", tool);

        let action = Action::new(
            "act_1".into(),
            "read_file".into(),
            "foo.txt".into(),
            serde_json::json!({"path": "foo.txt"}),
            RiskLevel::Low,
        );

        let permit = Permit::new(
            "task_1".into(),
            "act_1".into(),
            "fs_read".into(),
            None,
            RiskClass::Low,
            300,
        );

        let result = adapter.dispatch(&action, &permit).await;
        assert!(matches!(result, Err(DomainError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn test_mcp_capability_adapter_rejects_mismatched_permit() {
        let tool = Arc::new(MockTool {
            name: "read_file".into(),
            output: "hello".into(),
            calls: Arc::new(AtomicUsize::new(0)),
        });
        let adapter = McpCapabilityAdapter::new("fs_read", tool);

        let action = Action::new(
            "act_actual".into(),
            "read_file".into(),
            "foo.txt".into(),
            serde_json::json!({}),
            RiskLevel::Low,
        );

        let permit = Permit::new(
            "task_1".into(),
            "act_different".into(),
            "fs_read".into(),
            None,
            RiskClass::Low,
            300,
        );

        let res = adapter.dispatch(&action, &permit).await;
        assert!(res.is_err());
    }
}
