use super::*;
use async_trait::async_trait;
use custos_domain::{
    ActionLifecycleState, ContextItem, ContextPack, DomainError, RiskLevel, WorkerRun,
};
use custos_provider::events::ToolCall;
use custos_provider::request::{ModelResponse, ProviderRequest};

pub(super) struct MockTestProvider {
    pub(super) tool_to_return: Option<ToolCall>,
}

#[async_trait]
impl ModelPort for MockTestProvider {
    fn provider_id(&self) -> &str {
        "mock-test-provider"
    }

    async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut resp = ModelResponse::text("Proposed modification", "mock-model", 42);
        if let Some(tool) = &self.tool_to_return {
            resp.tool_calls.push(tool.clone());
        }
        Ok(resp)
    }
}

pub(super) struct ScriptedMultiTurnProvider {
    pub(super) responses: std::sync::Mutex<Vec<ModelResponse>>,
}

#[async_trait]
impl ModelPort for ScriptedMultiTurnProvider {
    fn provider_id(&self) -> &str {
        "scripted-provider"
    }

    async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut list = self.responses.lock().unwrap();
        if list.is_empty() {
            Ok(ModelResponse::text(
                "Default completion",
                "scripted-model",
                10,
            ))
        } else {
            Ok(list.remove(0))
        }
    }
}

pub(super) struct TestToolExecutor {
    pub(super) allow_delete: bool,
}

#[async_trait]
impl GovernedToolExecutor for TestToolExecutor {
    async fn authorize(
        &self,
        _ctx: &InvocationContext,
        name: &str,
        _params: &serde_json::Value,
    ) -> Result<(), DomainError> {
        if name == "delete_file" && !self.allow_delete {
            return Err(DomainError::Validation(
                "Permission denied: delete_file is disallowed".into(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<ToolResult, DomainError> {
        match name {
            "read_file" => Ok(ToolResult::Success(
                serde_json::json!({ "content": "fn main() {}" }),
            )),
            "write_file" => Ok(ToolResult::Success(
                serde_json::json!({ "written": true, "path": params["path"] }),
            )),
            "large_output_tool" => {
                let large = "x".repeat(10_000);
                Ok(ToolResult::Success(serde_json::json!({ "data": large })))
            }
            "multibyte_tool" => {
                let text = "Tiếng Việt — Hệ thống AI Custos 🚀 🦀 ".repeat(50);
                Ok(ToolResult::Success(serde_json::json!({ "text": text })))
            }
            _ => Ok(ToolResult::Success(serde_json::json!({ "ok": true }))),
        }
    }
}

#[tokio::test]
async fn test_governed_agent_runtime_yields_action_intent() {
    let tool = ToolCall {
        call_id: "call_write_1".to_string(),
        name: "write_file".to_string(),
        arguments: serde_json::json!({
            "path": "src/main.rs",
            "content": "fn main() {}"
        }),
    };

    let provider = Arc::new(MockTestProvider {
        tool_to_return: Some(tool),
    });

    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a sovereign coding agent",
        vec![serde_json::json!({"name": "write_file"})],
    );

    let worker_run = WorkerRun::new("task_abc".to_string(), "coder".to_string(), 3);
    let context_pack = ContextPack {
        id: "pack_1".to_string(),
        items: vec![ContextItem {
            id: "item_1".to_string(),
            source: "specs.md".to_string(),
            content: "Implement main".to_string(),
            score: 1.0,
            tokens: 10,
            provenance_hash: None,
            is_tainted: false,
        }],
        total_tokens: 10,
        context_digest:
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
    };

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].name, "write_file");
    assert_eq!(intents[0].target, "src/main.rs");
    assert_eq!(intents[0].task_id.as_deref(), Some("task_abc"));
    assert_eq!(intents[0].risk_level, RiskLevel::High);
    assert!(intents[0].evidence_required);
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Intent);
}

#[tokio::test]
async fn test_multi_step_agent_loop_model_tool_result_continuation() {
    // Turn 1: Propose read_file
    let mut resp1 = ModelResponse::text("Let me read the file first", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_read_1".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({ "path": "src/main.rs" }),
    });

    // Turn 2: Propose write_file after seeing read result
    let mut resp2 = ModelResponse::text("Now I will write the update", "test-model", 30);
    resp2.tool_calls.push(ToolCall {
        call_id: "call_write_2".to_string(),
        name: "write_file".to_string(),
        arguments: serde_json::json!({ "path": "src/main.rs", "content": "new_code" }),
    });

    // Turn 3: Complete without further tool calls
    let resp3 = ModelResponse::text("Task complete successfully", "test-model", 15);

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2, resp3]),
    });

    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![
            serde_json::json!({ "name": "read_file" }),
            serde_json::json!({ "name": "write_file" }),
        ],
    )
    .with_tool_executor(executor)
    .with_max_iterations(5);

    let worker_run = WorkerRun::new("task_multi".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap();
    assert_eq!(intents.len(), 2);
    assert_eq!(intents[0].name, "read_file");
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Receipt);
    assert_eq!(intents[1].name, "write_file");
    assert_eq!(intents[1].lifecycle_state, ActionLifecycleState::Receipt);
}

#[tokio::test]
async fn test_rejected_tool_clean_error_no_hang() {
    // Model proposes delete_file which is disallowed
    let mut resp1 = ModelResponse::text("Deleting file...", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_del_1".to_string(),
        name: "delete_file".to_string(),
        arguments: serde_json::json!({ "path": "critical.db" }),
    });

    let resp2 = ModelResponse::text("I see the tool was rejected. Stopping.", "test-model", 15);

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2]),
    });

    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![serde_json::json!({ "name": "delete_file" })],
    )
    .with_tool_executor(executor)
    .with_max_iterations(3);

    let worker_run = WorkerRun::new("task_reject".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].name, "delete_file");
    // Rejection must mark the intent Failed, not hang or panic
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Failed);
}

#[tokio::test]
async fn test_tool_output_cap_truncation_no_panic() {
    let mut resp1 = ModelResponse::text("Reading big output...", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_big_1".to_string(),
        name: "large_output_tool".to_string(),
        arguments: serde_json::json!({}),
    });

    let resp2 = ModelResponse::text("Got the truncated output, finished.", "test-model", 15);

    let provider = Arc::new(ScriptedMultiTurnProvider {
        responses: std::sync::Mutex::new(vec![resp1, resp2]),
    });

    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a test agent",
        vec![serde_json::json!({ "name": "large_output_tool" })],
    )
    .with_tool_executor(executor)
    .with_tool_output_cap(100) // Cap to 100 bytes
    .with_max_iterations(2);

    let worker_run = WorkerRun::new("task_trunc".to_string(), "coder".to_string(), 1);
    let context_pack =
        ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Receipt);
}

#[test]
fn test_truncate_to_char_boundary_direct() {
    let mut s = "Tiếng Việt 🚀".to_string();
    // 'ế' starts at byte 2 and ends at byte 5 (3 bytes long)
    truncate_to_char_boundary(&mut s, 4);
    assert!(s.is_char_boundary(s.len()));
    assert_eq!(s, "Ti");

    let mut emoji = "🦀🚀".to_string(); // each 4 bytes
    truncate_to_char_boundary(&mut emoji, 5); // lands inside 🚀
    assert_eq!(emoji, "🦀");
}

#[tokio::test]
async fn test_tool_output_cap_truncation_multibyte_utf8_no_panic() {
    let executor = Arc::new(TestToolExecutor {
        allow_delete: false,
    });

    for cap in [15, 17, 21, 23, 27, 33, 45, 51] {
        let mut resp1 = ModelResponse::text("Reading multibyte...", "test-model", 20);
        resp1.tool_calls.push(ToolCall {
            call_id: "call_mb_1".to_string(),
            name: "multibyte_tool".to_string(),
            arguments: serde_json::json!({}),
        });
        let resp2 = ModelResponse::text("Finished.", "test-model", 10);
        let provider = Arc::new(ScriptedMultiTurnProvider {
            responses: std::sync::Mutex::new(vec![resp1, resp2]),
        });

        let runtime = GovernedAgentRuntime::new(
            provider,
            "System",
            vec![serde_json::json!({ "name": "multibyte_tool" })],
        )
        .with_tool_executor(executor.clone())
        .with_tool_output_cap(cap)
        .with_max_iterations(2);

        let worker_run = WorkerRun::new("task_mb".to_string(), "coder".to_string(), 1);
        let context_pack =
            ContextPack::new("pack_mb".to_string(), vec![], 0, "sha256:0".to_string());

        let intents = runtime
            .execute_turn(&worker_run, &context_pack)
            .await
            .unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Receipt);
    }
}

mod steering_and_writer;
