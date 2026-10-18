//! Custos external adapter implementations.
//!
//! This crate owns concrete I/O: model connections, vendor transports, local
//! inference, native agent harnesses, MCP/A2A transports, filesystem/process
//! capabilities, workspaces and platform isolation. It implements contracts
//! owned by `custos-provider` and `custos-core`; it does not decide Task
//! completion, mint authority or own product routing policy.
//!
//! Module presence is not a readiness claim. Several modules are compatibility
//! wrappers, test doubles, protocol scaffolds or fail-closed placeholders. The
//! daemon is the production composition root and must expose only capabilities
//! backed by conformance evidence.

pub mod a2a;
pub mod download_manager;
pub mod harness;
pub mod local_inference;
pub mod mcp;
pub mod providers;
pub mod roaming;
pub mod sandbox;
pub mod tools;
pub mod workspace;
pub mod repository_git;

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
