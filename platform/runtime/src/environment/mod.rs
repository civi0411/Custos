//! Sovereign Agent Execution Environment (AEE) Runtime Subsystem
//!
//! Orchestrates environment provisioning, verification, and teardown across
//! workspaces, terminals, notebooks, and compute targets.

pub mod compute;
pub mod coordinator;
pub mod notebook;
pub mod terminal;
pub mod workspace;

pub use coordinator::EnvironmentCoordinator;
