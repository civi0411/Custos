//! Unified Integration Registry Module
//!
//! Manages connection identities, capabilities, mediation tiers, and lifecycle states
//! for Model Connections, Tool Servers (MCP), Agent Runtimes, and Remote Peers.

pub mod binding;
pub mod registry;

pub use binding::*;
pub use registry::*;
