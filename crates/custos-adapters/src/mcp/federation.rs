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
use tokio::sync::RwLock;
use tracing::info;

pub struct McpFederationHub {
    servers: Arc<RwLock<HashMap<String, Arc<dyn McpClientTrait>>>>,
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
        }
    }

    /// Register an MCP server under a specific namespace prefix
    pub async fn register_server(&self, prefix: impl Into<String>, client: Arc<dyn McpClientTrait>) {
        let prefix = prefix.into();
        info!("Registering MCP server under namespace prefix '{}'", prefix);
        let mut guard = self.servers.write().await;
        guard.insert(prefix, client);
    }

    /// List all federated tools across all registered servers with namespaced names (`prefix__name`)
    pub async fn list_federated_tools(&self) -> Result<Vec<ToolDefinition>> {
        let guard = self.servers.read().await;
        let mut federated_tools = Vec::new();

        for (prefix, client) in guard.iter() {
            match client.list_tools(None).await {
                Ok(tools) => {
                    for tool in tools {
                        let namespaced_name = format!("{}__{}", prefix, tool.name);
                        federated_tools.push(ToolDefinition {
                            name: namespaced_name,
                            description: tool.description.unwrap_or_default().to_string(),
                            parameters: serde_json::to_value(tool.input_schema).unwrap_or_default(),
                        });
                    }
                }
                Err(err) => {
                    tracing::warn!("Failed to list tools from MCP server '{}': {}", prefix, err);
                }
            }
        }

        Ok(federated_tools)
    }

    /// Dispatch a namespaced tool call (`prefix__tool_name`) to the correct MCP server
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

        let guard = self.servers.read().await;
        let client = guard.get(prefix).cloned().ok_or_else(|| {
            DomainError::NotFound {
                kind: "McpServer".to_string(),
                id: prefix.to_string(),
            }
        })?;

        client
            .call_tool(ctx, original_name, args)
            .await
            .map_err(|e| DomainError::Validation(format!("MCP call failed: {}", e)))
    }
}
