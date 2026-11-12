use super::*;
use async_trait::async_trait;
use custos_core::contracts::harness::ToolMediationLevel;
use custos_domain::{
    ActionLifecycleState, ContextPack, DomainError, WorkerRun,
};
use custos_provider::events::ToolCall;
use custos_provider::request::ModelResponse;
use std::collections::HashSet;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_cancel_run_graceful_stop() {
    let mut resp1 = ModelResponse::text("Starting tool call...", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_read_1".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({ "path": "test.txt" }),
    });

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1]),
    });

    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![serde_json::json!({ "name": "read_file" })],
    )
    .with_tool_executor(executor);

    let worker_run = WorkerRun::new("task_cancel".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    // Cancel the run in advance
    runtime.cancel_run(&worker_run.id).await.unwrap();

    // Must fail with Validation error indicating cancellation
    let err = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("canceled"));
}

struct SelfCancellingToolExecutor {
    cancelled_handle: Arc<RwLock<HashSet<String>>>,
}

#[async_trait]
impl GovernedToolExecutor for SelfCancellingToolExecutor {
    async fn authorize(
        &self,
        _ctx: &InvocationContext,
        _name: &str,
        _params: &serde_json::Value,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn execute(
        &self,
        ctx: &InvocationContext,
        _name: &str,
        _params: &serde_json::Value,
    ) -> Result<ToolResult, DomainError> {
        self.cancelled_handle
            .write()
            .await
            .insert(ctx.run_id.clone());
        Ok(ToolResult::Success(serde_json::json!({"step": 1})))
    }
}

#[tokio::test]
async fn test_cancel_run_during_loop_stops_and_errors() {
    let mut resp1 = ModelResponse::text("Call 1", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_cancel_1".to_string(),
        name: "step_one".to_string(),
        arguments: serde_json::json!({}),
    });
    let mut resp2 = ModelResponse::text("Call 2", "test-model", 20);
    resp2.tool_calls.push(ToolCall {
        call_id: "call_cancel_2".to_string(),
        name: "step_two".to_string(),
        arguments: serde_json::json!({}),
    });

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2]),
    });

    let cancelled_runs = Arc::new(RwLock::new(HashSet::new()));
    let executor = Arc::new(SelfCancellingToolExecutor {
        cancelled_handle: cancelled_runs.clone(),
    });

    let mut runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![
            serde_json::json!({ "name": "step_one" }),
            serde_json::json!({ "name": "step_two" }),
        ],
    )
    .with_tool_executor(executor)
    .with_max_iterations(5);
    runtime.cancelled_runs = cancelled_runs;

    let worker_run = WorkerRun::new("task_cancel_mid".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    let err = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(err
        .to_string()
        .contains("canceled during agent loop execution"));
}

#[tokio::test]
async fn test_profile_honest_tool_capabilities() {
    let provider = Arc::new(MockTestProvider {
        tool_to_return: None,
    });

    // When tools are empty: Must declare ObserveOnly
    let empty_runtime = GovernedAgentRuntime::new(provider.clone(), "Prompt", vec![]);
    assert_eq!(
        empty_runtime.profile().tool_mediation,
        ToolMediationLevel::ObserveOnly
    );
    assert!(!empty_runtime.is_tool_enabled());

    // When tools are registered: May declare CustosMediated
    let enabled_runtime = GovernedAgentRuntime::new(
        provider,
        "Prompt",
        vec![serde_json::json!({ "name": "sample_tool" })],
    );
    assert_eq!(
        enabled_runtime.profile().tool_mediation,
        ToolMediationLevel::CustosMediated
    );
    assert!(enabled_runtime.is_tool_enabled());
}

#[tokio::test]
async fn test_iteration_exhaustion_returns_limited_error() {
    // Model keeps returning tool calls without completing across all max_iterations
    let mut resp1 = ModelResponse::text("Call 1", "test-model", 10);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_1".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({ "path": "1.txt" }),
    });
    let mut resp2 = ModelResponse::text("Call 2", "test-model", 10);
    resp2.tool_calls.push(ToolCall {
        call_id: "call_2".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({ "path": "2.txt" }),
    });
    let mut resp3 = ModelResponse::text("Call 3", "test-model", 10);
    resp3.tool_calls.push(ToolCall {
        call_id: "call_3".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({ "path": "3.txt" }),
    });

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2, resp3]),
    });

    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });
    let runtime = GovernedAgentRuntime::new(
        provider,
        "System prompt",
        vec![serde_json::json!({ "name": "read_file" })],
    )
    .with_tool_executor(executor)
    .with_max_iterations(2); // Only 2 iterations allowed

    let worker_run = WorkerRun::new("task_exhaust".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_ex".to_string(), vec![], 0, "sha256:0".to_string());

    let err = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::BudgetExceeded(_)));
    assert!(err.to_string().contains("iteration limit"));
}

#[tokio::test]
async fn test_cleanup_steering_handles_after_execution() {
    let resp1 = ModelResponse::text("Finished without tools", "test-model", 10);
    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1]),
    });

    let runtime = GovernedAgentRuntime::new(provider, "System prompt", vec![]);
    let worker_run = WorkerRun::new("task_steer_clean".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_st".to_string(), vec![], 0, "sha256:0".to_string());

    // Add steering guidance
    runtime
        .steer_run(&worker_run.id, "Follow pattern X")
        .await
        .unwrap();
    assert!(!runtime.steer_messages.read().await.is_empty());

    // Execute turn
    let _ = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap();

    // Steering handles for this run must be cleaned up
    assert!(!runtime
        .steer_messages
        .read()
        .await
        .contains_key(&worker_run.id));
}

struct ApprovalPendingToolExecutor;

#[async_trait]
impl GovernedToolExecutor for ApprovalPendingToolExecutor {
    async fn authorize(
        &self,
        _ctx: &InvocationContext,
        _name: &str,
        _params: &serde_json::Value,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &InvocationContext,
        _name: &str,
        _params: &serde_json::Value,
    ) -> Result<ToolResult, DomainError> {
        Ok(ToolResult::PendingApproval {
            attempt_id: "att_gate_b1".to_string(),
        })
    }
}

#[tokio::test]
async fn pending_approval_does_not_retry_as_failure() {
    let mut resp1 = ModelResponse::text("I want to update the file", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_write_approval".to_string(),
        name: "write_file".to_string(),
        arguments: serde_json::json!({ "path": "sensitive.txt", "content": "secret" }),
    });

    let resp2 = ModelResponse::text(
        "Action is awaiting approval from operator.",
        "test-model",
        10,
    );

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2]),
    });

    let executor = Arc::new(ApprovalPendingToolExecutor);
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![serde_json::json!({ "name": "write_file" })],
    )
    .with_tool_executor(executor)
    .with_max_iterations(5);

    let worker_run = WorkerRun::new("task_approval".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect("Pending approval turn should not abort or error");

    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Permitted);
}

struct RejectingToolExecutor;

#[async_trait]
impl GovernedToolExecutor for RejectingToolExecutor {
    async fn authorize(
        &self,
        _ctx: &InvocationContext,
        name: &str,
        _params: &serde_json::Value,
    ) -> Result<(), DomainError> {
        if name == "write_file" {
            return Err(DomainError::Unauthorized(
                "Write not authorized by policy".into(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &InvocationContext,
        _name: &str,
        _params: &serde_json::Value,
    ) -> Result<ToolResult, DomainError> {
        panic!("Execute should NOT be called for unauthorized tool");
    }
}

#[tokio::test]
async fn unauthorized_write_is_rejected_not_executed() {
    let mut resp1 = ModelResponse::text("I want to write a file", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_unauth_write".to_string(),
        name: "write_file".to_string(),
        arguments: serde_json::json!({ "path": "unauthorized.txt", "content": "malicious" }),
    });

    let resp2 = ModelResponse::text(
        "I understand the write request was denied.",
        "test-model",
        10,
    );

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2]),
    });

    let executor = Arc::new(RejectingToolExecutor);
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![serde_json::json!({ "name": "write_file" })],
    )
    .with_tool_executor(executor)
    .with_max_iterations(5);

    let worker_run = WorkerRun::new("task_unauth".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_unauth".to_string(), vec![], 0, "sha256:0".to_string());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect("Turn should complete recording the denial");

    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Failed);
}

#[test]
fn test_limited_writer_up_to_limit() {
    use std::io::Write;
    let mut w = LimitedWriter::new(Vec::new(), 10);
    w.write_all(b"hello").unwrap();
    let (buf, capped) = w.into_inner();
    assert!(!capped);
    assert_eq!(buf, b"hello");
}

#[test]
fn test_limited_writer_caps_at_exact_limit() {
    use std::io::Write;
    let mut w = LimitedWriter::new(Vec::new(), 5);
    let _ = w.write(b"hello world");
    let (buf, capped) = w.into_inner();
    assert!(capped);
    assert_eq!(&buf, b"hello");
}

#[test]
fn test_limited_writer_zero_cap_is_immediately_capped() {
    use std::io::Write;
    let mut w = LimitedWriter::new(Vec::new(), 0);
    let _ = w.write(b"x");
    let (buf, capped) = w.into_inner();
    assert!(capped);
    assert!(buf.is_empty());
}
