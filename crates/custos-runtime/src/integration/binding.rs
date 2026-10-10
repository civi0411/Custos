//! Canonical Integration Binding Definitions
//!
//! Models the connection identity, directionality, mediation tier,
//! and lifecycle state for all external protocols and services.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Category of external integration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationKind {
    /// AI Model endpoints or proxies (OpenAI, Anthropic, Gemini, Ollama, 9Router profile)
    ModelConnection,
    /// Tool, resource, or prompt providers (MCP stdio/HTTP servers, native tools)
    ToolServer,
    /// Multi-step autonomous coding harnesses (Claude Code CLI, Codex App Server, ACP)
    AgentHarness,
    /// Independent remote peer agents across network boundaries (A2A)
    RemotePeer,
}

/// Unified lifecycle state machine for all external connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleState {
    /// Declared in configuration or discovery source, not yet verified
    Configured,
    /// Metadata, capabilities, and version discovered from endpoint
    Discovered,
    /// Successfully exercised with a test handshake or ping fixture
    Validated,
    /// Admitted by user policy and actively available for routing
    Enabled,
    /// Experiencing errors, rate limits, or transient connection failures
    Degraded,
    /// Explicitly disabled by user or policy
    Disabled,
}

/// Directionality of the protocol interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingDirection {
    Inbound,
    Outbound,
    Bidirectional,
}

/// Level of mediation Custos has over native effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediationLevel {
    /// Every side-effect is intercepted and requires a Custos ExecutionPermit
    FullPermitMediated,
    /// Custos observes tool events and output, but cannot intercept before dispatch
    ObservedOnly,
    /// External process runs unobserved native effects inside its own sandbox
    Unsupervised,
}

/// Canonical registration record for an external integration binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationBinding {
    pub binding_id: Uuid,
    pub kind: IntegrationKind,
    pub name: String,
    pub endpoint_or_binary: String,
    pub protocol_version: String,
    pub direction: BindingDirection,
    pub mediation_level: MediationLevel,
    pub state: LifecycleState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tested_capabilities: Vec<String>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl IntegrationBinding {
    pub fn new(
        binding_id: Uuid,
        kind: IntegrationKind,
        name: impl Into<String>,
        endpoint_or_binary: impl Into<String>,
        protocol_version: impl Into<String>,
    ) -> Self {
        Self {
            binding_id,
            kind,
            name: name.into(),
            endpoint_or_binary: endpoint_or_binary.into(),
            protocol_version: protocol_version.into(),
            direction: BindingDirection::Outbound,
            mediation_level: MediationLevel::FullPermitMediated,
            state: LifecycleState::Configured,
            tested_capabilities: Vec::new(),
            last_health_check: None,
            error_message: None,
        }
    }
}
