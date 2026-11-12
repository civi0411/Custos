//! Agent-to-Agent (A2A) Wire Protocol Types (v0.3 / v1.0 compatible).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent Card describing an autonomous agent's identity, capabilities, and interfaces.
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

/// Standard A2A Task state machine stages.
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

/// JSON-RPC 2.0 A2A Delegation Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aJsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 A2A Delegation Response.
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

/// Resource & cost bounds for an A2A task delegation under SADE Packet R3.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct A2aDelegationBudget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
}

/// Task Payload exchanged in A2A task interactions under SADE supervision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskPayload {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    pub state: A2aTaskState,
    #[serde(default)]
    pub artifacts: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<A2aDelegationBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Helpers for Agent Card path stripping and anchoring.
pub fn strip_agent_card_suffix(path: &str) -> &str {
    let path = path.strip_suffix("/.well-known/agent.json").unwrap_or(path);
    path.strip_suffix("/.well-known/agent-card.json")
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_agent_card_suffix() {
        assert_eq!(
            strip_agent_card_suffix(
                "http://localhost:8080/agents/custos/.well-known/agent-card.json"
            ),
            "http://localhost:8080/agents/custos"
        );
        assert_eq!(strip_agent_card_suffix("/.well-known/agent.json"), "");
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
}

