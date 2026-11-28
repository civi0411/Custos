//! Terminal Runtime Module
//!
//! Provides bounded pseudo-terminal (PTY) streams scoped to ExecutionWorkspaces.

pub mod buffer;
pub mod coordinator;
pub mod osc_scanner;

pub use coordinator::TerminalCoordinator;
pub use osc_scanner::{
    Osc7501Event, Osc7501StatefulScanner, Osc7501Status, ResolvedAgentStatusSource,
    StatusPrecedencePolicy, StatusPrecedenceResolver,
};
