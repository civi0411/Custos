//! Governed Agent Runtime Implementation (Deep Dissection of Agent Loop)
//!
//! Bridges model inference and agent loop mechanics into the Sovereign Task Kernel
//! via `custos_provider::AgentRuntimePort`.
//!
//! Rather than executing tools in an unconstrained wild loop, candidate tool calls
//! are intercepted and transformed into `ActionIntent`s that require cryptographic
//! `ExecutionPermit`s from the Security Kernel (`custos-core`).

use async_trait::async_trait;
use custos_domain::{
    Action, ActionIntent, ActionLifecycleState, ContextPack, DomainError, RiskLevel, WorkerRun,
};
use custos_core::contracts::harness::{
    AgentRuntimePort, CostVisibility, HarnessProfile, ToolMediationLevel, WorktreeOwnership,
};
use custos_provider::{ModelProvider, ProviderRequest};
use std::sync::Arc;

pub struct GovernedAgentRuntime {
    provider: Arc<dyn ModelProvider>,
    system_prompt: String,
    available_tools: Vec<serde_json::Value>,
}

impl GovernedAgentRuntime {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        system_prompt: impl Into<String>,
        available_tools: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            provider,
            system_prompt: system_prompt.into(),
            available_tools,
        }
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
        HarnessProfile {
            harness_id: "governed-agent-runtime".into(),
            tool_mediation: ToolMediationLevel::CustosMediated,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: true,
            cost_visibility: CostVisibility::ExactTokens,
        }
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        // 1. Build prompt from context pack items and system instructions
        let mut prompt_content = String::new();
        prompt_content.push_str(&format!("System: {}\n\n", self.system_prompt));
        prompt_content.push_str(&format!(
            "Task ID: {}\nWorker Role: {}\n\nContext Items:\n",
            worker_run.task_id, worker_run.worker_id
        ));

        for item in &context_pack.items {
            prompt_content.push_str(&format!("--- Source: {} ---\n{}\n\n", item.source, item.content));
        }

        // 2. Formulate provider request
        let request = ProviderRequest {
            request_id: custos_domain::new_id("req"),
            task_id: worker_run.task_id.clone(),
            span_num: worker_run.attempt_id,
            prompt: prompt_content,
            model: self.provider.provider_id().to_string(),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools: if self.available_tools.is_empty() {
                None
            } else {
                Some(self.available_tools.clone())
            },
        };

        // 3. Invoke inference
        let response = self.provider.generate(&request).await?;

        // 4. Intercept tool calls and normalize into ActionIntents
        let mut action_intents = Vec::new();
        for call in response.tool_calls {
            let risk = Self::classify_tool_risk(&call.name, &call.arguments);
            let target = Self::extract_target(&call.arguments);

            let mut intent = Action::new(call.call_id, call.name, target, call.arguments, risk);
            intent.task_id = Some(worker_run.task_id.clone());
            intent.lifecycle_state = ActionLifecycleState::Intent;
            action_intents.push(intent);
        }

        Ok(action_intents)
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
    impl ModelProvider for MockTestProvider {
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
            context_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        };

        let intents = runtime.execute_turn(&worker_run, &context_pack).await.unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].name, "write_file");
        assert_eq!(intents[0].target, "src/main.rs");
        assert_eq!(intents[0].task_id.as_deref(), Some("task_abc"));
        assert_eq!(intents[0].risk_level, RiskLevel::High);
        assert!(intents[0].evidence_required);
        assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Intent);
    }
}
