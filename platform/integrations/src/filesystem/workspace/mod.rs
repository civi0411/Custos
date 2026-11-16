//! Execution workspace provisioning, worktree management, and GitHub account binding.

pub mod files;
pub mod folder_picker;
pub mod git_discovery;
pub mod git_ops;
pub mod git_port;
pub mod github;
pub mod provider;

pub use files::LocalWorkspaceFilesAdapter;
pub use folder_picker::pick_folder;
pub use git_discovery::*;
pub use git_port::LocalRepositoryGitAdapter;
pub use provider::LocalWorkspaceProvider;
