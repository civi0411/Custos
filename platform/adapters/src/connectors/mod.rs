//! Custos Connectors & Plugin Gateways
//!
//! External network protocols & plugin federation:
//! - `mcp`: Anthropic Model Context Protocol Hub (Plugin gateway for Gmail, Notion, Slack, Postgres)
//! - `a2a`: Agent-to-Agent JSON-RPC 2.0 protocol with AgentCards
//! - `roaming`: P2P worktree synchronization mesh via Iroh

pub mod a2a;
pub mod compute;
pub mod mcp;
pub mod oauth;
#[cfg(feature = "roaming")]
pub mod roaming;

// Direct ergonomic exports
pub use compute::{ColabTargetAdapter, KaggleTargetAdapter, RentedGpuTargetAdapter};
pub use mcp::McpFederationHub;
