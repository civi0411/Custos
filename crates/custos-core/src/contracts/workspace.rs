//! Workspace Port Contracts (RFC 006 / OrCa Integration)
//!
//! Defines the core interfaces for ExecutionWorkspace persistence (WorkspaceRepository)
//! and host environment lifecycle management (WorkspaceProvider).

use async_trait::async_trait;
use custos_domain::{
    DirtyManifest, DomainError, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus,
};

/// Persistence Port for ExecutionWorkspace entities.
///
/// Implemented by `custos-persistence` via SQLite.
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    /// Persists or inserts an execution workspace record.
    async fn save_workspace(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;

    /// Retrieves an execution workspace by its ID.
    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError>;

    /// Lists all workspaces.
    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError>;

    /// Updates only the workspace status and updated_at timestamp.
    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError>;

    /// Deletes a workspace record from persistence.
    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError>;
}

/// Host Operations Port for physical workspace provisioning, inspection, and recovery.
///
/// Implemented by `custos-adapters`.
#[async_trait]
pub trait WorkspaceProvider: Send + Sync {
    /// Validates whether the environment and paths are valid for the requested workspace kind.
    async fn validate(&self, kind: &WorkspaceKind, path: &str) -> Result<(), DomainError>;

    /// Physically allocates the directory, clones/adds git worktree, or prepares remote SSH target.
    async fn provision(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;

    /// Executes optional post-creation setup hooks or scripts in the provisioned workspace.
    async fn setup(
        &self,
        workspace: &ExecutionWorkspace,
        script: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Inspects the workspace for uncommitted changes and retrieves dirty manifest.
    async fn inspect_dirty(
        &self,
        _workspace: &ExecutionWorkspace,
    ) -> Result<DirtyManifest, DomainError> {
        Ok(DirtyManifest::default())
    }

    /// Generates unified diff relative to the recorded base commit or HEAD.
    async fn diff(&self, _workspace: &ExecutionWorkspace) -> Result<String, DomainError> {
        Ok(String::new())
    }

    /// Attempts to reconcile or recover an existing workspace on host.
    async fn recover(&self, _workspace: &ExecutionWorkspace) -> Result<bool, DomainError> {
        Ok(true)
    }

    /// Cleans up or detaches the physical workspace directory or worktree.
    async fn teardown(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;
}
