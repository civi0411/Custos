//! Sovereign Worker Step Executor (PR 4 - Gate 5 / Execution Spine)
//!
//! Routes DAG node execution steps to ModelPort (direct LLM turn),
//! AgentRuntimePort (governed harness session), or SandboxPort (tool execution).
//! Retains full backward compatibility with simulated OI domain benchmark nodes.

use async_trait::async_trait;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::SandboxPort;
use custos_domain::oi::{WorkerResult, WorkerStatus};
use custos_domain::{new_id, ActionIntent, DomainError, RiskLevel};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelPort;
use std::sync::Arc;

use super::graph_runtime::StepExecutor;

pub struct WorkerExecutor {
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    harness: Option<Arc<dyn AgentRuntimePort>>,
    sandbox: Option<Arc<dyn SandboxPort>>,
}

impl WorkerExecutor {
    pub fn new() -> Self {
        Self {
            kernel: None,
            model: None,
            harness: None,
            sandbox: None,
        }
    }

    pub fn with_kernel(mut self, kernel: Arc<dyn KernelPort>) -> Self {
        self.kernel = Some(kernel);
        self
    }

    pub fn with_model(mut self, model: Arc<dyn ModelPort>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_harness(mut self, harness: Arc<dyn AgentRuntimePort>) -> Self {
        self.harness = Some(harness);
        self
    }

    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxPort>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    pub fn kernel(&self) -> Option<&Arc<dyn KernelPort>> {
        self.kernel.as_ref()
    }

    pub fn model(&self) -> Option<&Arc<dyn ModelPort>> {
        self.model.as_ref()
    }

    pub fn harness(&self) -> Option<&Arc<dyn AgentRuntimePort>> {
        self.harness.as_ref()
    }

    pub fn sandbox(&self) -> Option<&Arc<dyn SandboxPort>> {
        self.sandbox.as_ref()
    }
}

impl Default for WorkerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for WorkerExecutor {
    async fn execute_step(
        &self,
        node_id: &str,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        match action_type {
            "model_turn" | "model" | "llm" => {
                let model = self.model.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires ModelPort, but none was configured",
                        node_id, action_type
                    ))
                })?;

                let prompt = inputs
                    .get("prompt")
                    .or_else(|| inputs.get("instruction"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Execute model turn");

                let req = ProviderRequest::new(
                    new_id("req"),
                    node_id,
                    1,
                    prompt.to_string(),
                    model.provider_id(),
                );

                let resp = model.generate(&req).await?;
                serde_json::to_value(&resp)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            "tool_call" | "sandbox" => {
                let sandbox = self.sandbox.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires SandboxPort, but none was configured",
                        node_id, action_type
                    ))
                })?;

                let action_name = inputs
                    .get("tool")
                    .or_else(|| inputs.get("action_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("developer__bash");

                let params = inputs
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                let mut action = ActionIntent::new(
                    new_id("act"),
                    action_name.to_string(),
                    node_id.to_string(),
                    params,
                    RiskLevel::Low,
                );
                action.assurance = custos_domain::Assurance::CustosMediated;

                let permit = if let Some(ref kernel) = self.kernel {
                    let p = kernel.request_permit(node_id, &action, "worker_executor").await?;
                    kernel.consume_permit(&p.id, &action).await?
                } else {
                    return Err(DomainError::Unauthorized(format!(
                        "Cannot execute tool call on node '{}': KernelPort required to evaluate policy and mint ExecutionPermit",
                        node_id
                    )));
                };

                let receipt = sandbox.execute(&action, &permit).await?;
                serde_json::to_value(&receipt)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            "harness_run" | "agent_turn" => {
                let harness = self.harness.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires AgentRuntimePort, but none was configured",
                        node_id, action_type
                    ))
                })?;

                let wrun = custos_domain::WorkerRun::new(
                    node_id.to_string(),
                    harness.profile().harness_id.clone(),
                    3,
                );

                let context_pack = custos_domain::ContextPack::new(
                    new_id("ctx"),
                    Vec::new(),
                    0,
                    "sha256:empty".into(),
                );

                let intents = harness.execute_turn(&wrun, &context_pack).await?;
                serde_json::to_value(&intents)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            "execute_worker" => {
                // Backward-compatible simulation & OI domain topology execution
                let role = inputs
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let harness = inputs
                    .get("harness_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let result = WorkerResult {
                    packet_id: custos_domain::new_id("wpk"),
                    status: WorkerStatus::Sufficient,
                    payload: format!(
                        "Simulated execution of node {} by {} ({})",
                        node_id, role, harness
                    ),
                    tokens_used: 100,
                    cost_usd: 0.01,
                    evidence_references: vec![],
                };

                serde_json::to_value(result).map_err(|e| DomainError::Validation(e.to_string()))
            }

            other => Err(DomainError::Validation(format!(
                "Unsupported action type '{}' for node '{}'",
                other, node_id
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_provider::request::ModelResponse;

    struct TestMockModel;

    #[async_trait]
    impl ModelPort for TestMockModel {
        fn provider_id(&self) -> &str {
            "test_mock"
        }

        async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            Ok(ModelResponse::text(
                format!("Mock generated response for {}", req.task_id),
                "test_mock",
                30,
            ))
        }
    }

    #[tokio::test]
    async fn test_worker_executor_execute_worker_compat() {
        let executor = WorkerExecutor::new();
        let inputs = serde_json::json!({
            "role": "engineer",
            "harness_id": "claude_code"
        });

        let val = executor
            .execute_step("node_1", "execute_worker", &inputs)
            .await
            .unwrap();

        let res: WorkerResult = serde_json::from_value(val).unwrap();
        assert_eq!(res.status, WorkerStatus::Sufficient);
        assert!(res.payload.contains("node_1 by engineer (claude_code)"));
    }

    #[tokio::test]
    async fn test_worker_executor_model_routing() {
        let provider = Arc::new(TestMockModel);
        let executor = WorkerExecutor::new().with_model(provider);

        let inputs = serde_json::json!({
            "prompt": "Analyze repository architecture"
        });

        let val = executor
            .execute_step("node_model", "model_turn", &inputs)
            .await
            .expect("Model turn execution should succeed");

        assert!(val.get("content").is_some());
    }

    #[tokio::test]
    async fn test_worker_executor_missing_port_error() {
        let executor = WorkerExecutor::new();
        let inputs = serde_json::json!({});

        let err = executor
            .execute_step("node_err", "model_turn", &inputs)
            .await
            .expect_err("Should error when model port is missing");

        match err {
            DomainError::Validation(msg) => {
                assert!(msg.contains("requires ModelPort"));
            }
            other => panic!("Expected DomainError::Validation, got {:?}", other),
        }
    }

    struct DummySandbox;
    #[async_trait]
    impl SandboxPort for DummySandbox {
        fn driver_id(&self) -> &str {
            "dummy"
        }
        async fn execute(
            &self,
            _intent: &ActionIntent,
            _permit: &custos_domain::Permit,
        ) -> Result<custos_domain::ExecutionReceipt, DomainError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn test_worker_executor_tool_call_without_kernel_unauthorized() {
        let executor = WorkerExecutor::new().with_sandbox(Arc::new(DummySandbox));
        let inputs = serde_json::json!({ "tool": "developer__bash" });

        let err = executor
            .execute_step("node_tool", "tool_call", &inputs)
            .await
            .expect_err("Tool call without KernelPort must be rejected as Unauthorized");

        assert!(matches!(err, DomainError::Unauthorized(_)));
    }
}
