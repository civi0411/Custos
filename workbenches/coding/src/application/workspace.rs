//! Coding Workspace Application Service
//!
//! Enforces worktree isolation, dirty tree inspection, and safe patching for software development.

use custos_contracts::{
    identity::error::DomainError,
    resources::{
        ports::{WorkspaceProvider, WorkspaceRepository},
        workspace::{DirtyManifest, ExecutionWorkspace, WorkspaceId, WorkspaceStatus},
    },
};
use std::sync::Arc;

pub struct WorkspaceApplicationService {
    repo: Arc<dyn WorkspaceRepository>,
    provider: Arc<dyn WorkspaceProvider>,
}

impl WorkspaceApplicationService {
    pub fn new(repo: Arc<dyn WorkspaceRepository>, provider: Arc<dyn WorkspaceProvider>) -> Self {
        Self { repo, provider }
    }

    pub async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        self.repo.get_workspace(id).await
    }

    pub async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        self.repo.list_workspaces().await
    }

    pub async fn inspect_dirty(
        &self,
        workspace: &ExecutionWorkspace,
    ) -> Result<DirtyManifest, DomainError> {
        self.provider.inspect_dirty(workspace).await
    }

    pub async fn diff(&self, workspace: &ExecutionWorkspace) -> Result<String, DomainError> {
        self.provider.diff(workspace).await
    }

    pub async fn update_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        self.repo.update_workspace_status(id, status).await
    }
}
