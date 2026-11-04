//! Execution workspace provisioning, worktree management, and GitHub account binding.

pub mod git_port;
pub mod github;
pub mod provider;

pub use git_port::LocalRepositoryGitAdapter;
pub use provider::LocalWorkspaceProvider;
