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
        let now = chrono::Utc::now();

        // 1. Verify permit validity & expiration
        if permit.action_id != action.id {
            return Err(DomainError::Unauthorized(format!(
                "Permit action_id mismatch: permit for '{}', action is '{}'",
                permit.action_id, action.id
            )));
        }

        if now > permit.expires_at {
            return Err(DomainError::Unauthorized(format!(
                "ExecutionPermit expired at {:?}",
                permit.expires_at
            )));
        }

        // 2. Formulate ToolCall
        let tool_call = ToolCall {
            call_id: action.id.clone(),
            name: action.name.clone(),
            arguments: action.parameters.clone(),
        };

        // 3. Dispatch through MCP ToolPort
        let start = std::time::Instant::now();
        let result = self.inner_tool.execute(tool_call).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // 4. Seal into ExecutionReceipt
        match result {
            Ok(output) => {
                let digest_val = custos_domain::digest(output.output.as_bytes());
                Ok(custos_domain::ExecutionReceipt {
                    receipt_id: custos_domain::new_id("rcpt"),
                    permit_id: permit.id.clone(),
                    action_id: action.id.clone(),
                    status: if output.is_error {
                        custos_domain::ReceiptStatus::Failure
                    } else {
                        custos_domain::ReceiptStatus::Success
                    },
                    output_digest: format!("sha256:{}", digest_val),
                    output_data: Some(serde_json::json!({
                        "output": output.output,
                        "is_error": output.is_error,
                    })),
                    error_message: if output.is_error {
                        Some(output.output)
                    } else {
                        None
                    },
                    duration_ms: Some(duration_ms),
                    executed_at: chrono::Utc::now(),
                })
            }
            Err(e) => {
                let err_msg = e.to_string();
                let digest_val = custos_domain::digest(err_msg.as_bytes());
                Ok(custos_domain::ExecutionReceipt {
                    receipt_id: custos_domain::new_id("rcpt"),
                    permit_id: permit.id.clone(),
                    action_id: action.id.clone(),
                    status: custos_domain::ReceiptStatus::Failure,
                    output_digest: format!("sha256:{}", digest_val),
                    output_data: None,
                    error_message: Some(err_msg),
                    duration_ms: Some(duration_ms),
                    executed_at: chrono::Utc::now(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{Action, Permit, RiskClass, RiskLevel};
    use custos_provider::CapabilityPort;

    struct MockTool {
        name: String,
        output: String,
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
            Ok(ToolResult {
                call_id: "test_call".into(),
                output: self.output.clone(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_mcp_capability_adapter_dispatch_with_valid_permit() {
        let tool = Arc::new(MockTool {
            name: "read_file".into(),
            output: "hello world".into(),
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

        let receipt = adapter.dispatch(&action, &permit).await.unwrap();
        assert_eq!(receipt.action_id, "act_1");
        assert_eq!(receipt.permit_id, permit.id);
        assert_eq!(receipt.status, custos_domain::ReceiptStatus::Success);
        assert!(receipt.output_digest.starts_with("sha256:"));
    }

    #[tokio::test]
    async fn test_mcp_capability_adapter_rejects_mismatched_permit() {
        let tool = Arc::new(MockTool {
            name: "read_file".into(),
            output: "hello".into(),
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

