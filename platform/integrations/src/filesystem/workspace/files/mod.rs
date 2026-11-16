//! Workspace Files & Git Diff Runtime Subsystem
//!
//! Provides strictly scoped, fail-closed file tree browsing, file read/write,
//! and Git diff / stage / discard operations bounded to an `ExecutionWorkspace`.

use async_trait::async_trait;
use std::path::{Component, Path, PathBuf};

use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceDiffSummary, WorkspaceFileContent,
    WorkspaceFileDiff, WorkspaceFileTree, WriteWorkspaceFileParams,
};

pub mod git;
pub mod io;

#[cfg(test)]
mod tests;

pub use git::{is_git_repo, run_git};

/// Default maximum bytes to read from a single file (2 MB)
pub const DEFAULT_MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
/// Hard ceiling maximum file bytes (10 MB)
pub const HARD_MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
/// Maximum entries to return in a file tree before truncation
pub const MAX_TREE_ENTRIES: usize = 5000;

/// Resolves and validates a relative path strictly inside `base_dir`.
/// Fail-closed: returns `DomainError::Validation` if the path attempts traversal.
pub fn resolve_safe_path(base_dir: &Path, rel_path: &str) -> Result<PathBuf, DomainError> {
    let trimmed = rel_path.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err(DomainError::Validation("File path cannot be empty".into()));
    }

    let input_path = Path::new(&trimmed);

    // Reject absolute paths that don't start cleanly
    for comp in input_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(DomainError::Validation(
                    "Path traversal ('..') is strictly prohibited".into(),
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                // If input starts with root slash, treat as relative to base_dir
                continue;
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    // Strip leading slashes to safely join with base_dir
    let clean_rel = trimmed.trim_start_matches('/');
    let target = base_dir.join(clean_rel);

    // Verify canonical containment if base_dir exists
    if let Ok(canonical_base) = base_dir.canonicalize() {
        if target.exists() {
            if let Ok(canonical_target) = target.canonicalize() {
                if !canonical_target.starts_with(&canonical_base) {
                    return Err(DomainError::Validation(
                        "Access denied: resolved path escapes execution workspace boundary".into(),
                    ));
                }
            }
        } else if let Some(parent) = target.parent() {
            // Check closest existing ancestor
            let mut curr = parent;
            while !curr.exists() {
                if let Some(p) = curr.parent() {
                    curr = p;
                } else {
                    break;
                }
            }
            if let Ok(canonical_curr) = curr.canonicalize() {
                if !canonical_curr.starts_with(&canonical_base) {
                    return Err(DomainError::Validation(
                        "Access denied: target path escapes execution workspace boundary".into(),
                    ));
                }
            }
        }
    }

    Ok(target)
}

/// Physical workspace file & Git diff adapter implementing `WorkspaceFilesPort`.
#[derive(Clone, Default)]
pub struct LocalWorkspaceFilesAdapter;

impl LocalWorkspaceFilesAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkspaceFilesPort for LocalWorkspaceFilesAdapter {
    async fn get_file_tree(
        &self,
        workspace: &ExecutionWorkspace,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, DomainError> {
        io::get_file_tree(workspace, relative_dir, max_depth).await
    }

    async fn read_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, DomainError> {
        io::read_file(workspace, relative_path, max_bytes).await
    }

    async fn write_file(
        &self,
        workspace: &ExecutionWorkspace,
        params: WriteWorkspaceFileParams,
    ) -> Result<WorkspaceFileContent, DomainError> {
        io::write_file(workspace, params).await
    }

    async fn get_diff(
        &self,
        workspace: &ExecutionWorkspace,
        staged: Option<bool>,
    ) -> Result<WorkspaceDiffSummary, DomainError> {
        git::get_diff(workspace, staged).await
    }

    async fn get_file_diff(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceFileDiff, DomainError> {
        git::get_file_diff(workspace, relative_path, staged).await
    }

    async fn stage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError> {
        git::stage_file(workspace, relative_path).await
    }

    async fn unstage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError> {
        git::unstage_file(workspace, relative_path).await
    }

    async fn discard_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError> {
        git::discard_file(workspace, relative_path).await
    }

    async fn commit(
        &self,
        workspace: &ExecutionWorkspace,
        message: &str,
    ) -> Result<String, DomainError> {
        git::commit(workspace, message).await
    }
}
