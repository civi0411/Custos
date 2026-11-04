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
pub mod compute;
pub mod download_manager;
pub mod harness;
pub mod local_inference;
pub mod market;
pub mod mcp;
pub mod notebook;
pub mod providers;
#[cfg(feature = "roaming")]
pub mod roaming;
pub mod sandbox;
pub mod terminal;
pub mod tools;
pub mod workspace;
pub use workspace::github as repository_git;


pub mod model {
    pub use crate::local_inference::model::*;
    pub use custos_provider::types::model::*;
}

pub use local_inference::backend;
pub use local_inference::provider_utils::*;
pub use local_inference::tool_emulation;
pub use local_inference::tool_parsing;

// Internal crate re-exports required by vendored provider client implementations
pub(crate) use local_inference::*;
pub(crate) use providers::*;


