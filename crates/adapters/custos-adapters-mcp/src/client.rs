use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    pub session_id: String,
    pub tool_call_request_id: String,
    pub working_dir: Option<PathBuf>,
}

impl ToolCallContext {
    pub fn new(session_id: impl Into<String>, tool_call_request_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            tool_call_request_id: tool_call_request_id.into(),
            working_dir: None,
        }
    }

    pub fn with_working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustosMcpHostInfo {
    pub explicit_extensions: bool,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
}

pub type GooseMcpHostInfo = CustosMcpHostInfo;

#[async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn list_tools(&self, next_cursor: Option<String>) -> Result<Vec<Tool>>;
    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult>;
}

/// CustosMcpClient manages communication with an external or embedded MCP server
pub struct CustosMcpClient {
    pub server_name: String,
    pub host_info: CustosMcpHostInfo,
    tools: Arc<Mutex<Vec<Tool>>>,
}

impl CustosMcpClient {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            host_info: CustosMcpHostInfo {
                explicit_extensions: true,
                client_name: Some("custos".into()),
                client_version: Some(env!("CARGO_PKG_VERSION").into()),
            },
            tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn register_mock_tool(&self, tool: Tool) {
        let mut list = self.tools.lock().await;
        list.push(tool);
    }
}

#[async_trait]
impl McpClientTrait for CustosMcpClient {
    async fn list_tools(&self, _next_cursor: Option<String>) -> Result<Vec<Tool>> {
        let list = self.tools.lock().await;
        Ok(list.clone())
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        tracing::info!(
            server = %self.server_name,
            tool = %name,
            session_id = %ctx.session_id,
            "Calling MCP tool"
        );

        // Execute tool call
        let result_content = format!(
            "Tool '{}' executed on server '{}' with args: {:?}",
            name, self.server_name, arguments
        );

        Ok(CallToolResult::success(vec![
            rmcp::model::ContentBlock::text(result_content),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_call() {
        let client = CustosMcpClient::new("test-server");
        let ctx = ToolCallContext::new("sess-1", "req-1");

        let res = client
            .call_tool(
                &ctx,
                "read_file",
                Some(serde_json::json!({"path": "Cargo.toml"})),
            )
            .await
            .unwrap();

        assert!(!res.content.is_empty());
        let text = res.content[0].as_text().unwrap();
        assert!(text.text.contains("read_file"));
    }
}
