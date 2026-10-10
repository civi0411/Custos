//! Agent-to-Agent (A2A) Protocol and Agent Card Discovery
//!
//! Synthesized from AgentGateway (`crates/agentgateway/src/a2a/mod.rs`).
//! Implements standard A2A specification (v0.3 and v1.0 compatible) enabling
//! Custos to participate in peer agent delegation networks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent Card describing an autonomous agent's identity, capabilities, and interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub version: String,
    pub url: String,
    #[serde(rename = "supportedInterfaces", default)]
    pub supported_interfaces: Vec<AgentInterface>,
    pub capabilities: AgentCapabilities,
    #[serde(default)]
    pub authentication: AgentAuthRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInterface {
    pub protocol: String, // "jsonrpc-2.0", "mcp", "http"
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub streaming: bool,
    pub stateful_context: bool,
    pub tool_delegation: bool,
    pub human_in_the_loop: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentAuthRequirement {
    #[serde(rename = "type")]
    pub auth_type: String, // "none", "bearer", "oauth2"
    pub scopes: Vec<String>,
}

/// Standard A2A Task state machine stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2aTaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
}

impl A2aTaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Working => "working",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

/// JSON-RPC 2.0 A2A Delegation Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aJsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 A2A Delegation Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aJsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<A2aJsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aJsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Task Payload exchanged in A2A task interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskPayload {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    pub state: A2aTaskState,
    #[serde(default)]
    pub artifacts: Vec<Value>,
}

/// Helpers for Agent Card path stripping and anchoring
pub fn strip_agent_card_suffix(path: &str) -> &str {
    let path = path.strip_suffix("/.well-known/agent.json").unwrap_or(path);
    path.strip_suffix("/.well-known/agent-card.json")
        .unwrap_or(path)
}

/// Dispatcher for standard A2A JSON-RPC methods
pub struct A2aDispatcher;

impl A2aDispatcher {
    /// Inspect an A2A JSON-RPC request and extract context ID if present
    pub fn extract_context_id(req: &A2aJsonRpcRequest) -> Option<String> {
        req.params
            .get("contextId")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    /// Build a successful A2A response
    pub fn success_response(id: Value, result: Value) -> A2aJsonRpcResponse {
        A2aJsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build a method not found error response
    pub fn method_not_found(id: Value, method: &str) -> A2aJsonRpcResponse {
        A2aJsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(A2aJsonRpcError {
                code: -32601,
                message: format!("A2A Method not found: {}", method),
                data: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_strip_agent_card_suffix() {
        assert_eq!(
            strip_agent_card_suffix("http://localhost:8080/agents/custos/.well-known/agent-card.json"),
            "http://localhost:8080/agents/custos"
        );
        assert_eq!(
            strip_agent_card_suffix("/.well-known/agent.json"),
            ""
        );
    }

    #[test]
    fn test_agent_card_serialization() {
        let card = AgentCard {
            name: "Custos Autonomous Agent".to_string(),
            description: "Sovereign Engineering and Research Agent".to_string(),
            version: "0.1.0".to_string(),
            url: "http://localhost:1455".to_string(),
            supported_interfaces: vec![AgentInterface {
                protocol: "jsonrpc-2.0".to_string(),
                url: "http://localhost:1455/a2a".to_string(),
            }],
            capabilities: AgentCapabilities {
                streaming: true,
                stateful_context: true,
                tool_delegation: true,
                human_in_the_loop: true,
            },
            authentication: AgentAuthRequirement {
                auth_type: "bearer".to_string(),
                scopes: vec!["task:execute".to_string()],
            },
        };

        let json = serde_json::to_string(&card).expect("must serialize");
        let parsed: AgentCard = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(parsed.name, "Custos Autonomous Agent");
        assert!(parsed.capabilities.streaming);
    }

    #[test]
    fn test_a2a_dispatcher_method_not_found() {
        let resp = A2aDispatcher::method_not_found(json!(1), "unknown.method");
        assert_eq!(resp.error.unwrap().code, -32601);
    }
}
