//! Custos Cognitive Orchestration & Runtime
//!
//! Manages the agent loop, cognitive S1/S2 routing, workflow DAG execution,
//! session lifecycle, and context window assembly.

pub extern crate custos_core as custos_kernel;
pub extern crate custos_domain as custos_core_domain;
pub extern crate custos_provider as custos_provider_sdk;
pub extern crate custos_provider as custos_provider_types;

pub mod agent;
pub mod cognitive;
pub mod context;
pub mod context_management;
pub mod gateway;
pub mod memory_service;
pub mod session;
pub mod workflow;

// Compatibility shims for intra-crate modules
pub use cognitive as custos_cognitive;
pub use context as custos_context;
pub use context_management::*;

// Re-exports
#[allow(ambiguous_glob_reexports)]
pub use agent::*;
pub use cognitive::*;
pub use context::*;
pub use session::*;
pub use workflow::*;
