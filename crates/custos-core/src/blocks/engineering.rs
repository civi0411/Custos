//! Engineering Block (Core Layer)
//!
//! Contracts and policies empowering the Engineering Workbench:
//! workspace git operations, file system reads/writes, PTY terminal execution,
//! and filesystem path isolation policies.

pub use crate::contracts::terminal::{
    TerminalDriverPort, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle,
};
pub use crate::contracts::workspace::{RepositoryGitPort, WorkspaceProvider, WorkspaceRepository};
pub use crate::contracts::workspace_files::WorkspaceFilesPort;
pub use crate::sandbox_policy::PathSandbox;
