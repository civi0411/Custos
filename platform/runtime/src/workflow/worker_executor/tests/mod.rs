use super::*;
use crate::workflow::contracts::ExecutionContext;
use custos_domain::RiskLevel;
use custos_provider::request::ModelResponse;

mod outbox;

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
async fn unbound_worker_nodes_cannot_fabricate_results() {
    let executor = WorkerExecutor::new();
    let inputs = serde_json::json!({
        "role": "engineer",
        "harness_id": "claude_code"
    });
    let ctx = ExecutionContext::new("task_01", "run_01", "node_1", "worker_01");

    let error = executor
        .execute_step(&ctx, "execute_worker", &inputs)
        .await
        .unwrap_err();
    assert!(
        matches!(error, DomainError::Validation(message) if message.contains("Unsupported action type"))
    );
}

#[tokio::test]
async fn test_worker_executor_model_routing() {
    let provider = Arc::new(TestMockModel);
    let executor = WorkerExecutor::new().with_model(provider);
    let ctx = ExecutionContext::new("task_model", "run_model", "node_model", "worker_model");

    let inputs = serde_json::json!({
        "prompt": "Analyze repository architecture"
    });

    let val = executor
        .execute_step(&ctx, "model_turn", &inputs)
        .await
        .expect("Model turn execution should succeed");

    assert!(val.get("content").is_some());
}

#[tokio::test]
async fn test_worker_executor_missing_port_error() {
    let executor = WorkerExecutor::new();
    let inputs = serde_json::json!({});
    let ctx = ExecutionContext::new("task_err", "run_err", "node_err", "worker_err");

    let err = executor
        .execute_step(&ctx, "model_turn", &inputs)
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
    let ctx = ExecutionContext::new("task_tool", "run_tool", "node_tool", "worker_tool");

    let err = executor
        .execute_step(&ctx, "tool_call", &inputs)
        .await
        .expect_err("Tool call without KernelPort must be rejected as Unauthorized");

    assert!(matches!(err, DomainError::Unauthorized(_)));
}

#[tokio::test]
async fn dag_step_uses_task_id_not_node_id() {
    struct CapturingKernel {
        captured_task_id: std::sync::Mutex<Option<String>>,
    }

    #[async_trait]
    impl KernelPort for CapturingKernel {
        async fn create_task(
            &self,
            _title: String,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn create_task_with_contract(
            &self,
            _title: String,
            _contract: custos_domain::TaskContract,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn get_task(
            &self,
            _task_id: &str,
        ) -> Result<Option<custos_domain::Task>, DomainError> {
            unreachable!()
        }
        async fn transition_task(
            &self,
            _task_id: &str,
            _next: custos_domain::TaskStatus,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn request_permit(
            &self,
            task_id: &str,
            _intent: &ActionIntent,
            _actor: &str,
        ) -> Result<custos_domain::Permit, DomainError> {
            *self.captured_task_id.lock().unwrap() = Some(task_id.to_string());
            Ok(custos_domain::Permit::new(
                task_id.into(),
                "act_1".into(),
                "sandbox".into(),
                None,
                custos_domain::RiskClass::Low,
                60,
            ))
        }
        async fn consume_permit(
            &self,
            _permit_id: &str,
            intent: &ActionIntent,
        ) -> Result<custos_domain::Permit, DomainError> {
            Ok(custos_domain::Permit::new(
                intent
                    .task_id
                    .clone()
                    .unwrap_or_else(|| "task_default".into()),
                intent.id.clone(),
                "sandbox".into(),
                None,
                custos_domain::RiskClass::Low,
                60,
            ))
        }
        async fn verify_evidence(
            &self,
            _bundle: &custos_core::evidence::EvidenceBundle,
        ) -> Result<custos_domain::VerificationClaim, DomainError> {
            unreachable!()
        }
        async fn grant_capability(
            &self,
            _task_id: &str,
            _grantee: &str,
            _capability: &str,
        ) -> Result<custos_domain::Grant, DomainError> {
            unreachable!()
        }
        async fn find_active_grant(
            &self,
            _task_id: &str,
            _capability: &str,
        ) -> Result<Option<custos_domain::Grant>, DomainError> {
            Ok(None)
        }
        async fn complete_task(
            &self,
            _task_id: &str,
            _summary: String,
            _claims: Vec<custos_domain::VerificationClaim>,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
    }

    struct EchoSandbox;
    #[async_trait]
    impl SandboxPort for EchoSandbox {
        fn driver_id(&self) -> &str {
            "echo"
        }
        async fn execute(
            &self,
            action: &ActionIntent,
            permit: &custos_domain::Permit,
        ) -> Result<custos_domain::ExecutionReceipt, DomainError> {
            Ok(custos_domain::ExecutionReceipt {
                receipt_id: "rcpt_1".into(),
                permit_id: permit.id.clone(),
                action_id: action.id.clone(),
                status: custos_domain::ReceiptStatus::Success,
                output_digest: "blake3:00".into(),
                output_data: Some(
                    serde_json::json!({ "task_id": action.task_id, "target": action.target }),
                ),
                error_message: None,
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
                assurance: custos_domain::Assurance::CustosMediated,
            })
        }
    }

    let kernel = Arc::new(CapturingKernel {
        captured_task_id: std::sync::Mutex::new(None),
    });
    let executor = WorkerExecutor::new()
        .with_kernel(kernel.clone())
        .with_sandbox(Arc::new(EchoSandbox));

    let ctx = ExecutionContext::new("real_task_123", "run_456", "dag_node_789", "worker_1");
    let inputs =
        serde_json::json!({ "tool": "read_file", "parameters": { "path": "test.txt" } });

    let res = executor
        .execute_step(&ctx, "tool_call", &inputs)
        .await
        .unwrap();
    assert_eq!(
        kernel.captured_task_id.lock().unwrap().as_deref(),
        Some("real_task_123")
    );
    assert_ne!(
        kernel.captured_task_id.lock().unwrap().as_deref(),
        Some("dag_node_789")
    );
    assert_eq!(res["output_data"]["task_id"], "real_task_123");
    assert_eq!(res["output_data"]["target"], "test.txt");
}

#[tokio::test]
async fn shell_tool_does_not_inherit_read_risk_level() {
    struct MockKernel;
    #[async_trait]
    impl KernelPort for MockKernel {
        async fn create_task(
            &self,
            _title: String,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn create_task_with_contract(
            &self,
            _title: String,
            _contract: custos_domain::TaskContract,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn get_task(
            &self,
            _task_id: &str,
        ) -> Result<Option<custos_domain::Task>, DomainError> {
            unreachable!()
        }
        async fn transition_task(
            &self,
            _task_id: &str,
            _next: custos_domain::TaskStatus,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn request_permit(
            &self,
            _task_id: &str,
            _intent: &ActionIntent,
            _actor: &str,
        ) -> Result<custos_domain::Permit, DomainError> {
            unreachable!()
        }
        async fn consume_permit(
            &self,
            _permit_id: &str,
            _intent: &ActionIntent,
        ) -> Result<custos_domain::Permit, DomainError> {
            unreachable!()
        }
        async fn verify_evidence(
            &self,
            _bundle: &custos_core::evidence::EvidenceBundle,
        ) -> Result<custos_domain::VerificationClaim, DomainError> {
            unreachable!()
        }
        async fn grant_capability(
            &self,
            _task_id: &str,
            _grantee: &str,
            _capability: &str,
        ) -> Result<custos_domain::Grant, DomainError> {
            unreachable!()
        }
        async fn find_active_grant(
            &self,
            _task_id: &str,
            _capability: &str,
        ) -> Result<Option<custos_domain::Grant>, DomainError> {
            unreachable!()
        }
        async fn complete_task(
            &self,
            _task_id: &str,
            _summary: String,
            _claims: Vec<custos_domain::VerificationClaim>,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
    }
    let kernel = MockKernel;
    let read_risk = kernel.classify_effect("read_file", &serde_json::json!({}));
    let shell_risk =
        kernel.classify_effect("bash", &serde_json::json!({ "command": "echo 1" }));
    assert_eq!(read_risk, RiskLevel::Low);
    assert_eq!(shell_risk, RiskLevel::High);
    assert_ne!(read_risk, shell_risk);
}
