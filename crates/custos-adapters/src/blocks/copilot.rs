//! Copilot Workbench Adapter Block
//!
//! Provides concrete boundary implementations for model intelligence (cloud providers and
//! native local inference), external CLI agent harnesses, MCP tool federation, and A2A communication.

pub use crate::a2a::{
    A2aClient, A2aDelegationBudget, A2aDispatcher, A2aJsonRpcError, A2aJsonRpcRequest,
    A2aJsonRpcResponse, A2aTaskPayload, A2aTaskState, AgentAuthRequirement, AgentCapabilities,
    AgentCard, AgentInterface,
};
pub use crate::harness::adapter::{ClaudeCodeHarnessAdapter, CliHarness, OpenCodeHarnessAdapter};
pub use crate::harness::registry::{HarnessEntry, HarnessRegistry};
pub use crate::harness::spec::{HarnessSpec, HeadlessArgs, StreamProtocol};
pub use crate::mcp::federation::{McpFederationHub, McpToolGate};
