use async_trait::async_trait;
use custos_domain::{DomainError, RiskLevel};
use serde::{Deserialize, Serialize};

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

/// Evaluates tool name and arguments to classify deterministic risk level
pub(crate) fn classify_tool_risk(name: &str, _params: &serde_json::Value) -> RiskLevel {
    match name {
        "read_file" | "list_dir" | "search_web" | "view_file" | "inspect" => RiskLevel::Low,
        "edit_file" | "write_file" | "create_file" | "patch" => RiskLevel::High,
        "shell" | "bash" | "exec" | "run_command" | "terminal" => RiskLevel::High,
        "delete_file" | "rm" | "drop_table" | "force_push" => RiskLevel::Critical,
        _ => RiskLevel::Medium,
    }
}

/// Extracts the target resource (file path, URL, command) from parameters
pub(crate) fn extract_target(params: &serde_json::Value) -> String {
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
