//! Terminal Runtime Module
//!
//! Provides bounded pseudo-terminal (PTY) streams scoped to ExecutionWorkspaces.

pub mod buffer;
pub mod coordinator;
pub mod pty;

pub use coordinator::TerminalCoordinator;
