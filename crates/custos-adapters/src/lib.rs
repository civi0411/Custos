//! Custos Adapters
//!
//! External adapters: Model Providers (Claude, Codex, Antigravity, Local),
//! Model Context Protocol (MCP), OS Sandboxes (Seatbelt, Bubblewrap),
//! Local Inference, Download Manager, and Roaming.

pub mod a2a;
pub mod download_manager;
pub mod harness;
pub mod local_inference;
pub mod mcp;
pub mod providers;
pub mod roaming;
pub mod sandbox;
pub mod workspace;

pub mod model {
    pub use crate::local_inference::model::*;
    pub use custos_provider::types::model::*;
}

// Compatibility shims
pub use crate::mcp::adapters as custos_adapters_mcp;
pub use crate::mcp::core as custos_mcp;
pub use crate::providers as model_providers;
pub use crate::providers::fake as custos_adapter_provider_fake;
pub use crate::providers::providers as custos_providers;

// Re-exports for local_inference internal cross-references
pub use local_inference::backend;
pub use local_inference::provider_utils::*;
pub use local_inference::tool_emulation;
pub use local_inference::tool_parsing;

pub use a2a::*;
pub use download_manager::*;
pub use harness::*;
pub use local_inference::*;
pub use mcp::*;
pub use providers::*;
pub use roaming::*;
pub use sandbox::*;
pub use workspace::*;
