//! Custos Cognitive Orchestration & Runtime
//!
//! Manages the agent loop, cognitive S1/S2 routing, workflow DAG execution,
//! session lifecycle, and context window assembly.

pub mod agent;
pub mod cognitive;
pub mod context;
pub mod context_management;
pub mod gateway;
pub mod memory_service;
pub mod oi;
pub mod session;
pub mod workflow;
pub mod workspace;

// Compatibility shims for intra-crate modules and engine integration
pub use cognitive as custos_cognitive;
pub use context as custos_context;
pub use context_management as custos_context_management;
pub use context_management::*;
pub use custos_provider as custos_providers;

// Re-exports
#[allow(ambiguous_glob_reexports)]
pub use agent::*;
pub use cognitive::*;
pub use context::*;
pub use session::*;
pub use workflow::*;
pub use workspace::{CreateWorkspaceRequest, WorkspaceCoordinator};
