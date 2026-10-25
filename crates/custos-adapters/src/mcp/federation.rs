//! MCP Federation Hub
//!
//! Aggregates multiple upstream MCP servers into a single federated namespace.
//! Absorbs AgentGateway's MCP multi-server routing and federation architecture.

use crate::mcp::adapters::client::{McpClientTrait, ToolCallContext};
use anyhow::Result;
use custos_domain::DomainError;
use custos_provider::tool::ToolDefinition;
use rmcp::model::CallToolResult;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Authorization gate for MCP tool calls.
#[async_trait::async_trait]
pub trait ToolGate: Send + Sync {
    async fn authorize(
        &self,
        namespaced_tool: &str,
        arguments_digest: &str,
        ctx: &ToolCallContext,
    ) -> Result<(), DomainError>;
}

pub struct McpFederationHub {
    servers: Arc<RwLock<HashMap<String, Arc<dyn McpClientTrait>>>>,
    gate: Arc<RwLock<Option<Arc<dyn ToolGate>>>>,
}

impl Default for McpFederationHub {
    fn default() -> Self {
        Self::new()
    }
}

impl McpFederationHub {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            gate: Arc::new(RwLock::new(None)),
        }
    }

    /// Set an authorization gate for tool dispatch.
    pub async fn set_gate(&self, gate: Arc<dyn ToolGate>) {
        let mut guard = self.gate.write().await;
        *guard = Some(gate);
    }

    /// Register an MCP server under a specific namespace prefix.
    /// Prefix must be non-empty, alphanumeric / dashes / underscores, and cannot contain `__`.
    pub async fn register_server(
        &self,
        prefix: impl Into<String>,
        client: Arc<dyn McpClientTrait>,
    ) -> Result<(), DomainError> {
        let prefix = prefix.into();
        if prefix.is_empty() {
            return Err(DomainError::Validation("MCP prefix cannot be empty".into()));
        }
        if prefix.contains("__") {
            return Err(DomainError::Validation(format!(
                "MCP prefix '{}' cannot contain double underscore '__'",
                prefix
            )));
        }
        if !prefix.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(DomainError::Validation(format!(
                "MCP prefix '{}' contains invalid characters; only alphanumeric, underscore, and dash are allowed",
                prefix
            )));
        }

        info!("Registering MCP server under namespace prefix '{}'", prefix);
        let mut guard = self.servers.write().await;
        guard.insert(prefix, client);
        Ok(())
    }

    /// List all federated tools across all registered servers with namespaced names (`prefix__name`).
    /// Output is deterministically sorted alphabetically by tool name.
    pub async fn list_federated_tools(&self) -> Result<Vec<ToolDefinition>> {
        let guard = self.servers.read().await;
        let mut federated_tools = Vec::new();

        for (prefix, client) in guard.iter() {
            let fetch_future = client.list_tools(None);
            let fetch_res = tokio::time::timeout(Duration::from_secs(10), fetch_future).await;

            match fetch_res {
                Ok(Ok(tools)) => {
                    for tool in tools {
                        let namespaced_name = format!("{}__{}", prefix, tool.name);
                        federated_tools.push(ToolDefinition {
                            name: namespaced_name,
                            description: tool.description.unwrap_or_default().to_string(),
                            parameters: serde_json::to_value(tool.input_schema).unwrap_or_default(),
                        });
                    }
                }
                Ok(Err(err)) => {
                    tracing::warn!("Failed to list tools from MCP server '{}': {}", prefix, err);
                }
                Err(_) => {
                    tracing::warn!("Timeout listing tools from MCP server '{}'", prefix);
                }
            }
        }

        // Deterministic sorting by namespaced tool name
        federated_tools.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(federated_tools)
    }

    /// List all federated resources across registered MCP servers.
    pub async fn list_federated_resources(&self) -> Result<Vec<Value>> {
        let guard = self.servers.read().await;
        let mut federated_resources = Vec::new();

        for (prefix, client) in guard.iter() {
            if let Ok(Ok(resources)) = tokio::time::timeout(Duration::from_secs(10), client.list_resources(None)).await {
                for mut res in resources {
                    if let Some(obj) = res.as_object_mut() {
                        obj.insert("server_prefix".into(), Value::String(prefix.clone()));
                    }
                    federated_resources.push(res);
                }
            }
        }

        Ok(federated_resources)
    }

    /// Read a resource from a specific federated server prefix.
    pub async fn read_federated_resource(&self, prefix: &str, uri: &str) -> Result<Value, DomainError> {
        let guard = self.servers.read().await;
        let client = guard.get(prefix).cloned().ok_or_else(|| DomainError::NotFound {
            kind: "McpServer".into(),
            id: prefix.to_string(),
        })?;

        client.read_resource(uri).await.map_err(|e| DomainError::Validation(format!("Failed to read resource '{uri}': {e}")))
    }

    /// Dispatch a namespaced tool call (`prefix__tool_name`) to the correct MCP server
    /// Enforces a 30-second timeout on tool execution.
    pub async fn dispatch_call(
        &self,
        namespaced_name: &str,
        args: Option<Value>,
        ctx: &ToolCallContext,
    ) -> Result<CallToolResult, DomainError> {
        let parts: Vec<&str> = namespaced_name.splitn(2, "__").collect();
        if parts.len() != 2 {
            return Err(DomainError::Validation(format!(
                "Invalid federated tool name '{}', expected 'prefix__name'",
                namespaced_name
            )));
        }

        let prefix = parts[0];
        let original_name = parts[1];

        // S8: Authorize with ToolGate before attempting connection or dispatch
        let gate_opt = self.gate.read().await.clone();
        if let Some(gate) = gate_opt {
            let arg_digest = format!(
                "sha256:{}",
                custos_domain::digest(serde_json::to_string(&args).unwrap_or_default().as_bytes())
            );
            gate.authorize(namespaced_name, &arg_digest, ctx).await?;
        }

        let guard = self.servers.read().await;
        let client = guard
            .get(prefix)
            .cloned()
            .ok_or_else(|| DomainError::NotFound {
                kind: "McpServer".to_string(),
                id: prefix.to_string(),
            })?;

        let call_future = client.call_tool(ctx, original_name, args);
        match tokio::time::timeout(Duration::from_secs(30), call_future).await {
            Ok(res) => res.map_err(|e| DomainError::Validation(format!("MCP call failed: {}", e))),
            Err(_) => Err(DomainError::Validation(format!(
                "MCP tool call '{}' timed out after 30 seconds",
                namespaced_name
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::adapters::client::CustosMcpClient;

    struct DenyGate;

    #[async_trait::async_trait]
    impl ToolGate for DenyGate {
        async fn authorize(
            &self,
            namespaced_tool: &str,
            _arg_digest: &str,
            _ctx: &ToolCallContext,
        ) -> Result<(), DomainError> {
            if namespaced_tool.contains("forbidden") {
                return Err(DomainError::Unauthorized("Access denied by ToolGate".into()));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_federation_hub_tool_gate_authorization() {
        let hub = McpFederationHub::new();
        hub.set_gate(Arc::new(DenyGate)).await;

        let client = Arc::new(CustosMcpClient::new("server-secure"));
        hub.register_server("secure", client).await.unwrap();

        let ctx = ToolCallContext::new("sess-gate", "req-gate");

        // Forbidden tool must be rejected by gate before any dispatch
        let denied = hub.dispatch_call("secure__forbidden_tool", None, &ctx).await;
        assert!(denied.is_err());
        assert!(matches!(denied.unwrap_err(), DomainError::Unauthorized(msg) if msg.contains("ToolGate")));

        // Allowed tool passes gate
        let allowed = hub.dispatch_call("secure__allowed_tool", None, &ctx).await;
        assert!(allowed.is_ok());
    }

    #[tokio::test]
    async fn test_federation_hub_registration_and_dispatch() {
        let hub = McpFederationHub::new();
        let client1 = Arc::new(CustosMcpClient::new("server-alpha"));
        let client2 = Arc::new(CustosMcpClient::new("server-beta"));

        hub.register_server("alpha", client1).await.expect("alpha registered");
        hub.register_server("beta", client2).await.expect("beta registered");

        let ctx = ToolCallContext::new("sess-fed", "req-fed");
        let res = hub
            .dispatch_call("alpha__echo", Some(serde_json::json!({"msg": "hello"})), &ctx)
            .await
            .expect("dispatch succeeds");

        assert!(!res.content.is_empty());
        let text = res.content[0].as_text().unwrap();
        assert!(text.text.contains("echo"));
        assert!(text.text.contains("server-alpha"));
    }

    #[tokio::test]
    async fn test_federation_hub_rejects_invalid_prefix() {
        let hub = McpFederationHub::new();
        let client = Arc::new(CustosMcpClient::new("server-x"));

        assert!(hub.register_server("", client.clone()).await.is_err());
        assert!(hub.register_server("invalid__prefix", client.clone()).await.is_err());
        assert!(hub.register_server("invalid prefix with spaces", client.clone()).await.is_err());
        assert!(hub.register_server("valid_prefix-123", client.clone()).await.is_ok());
    }

    #[tokio::test]
    async fn test_federation_hub_deterministic_sorting() {
        let hub = McpFederationHub::new();
        let client_z = Arc::new(CustosMcpClient::new("z-srv"));
        let client_a = Arc::new(CustosMcpClient::new("a-srv"));

        let tool_z: rmcp::model::Tool = serde_json::from_value(serde_json::json!({
            "name": "z_tool",
            "description": "z description",
            "inputSchema": {}
        })).unwrap();
        let tool_a: rmcp::model::Tool = serde_json::from_value(serde_json::json!({
            "name": "a_tool",
            "description": "a description",
            "inputSchema": {}
        })).unwrap();

        client_z.register_mock_tool(tool_z).await;
        client_a.register_mock_tool(tool_a).await;

        hub.register_server("z", client_z).await.unwrap();
        hub.register_server("a", client_a).await.unwrap();

        let tools = hub.list_federated_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "a__a_tool");
        assert_eq!(tools[1].name, "z__z_tool");
    }
}
