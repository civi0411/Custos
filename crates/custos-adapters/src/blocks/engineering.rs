//! Engineering Workbench Adapter Block
//!
//! Provides concrete boundary implementations for Git repositories, linked worktrees,
//! POSIX pseudo-terminals (PTY), and operating system sandboxing.

pub use crate::sandbox::developer::SovereignDeveloperAdapter;
pub use crate::sandbox::CapabilityObservationsAdapter;
pub use crate::terminal::pty::{NativePtyDriver, PtyHandle};
pub use crate::workspace::git_port::LocalRepositoryGitAdapter;
pub use crate::workspace::github::GitHubBinding;
pub use crate::workspace::provider::LocalWorkspaceProvider;
