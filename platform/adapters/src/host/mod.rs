//! Custos Host OS & Physical Infrastructure Drivers
//!
//! Direct host system drivers:
//! - `sandbox`: OS kernel isolation (macOS Seatbelt, Linux Bubblewrap)
//! - `terminal`: Pseudo-terminal (PTY) master/slave controllers
//! - `workspace`: Atomic disk file IO & Git worktree lifecycles
//! - `notebook`: Shared Python/Jupyter kernel driver
//! - `packages`: Package installer & local filesystem package discovery
//! - `documents`: Bounded local document source acquisition
//! - `vault`: OS keyring and encrypted file credential vaults

pub mod binaries;
pub mod caffeinate;
pub mod documents;
pub mod integrations;
pub mod notebook;
pub mod operations;
pub mod packages;
pub mod sandbox;
pub mod telemetry;
pub mod terminal;
pub mod vault;
pub mod worker;
pub mod workspace;

// Direct ergonomic exports
pub use binaries::*;
pub use caffeinate::{CaffeinateManager, CAFFEINATE_MANAGER};
pub use workspace::git_discovery::*;
pub use documents::SourceFetchAdapter;
pub use notebook::LocalPythonDriver;
pub use sandbox::SovereignDeveloperAdapter;
pub use terminal::NativePtyDriver;
pub use vault::{FileCredentialVault, KeyringCredentialVault};
pub use worker::WorkerLease;
pub use workspace::{LocalWorkspaceFilesAdapter, LocalWorkspaceProvider};
