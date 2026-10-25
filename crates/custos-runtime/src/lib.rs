//! Custos Cognitive Orchestration & Runtime
//!
//! Manages the agent loop, cognitive S1/S2 routing, workflow DAG execution,
//! session lifecycle, and context window assembly.

pub mod agent;
pub mod cognitive;
pub mod context;
pub use context::compaction as context_management;
#[cfg(feature = "gateway-experimental")]
pub mod gateway;
pub mod integration;
pub mod memory_service;
pub mod notebook;
pub mod oi;
pub mod session;
pub mod terminal;
pub mod workflow;
pub mod workspace;

pub use integration::{
    BindingDirection, IntegrationBinding, IntegrationKind, IntegrationRegistry, LifecycleState,
    MediationLevel,
};

// Re-exports
#[allow(ambiguous_glob_reexports)]
pub use agent::*;
pub use cognitive::*;
pub use context::*;
pub use notebook::PythonKernelCoordinator;
pub use session::*;
pub use terminal::TerminalCoordinator;
pub use workflow::*;
pub use workspace::{CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceFilesCoordinator};
