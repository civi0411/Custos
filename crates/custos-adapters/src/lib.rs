//! Custos Adapters
//!
//! External adapters: Model Providers (Claude, Codex, Antigravity, Local),
//! Model Context Protocol (MCP), OS Sandboxes (Seatbelt, Bubblewrap),
//! Local Inference, Download Manager, and Roaming.

pub extern crate custos_domain as custos_core_domain;
pub extern crate custos_provider as custos_provider_sdk;
pub extern crate custos_provider as custos_provider_types;

pub mod download_manager;
pub mod local_inference;
pub mod mcp;
pub mod providers;
pub mod roaming;
pub mod sandbox;

pub mod model {
    pub use custos_provider::types::model::*;
    pub use crate::local_inference::model::*;
}

// Compatibility shims
pub use crate::mcp::adapters as custos_adapters_mcp;
pub use crate::mcp::core as custos_mcp;
pub use crate::providers::providers as custos_providers;
pub use crate::providers::fake as custos_adapter_provider_fake;
pub use crate::providers as model_providers;

// Re-exports for local_inference internal cross-references
pub use local_inference::backend;
pub use local_inference::tool_emulation;
pub use local_inference::tool_parsing;
pub use local_inference::provider_utils::*;

pub use download_manager::*;
pub use local_inference::*;
pub use mcp::*;
pub use providers::*;
pub use roaming::*;
pub use sandbox::*;
