//! Governed Agent Runtime Implementation (Deep Dissection of Agent Loop)
//!
//! Bridges model inference and agent loop mechanics into the Sovereign Task Kernel
//! via `custos_provider::AgentRuntimePort`.
//!
//! Rather than executing tools in an unconstrained wild loop, candidate tool calls
//! are intercepted and transformed into `ActionIntent`s that require cryptographic
//! `ExecutionPermit`s from the Security Kernel (`custos-core`).

use async_trait::async_trait;
use custos_core::contracts::harness::{
    AgentRuntimePort, CostVisibility, HarnessProfile, ToolMediationLevel, WorktreeOwnership,
};
use custos_domain::{
    Action, ActionIntent, ActionLifecycleState, ContextPack, DomainError, RiskLevel, WorkerRun,
};
use custos_provider::{ModelPort, ProviderRequest};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum iterations in a multi-turn agent loop to prevent infinite loops.
pub const DEFAULT_MAX_ITERATIONS: usize = 10;

/// Maximum bytes for tool output before truncation (64 KiB) to guard against unbounded memory consumption.
pub const DEFAULT_MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;

/// Truncate a String to at most `max_bytes` without slicing inside a multi-byte UTF-8 char.
pub fn truncate_to_char_boundary(s: &mut String, max_bytes: usize) {
    custos_domain::truncate_to_char_boundary(s, max_bytes);
}

/// Invocation context resolved by the backend (never trusted from model payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvocationContext {
    pub task_id: String,
    pub run_id: String,
    pub actor: String,
    pub scope: Option<String>,
}

/// Result of executing a governed tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolResult {
    Success(serde_json::Value),
    PendingApproval { attempt_id: String },
    Error(String),
}

/// Interceptor/Executor for tools invoked by the governed agent loop.
///
/// Binds model tool calls to Gate 2 Policy / Authority validation and execution.
#[async_trait]
pub trait GovernedToolExecutor: Send + Sync {
    /// Authority / policy / gateway check before tool execution.
    /// If authorization fails, returns DomainError explaining rejection.
    async fn authorize(
        &self,
        ctx: &InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<(), DomainError>;

    /// Executes the tool call and returns ToolResult.
    async fn execute(
        &self,
        ctx: &InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<ToolResult, DomainError>;
}

pub struct GovernedAgentRuntime {
    provider: Arc<dyn ModelPort>,
    system_prompt: String,
    available_tools: Vec<serde_json::Value>,
    tool_executor: Option<Arc<dyn GovernedToolExecutor>>,
    max_iterations: usize,
    max_tool_output_bytes: usize,
    cancelled_runs: Arc<RwLock<HashSet<String>>>,
    steer_messages: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl GovernedAgentRuntime {
    pub fn new(
        provider: Arc<dyn ModelPort>,
        system_prompt: impl Into<String>,
        available_tools: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            provider,
            system_prompt: system_prompt.into(),
            available_tools,
            tool_executor: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            max_tool_output_bytes: DEFAULT_MAX_TOOL_OUTPUT_BYTES,
            cancelled_runs: Arc::new(RwLock::new(HashSet::new())),
            steer_messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_tool_executor(mut self, executor: Arc<dyn GovernedToolExecutor>) -> Self {
        self.tool_executor = Some(executor);
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_tool_output_cap(mut self, cap: usize) -> Self {
        self.max_tool_output_bytes = cap;
        self
    }

    pub fn register_tool(&mut self, tool: serde_json::Value) {
        self.available_tools.push(tool);
    }

    pub fn with_tool(mut self, tool: serde_json::Value) -> Self {
        self.available_tools.push(tool);
        self
    }

    pub fn is_tool_enabled(&self) -> bool {
        !self.available_tools.is_empty()
    }

    /// Evaluates tool name and arguments to classify deterministic risk level
    fn classify_tool_risk(name: &str, _params: &serde_json::Value) -> RiskLevel {
        match name {
            "read_file" | "list_dir" | "search_web" | "view_file" | "inspect" => RiskLevel::Low,
            "edit_file" | "write_file" | "create_file" | "patch" => RiskLevel::High,
            "shell" | "bash" | "exec" | "run_command" | "terminal" => RiskLevel::High,
            "delete_file" | "rm" | "drop_table" | "force_push" => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        }
    }

    /// Extracts the target resource (file path, URL, command) from parameters
    fn extract_target(params: &serde_json::Value) -> String {
        params
            .get("path")
            .or_else(|| params.get("target_file"))
            .or_else(|| params.get("file"))
            .or_else(|| params.get("url"))
            .or_else(|| params.get("command"))
            .or_else(|| params.get("query"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_target")
            .to_string()
    }
}

#[async_trait]
impl AgentRuntimePort for GovernedAgentRuntime {
    fn harness_id(&self) -> &str {
        "governed-agent-runtime"
    }

    fn profile(&self) -> HarnessProfile {
        let tool_mediation = if self.available_tools.is_empty() {
            ToolMediationLevel::ObserveOnly
        } else {
            ToolMediationLevel::CustosMediated
        };
        HarnessProfile {
            harness_id: "governed-agent-runtime".into(),
            tool_mediation,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: true,
            cost_visibility: CostVisibility::ExactTokens,
        }
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        let mut cancelled = self.cancelled_runs.write().await;
        cancelled.insert(run_id.to_string());
        Ok(())
    }

    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        let mut steer = self.steer_messages.write().await;
        steer
            .entry(run_id.to_string())
            .or_default()
            .push(guidance.to_string());
        Ok(())
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let effective_run_id = worker_run
            .run_id
            .clone()
            .unwrap_or_else(|| worker_run.id.clone());

        if self.cancelled_runs.read().await.contains(&effective_run_id) {
            return Err(DomainError::Validation(format!(
                "Run {} was canceled before execution",
                effective_run_id
            )));
        }

        // 1. Build initial prompt from context pack items and system instructions
        let mut prompt_content = String::new();
        prompt_content.push_str(&format!("System: {}\n\n", self.system_prompt));
        prompt_content.push_str(&format!(
            "Task ID: {}\nWorker Role: {}\n\nContext Items:\n",
            worker_run.task_id, worker_run.worker_id
        ));

        // Inject steering guidance if any
        {
            let mut steer_map = self.steer_messages.write().await;
            if let Some(steer_notes) = steer_map.remove(&effective_run_id) {
                prompt_content.push_str("\n--- Steering Guidance ---\n");
                for note in steer_notes {
                    prompt_content.push_str(&format!("- {}\n", note));
                }
                prompt_content.push('\n');
            }
        }

        for item in &context_pack.items {
            prompt_content.push_str(&format!(
                "--- Source: {} ---\n{}\n\n",
                item.source, item.content
            ));
        }

        // Fast path: if no tool executor is configured, preserve single-turn intent extraction
        let executor = match &self.tool_executor {
            Some(exec) if !self.available_tools.is_empty() => exec.clone(),
            _ => {
                let request = ProviderRequest {
                    request_id: custos_domain::new_id("req"),
                    task_id: worker_run.task_id.clone(),
                    span_num: worker_run.attempt_id,
                    prompt: prompt_content,
                    model: self.provider.provider_id().to_string(),
                    provider_account_id: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    max_tokens: Some(4096),
                    tools: if self.available_tools.is_empty() {
                        None
                    } else {
                        Some(self.available_tools.clone())
                    },
                };
                let response = self.provider.generate(&request).await?;
                let mut action_intents = Vec::new();
                for call in response.tool_calls {
                    let risk = Self::classify_tool_risk(&call.name, &call.arguments);
                    let target = Self::extract_target(&call.arguments);

                    let mut intent =
                        Action::new(call.call_id, call.name, target, call.arguments, risk);
                    intent.task_id = Some(worker_run.task_id.clone());
                    intent.lifecycle_state = ActionLifecycleState::Intent;
                    intent.assurance = custos_domain::Assurance::CustosMediated;
                    action_intents.push(intent);
                }
                return Ok(action_intents);
            }
        };

        // Multi-step governed agent loop:
        // Model propose tool -> authority check -> execute -> next turn continuation
        let mut conversation_history = String::new();
        let mut accumulated_intents = Vec::new();
        let mut completed_cleanly = false;

        for iteration in 0..self.max_iterations {
            if self.cancelled_runs.read().await.contains(&effective_run_id) {
                tracing::info!(
                    run_id = %effective_run_id,
                    iteration,
                    "Agent run canceled during multi-turn loop"
                );
                break;
            }

            let mut current_prompt = prompt_content.clone();
            if !conversation_history.is_empty() {
                current_prompt.push_str("\n--- Conversation History & Tool Observations ---\n");
                current_prompt.push_str(&conversation_history);
            }

            let request = ProviderRequest {
                request_id: custos_domain::new_id("req"),
                task_id: worker_run.task_id.clone(),
                span_num: worker_run.attempt_id.saturating_add(iteration as u32),
                prompt: current_prompt,
                model: self.provider.provider_id().to_string(),
                provider_account_id: None,
                reasoning_effort: None,
                temperature: Some(0.2),
                max_tokens: Some(4096),
                tools: Some(self.available_tools.clone()),
            };

            let response = self.provider.generate(&request).await?;
            if !response.content.trim().is_empty() {
                conversation_history.push_str(&format!("\nModel: {}\n", response.content));
            }

            if response.tool_calls.is_empty() {
                tracing::debug!(
                    run_id = %effective_run_id,
                    iteration,
                    "Model completed without additional tool calls"
                );
                completed_cleanly = true;
                break;
            }

            for call in response.tool_calls {
                if self.cancelled_runs.read().await.contains(&effective_run_id) {
                    tracing::info!(
                        run_id = %effective_run_id,
                        "Agent run canceled before tool execution"
                    );
                    break;
                }

                let risk = Self::classify_tool_risk(&call.name, &call.arguments);
                let target = Self::extract_target(&call.arguments);
                let mut intent = Action::new(
                    call.call_id.clone(),
                    call.name.clone(),
                    target,
                    call.arguments.clone(),
                    risk,
                );
                intent.task_id = Some(worker_run.task_id.clone());
                intent.lifecycle_state = ActionLifecycleState::Intent;
                intent.assurance = custos_domain::Assurance::CustosMediated;

                conversation_history.push_str(&format!(
                    "\nTool Request [{}] {}: {}\n",
                    call.call_id, call.name, call.arguments
                ));

                let invocation_ctx = InvocationContext {
                    task_id: worker_run.task_id.clone(),
                    run_id: effective_run_id.clone(),
                    actor: worker_run.worker_id.clone(),
                    scope: None,
                };

                // 1. Authority / Gateway check
                match executor.authorize(&invocation_ctx, &call.name, &call.arguments).await {
                    Ok(()) => {
                        if self.cancelled_runs.read().await.contains(&effective_run_id) {
                            intent.lifecycle_state = ActionLifecycleState::Failed;
                            accumulated_intents.push(intent);
                            break;
                        }

                        // 2. Execute tool
                        match executor.execute(&invocation_ctx, &call.name, &call.arguments).await {
                            Ok(ToolResult::Success(val)) => {
                                intent.lifecycle_state = ActionLifecycleState::Receipt;
                                let mut output_str = serde_json::to_string(&val)
                                    .unwrap_or_else(|_| val.to_string());
                                // 3. Output cap check & truncation
                                if output_str.len() > self.max_tool_output_bytes {
                                    truncate_to_char_boundary(&mut output_str, self.max_tool_output_bytes);
                                    output_str.push_str(
                                        "\n... [TRUNCATED: output exceeded max tool output limit]",
                                    );
                                }
                                conversation_history.push_str(&format!(
                                    "Tool Result [{}]: {}\n",
                                    call.call_id, output_str
                                ));
                            }
                            Ok(ToolResult::PendingApproval { attempt_id }) => {
                                intent.lifecycle_state = ActionLifecycleState::Permitted;
                                conversation_history.push_str(&format!(
                                    "Tool Pending Approval [{}]: Attempt ID '{}' requires human authorization before execution.\n",
                                    call.call_id, attempt_id
                                ));
                            }
                            Ok(ToolResult::Error(err_msg)) => {
                                intent.lifecycle_state = ActionLifecycleState::Failed;
                                conversation_history.push_str(&format!(
                                    "Tool Error [{}]: {}\n",
                                    call.call_id, err_msg
                                ));
                            }
                            Err(err) => {
                                intent.lifecycle_state = ActionLifecycleState::Failed;
                                conversation_history.push_str(&format!(
                                    "Tool Error [{}]: {}\n",
                                    call.call_id, err
                                ));
                            }
                        }
                    }
                    Err(rejection_err) => {
                        // Authority / Gateway rejection: clean error without hanging
                        intent.lifecycle_state = ActionLifecycleState::Failed;
                        conversation_history.push_str(&format!(
                            "Tool Rejected [{}] by authority/gateway: {}\n",
                            call.call_id, rejection_err
                        ));
                    }
                }

                accumulated_intents.push(intent);
            }
        }

        // Cleanup steering and cancellation handles for this run
        {
            let mut cancelled = self.cancelled_runs.write().await;
            cancelled.remove(&effective_run_id);
            let mut steer = self.steer_messages.write().await;
            steer.remove(&effective_run_id);
        }

        // E2: Iteration exhaustion without clean completion must return BudgetExceeded (limited), never success
        if !completed_cleanly && accumulated_intents.len() >= self.max_iterations {
            return Err(DomainError::BudgetExceeded(format!(
                "Agent loop iteration limit of {} exceeded without completion (status: limited)",
                self.max_iterations
            )));
        }

        Ok(accumulated_intents)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::ContextItem;
    use custos_provider::events::ToolCall;
    use custos_provider::request::ModelResponse;

    struct MockTestProvider {
        tool_to_return: Option<ToolCall>,
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

    // =========================================================================
    // GATE TESTS: Multi-step loop, Authority, Truncation, Cancel & Profile Truth
    // =========================================================================

    struct ScriptedMultiTurnProvider {
        responses: std::sync::Mutex<Vec<ModelResponse>>,
    }

    #[async_trait]
    impl ModelPort for ScriptedMultiTurnProvider {
        fn provider_id(&self) -> &str {
            "scripted-provider"
        }

        async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            let mut list = self.responses.lock().unwrap();
            if list.is_empty() {
                Ok(ModelResponse::text("Default completion", "scripted-model", 10))
            } else {
                Ok(list.remove(0))
            }
        }
    }

    struct TestToolExecutor {
        allow_delete: bool,
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
                return Err(DomainError::Validation("Permission denied: delete_file is disallowed".into()));
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
                "read_file" => Ok(ToolResult::Success(serde_json::json!({ "content": "fn main() {}" }))),
                "write_file" => Ok(ToolResult::Success(serde_json::json!({ "written": true, "path": params["path"] }))),
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

        let executor = Arc::new(TestToolExecutor { allow_delete: false });
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
        let context_pack = ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

        let intents = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();
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

        let executor = Arc::new(TestToolExecutor { allow_delete: false });
        let runtime = GovernedAgentRuntime::new(
            provider,
            "You are a test agent",
            vec![serde_json::json!({ "name": "delete_file" })],
        )
        .with_tool_executor(executor)
        .with_max_iterations(3);

        let worker_run = WorkerRun::new("task_reject".to_string(), "coder".to_string(), 1);
        let context_pack = ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

        let intents = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();
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

        let executor = Arc::new(TestToolExecutor { allow_delete: false });
        let runtime = GovernedAgentRuntime::new(
            provider,
            "You are a test agent",
            vec![serde_json::json!({ "name": "large_output_tool" })],
        )
        .with_tool_executor(executor)
        .with_tool_output_cap(100) // Cap to 100 bytes
        .with_max_iterations(2);

        let worker_run = WorkerRun::new("task_trunc".to_string(), "coder".to_string(), 1);
        let context_pack = ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

        let intents = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();
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
        let executor = Arc::new(TestToolExecutor { allow_delete: false });

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
            let context_pack = ContextPack::new("pack_mb".to_string(), vec![], 0, "sha256:0".to_string());

            let intents = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();
            assert_eq!(intents.len(), 1);
            assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Receipt);
        }
    }

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

        let executor = Arc::new(TestToolExecutor { allow_delete: false });
        let runtime = GovernedAgentRuntime::new(
            provider,
            "You are a test agent",
            vec![serde_json::json!({ "name": "read_file" })],
        )
        .with_tool_executor(executor);

        let worker_run = WorkerRun::new("task_cancel".to_string(), "coder".to_string(), 1);
        let context_pack = ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

        // Cancel the run in advance
        runtime.cancel_run(&worker_run.id).await.unwrap();

        // Must fail with Validation error indicating cancellation
        let err = runtime.execute_turn(&worker_run, &context_pack).await.unwrap_err();
        assert!(err.to_string().contains("canceled"));
    }

    #[tokio::test]
    async fn test_profile_honest_tool_capabilities() {
        let provider = Arc::new(MockTestProvider { tool_to_return: None });

        // When tools are empty: Must declare ObserveOnly
        let empty_runtime = GovernedAgentRuntime::new(
            provider.clone(),
            "Prompt",
            vec![],
        );
        assert_eq!(empty_runtime.profile().tool_mediation, ToolMediationLevel::ObserveOnly);
        assert!(!empty_runtime.is_tool_enabled());

        // When tools are registered: May declare CustosMediated
        let enabled_runtime = GovernedAgentRuntime::new(
            provider,
            "Prompt",
            vec![serde_json::json!({ "name": "sample_tool" })],
        );
        assert_eq!(enabled_runtime.profile().tool_mediation, ToolMediationLevel::CustosMediated);
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

        let executor = Arc::new(TestToolExecutor { allow_delete: false });
        let runtime = GovernedAgentRuntime::new(
            provider,
            "System prompt",
            vec![serde_json::json!({ "name": "read_file" })],
        )
        .with_tool_executor(executor)
        .with_max_iterations(2); // Only 2 iterations allowed

        let worker_run = WorkerRun::new("task_exhaust".to_string(), "coder".to_string(), 1);
        let context_pack = ContextPack::new("pack_ex".to_string(), vec![], 0, "sha256:0".to_string());

        let err = runtime.execute_turn(&worker_run, &context_pack).await.unwrap_err();
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
        let context_pack = ContextPack::new("pack_st".to_string(), vec![], 0, "sha256:0".to_string());

        // Add steering guidance
        runtime.steer_run(&worker_run.id, "Follow pattern X").await.unwrap();
        assert!(!runtime.steer_messages.read().await.is_empty());

        // Execute turn
        let _ = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();

        // Steering handles for this run must be cleaned up
        assert!(!runtime.steer_messages.read().await.contains_key(&worker_run.id));
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

        let resp2 = ModelResponse::text("Action is awaiting approval from operator.", "test-model", 10);

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
        let context_pack = ContextPack::new("pack_1".to_string(), vec![], 0, "sha256:0".to_string());

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
                return Err(DomainError::Unauthorized("Write not authorized by policy".into()));
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

        let resp2 = ModelResponse::text("I understand the write request was denied.", "test-model", 10);

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
        let context_pack = ContextPack::new("pack_unauth".to_string(), vec![], 0, "sha256:0".to_string());

        let intents = runtime
            .execute_turn(&worker_run, &context_pack)
            .await
            .expect("Turn should complete recording the denial");

        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Failed);
    }
}

