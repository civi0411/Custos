//! Resources & Physical Environment Contracts

pub mod artifact;
pub mod compute;
pub mod notes;
pub mod ports;
pub mod repo;
pub mod workspace;
pub mod workspace_files;

pub use artifact::*;
pub use compute::*;
pub use notes::*;
pub use ports::*;
pub use repo::*;
pub use workspace::*;
pub use workspace_files::*;

// Re-export terminal from execution context
pub use crate::execution::terminal;
pub use crate::execution::terminal::*;
