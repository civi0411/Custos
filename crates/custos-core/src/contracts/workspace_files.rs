//! Workspace Files and Git Diffs Port Contract
//!
//! Provides abstract file exploration, file read/write buffers, and Git diff inspection
//! strictly scoped to an `ExecutionWorkspace`.

use async_trait::async_trait;
use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceDiffSummary, WorkspaceFileContent,
    WorkspaceFileDiff, WorkspaceFileTree, WriteWorkspaceFileParams,
};

#[async_trait]
pub trait WorkspaceFilesPort: Send + Sync {
    /// Lists files and directories within an execution workspace.
    async fn get_file_tree(
        &self,
        workspace: &ExecutionWorkspace,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, DomainError>;

    /// Reads the content of a file within an execution workspace.
    async fn read_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, DomainError>;

    /// Writes content to a file within an execution workspace.
    async fn write_file(
        &self,
        workspace: &ExecutionWorkspace,
        params: WriteWorkspaceFileParams,
    ) -> Result<WorkspaceFileContent, DomainError>;

    /// Computes Git diff summary and changed files for an execution workspace.
    async fn get_diff(
        &self,
        workspace: &ExecutionWorkspace,
        staged: Option<bool>,
    ) -> Result<WorkspaceDiffSummary, DomainError>;

    /// Computes Git diff for a specific file in an execution workspace.
    async fn get_file_diff(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceFileDiff, DomainError>;

    /// Stages a file in Git (`git add`).
    async fn stage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;

    /// Unstages a file in Git (`git restore --staged`).
    async fn unstage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;

    /// Discards changes to a file in Git (`git restore` or removes untracked file).
    async fn discard_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;
}
