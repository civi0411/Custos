//! Local & Worktree Workspace Host Adapter
//!
//! Handles physical directory allocations, Git worktree provisioning,
//! post-create setup scripts, and environment teardown.

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use custos_core::contracts::workspace::WorkspaceProvider;
use custos_domain::{DirtyManifest, DomainError, ExecutionWorkspace, WorkspaceKind};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

#[derive(Debug, Clone, Default)]
pub struct LocalWorkspaceProvider;

impl LocalWorkspaceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkspaceProvider for LocalWorkspaceProvider {
    async fn validate(&self, kind: &WorkspaceKind, path: &str) -> Result<(), DomainError> {
        if path.trim().is_empty() {
            return Err(DomainError::Validation(
                "Workspace path cannot be empty".into(),
            ));
        }

        match kind {
            WorkspaceKind::Folder { .. } => Ok(()),
            WorkspaceKind::Git {
                repo_path, branch, ..
            } => {
                let repo = Path::new(repo_path);
                if !repo.exists() {
                    return Err(DomainError::Validation(format!(
                        "Git repo path '{}' does not exist",
                        repo_path
                    )));
                }
                if branch.trim().is_empty() {
                    return Err(DomainError::Validation("Git branch cannot be empty".into()));
                }
                Ok(())
            }
            WorkspaceKind::RemoteSsh {
                host, remote_path, ..
            } => {
                if host.trim().is_empty() {
                    return Err(DomainError::Validation(
                        "Remote SSH host cannot be empty".into(),
                    ));
                }
                if remote_path.trim().is_empty() {
                    return Err(DomainError::Validation(
                        "Remote SSH path cannot be empty".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    async fn provision(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        match &workspace.kind {
            WorkspaceKind::Folder { .. } => {
                workspace.validate_path_safety()?;
                fs::create_dir_all(&workspace.path).await.map_err(|e| {
                    DomainError::Validation(format!(
                        "Failed to create workspace directory '{}': {}",
                        workspace.path, e
                    ))
                })?;
                Ok(())
            }
            WorkspaceKind::Git {
                repo_path,
                branch,
                base_commit,
            } => {
                let target_path = Path::new(&workspace.path);
                let main_repo = Path::new(repo_path);
                let is_base_repo = target_path == main_repo
                    || matches!((target_path.canonicalize(), main_repo.canonicalize()), (Ok(target), Ok(repo)) if target == repo);

                if is_base_repo {
                    if workspace.path.trim().is_empty() || workspace.path == "/" {
                        return Err(DomainError::Validation(
                            "Workspace path cannot be root".into(),
                        ));
                    }
                    if !main_repo.exists() {
                        return Err(DomainError::Validation(format!(
                            "Base repository '{}' does not exist",
                            repo_path
                        )));
                    }
                    let root = Command::new("git")
                        .current_dir(main_repo)
                        .args(["rev-parse", "--show-toplevel"])
                        .output()
                        .await
                        .map_err(|error| {
                            DomainError::Validation(format!("Cannot inspect Git checkout: {error}"))
                        })?;
                    if !root.status.success() {
                        return Err(DomainError::Validation(
                            "Existing directory is not a Git checkout".into(),
                        ));
                    }
                    let root_text = String::from_utf8_lossy(&root.stdout);
                    let observed_root =
                        Path::new(root_text.trim())
                            .canonicalize()
                            .map_err(|error| {
                                DomainError::Validation(format!("Cannot resolve Git root: {error}"))
                            })?;
                    let expected_root = main_repo.canonicalize().map_err(|error| {
                        DomainError::Validation(format!("Cannot resolve repository path: {error}"))
                    })?;
                    if observed_root != expected_root {
                        return Err(DomainError::Validation(
                            "Register the actual Git checkout root, not a subdirectory".into(),
                        ));
                    }
                    let observed = Command::new("git")
                        .current_dir(main_repo)
                        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
                        .output()
                        .await
                        .map_err(|error| {
                            DomainError::Validation(format!(
                                "Cannot inspect checkout branch: {error}"
                            ))
                        })?;
                    let actual = if observed.status.success() {
                        String::from_utf8_lossy(&observed.stdout).trim().to_string()
                    } else if observed.status.code() == Some(1) {
                        "HEAD".to_string()
                    } else {
                        return Err(DomainError::Validation(
                            "Cannot determine checkout branch".into(),
                        ));
                    };
                    if branch != &actual {
                        return Err(DomainError::Conflict(format!("Checkout is on '{actual}', not declared branch '{branch}'; registration never switches branches")));
                    }
                    return Ok(());
                }

                workspace.validate_path_safety()?;

                let checked = Command::new("git")
                    .current_dir(repo_path)
                    .args(["check-ref-format", "--branch", branch])
                    .output()
                    .await
                    .map_err(|error| {
                        DomainError::Validation(format!("Cannot validate Git branch: {error}"))
                    })?;
                if branch.starts_with('-') || branch.contains("@{") || !checked.status.success() {
                    return Err(DomainError::Validation(
                        "Invalid literal Git branch name".into(),
                    ));
                }
                let resolved_base = if let Some(base) = base_commit {
                    let resolved = Command::new("git")
                        .current_dir(repo_path)
                        .args([
                            "rev-parse",
                            "--verify",
                            "--end-of-options",
                            &format!("{base}^{{commit}}"),
                        ])
                        .output()
                        .await
                        .map_err(|error| {
                            DomainError::Validation(format!(
                                "Cannot resolve worktree base: {error}"
                            ))
                        })?;
                    if !resolved.status.success() {
                        return Err(DomainError::Validation(
                            "Worktree base does not resolve to a commit".into(),
                        ));
                    }
                    Some(String::from_utf8_lossy(&resolved.stdout).trim().to_string())
                } else {
                    None
                };

                if target_path.exists() {
                    return Err(DomainError::Conflict(format!(
                        "Target worktree path '{}' already exists",
                        workspace.path
                    )));
                }

                // Ensure parent directory of target worktree exists
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        DomainError::Validation(format!(
                            "Failed to create parent directory for worktree: {}",
                            e
                        ))
                    })?;
                }

                // B3 Safety: Check if the branch already exists in the repository
                let mut check_branch_cmd = Command::new("git");
                check_branch_cmd
                    .current_dir(repo_path)
                    .arg("rev-parse")
                    .arg("--verify")
                    .arg(format!("refs/heads/{}", branch));

                let branch_probe = check_branch_cmd.output().await.map_err(|error| {
                    DomainError::Validation(format!("Cannot inspect existing branch: {error}"))
                })?;
                let branch_exists = branch_probe.status.success();

                let mut cmd = Command::new("git");
                cmd.current_dir(repo_path);
                cmd.arg("worktree").arg("add");

                if branch_exists {
                    // Refuse to reset or clobber an existing branch when base_commit is specified
                    if base_commit.is_some() {
                        return Err(DomainError::Conflict(format!(
                            "Branch '{}' already exists in repository; refusing to reset branch to base_commit during worktree allocation",
                            branch
                        )));
                    }
                    // Non-resetting attachment: check out existing branch without -b / -B
                    // (Git will fail cleanly if the branch is already checked out elsewhere)
                    cmd.arg(&workspace.path).arg(branch);
                } else {
                    // Unique new branch creation with -b (never destructive -B)
                    cmd.arg("-b").arg(branch).arg(&workspace.path);
                    if let Some(commit) = resolved_base {
                        cmd.arg(commit);
                    }
                }

                let output = cmd.output().await.map_err(|e| {
                    DomainError::Validation(format!("Failed to execute git command: {}", e))
                })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(DomainError::Validation(format!(
                        "git worktree add failed: {}",
                        stderr.trim()
                    )));
                }

                Ok(())
            }
            WorkspaceKind::RemoteSsh { host, .. } => Err(DomainError::Validation(format!(
                "Remote SSH provisioning for host '{}' not yet supported",
                host
            ))),
        }
    }

    async fn setup(
        &self,
        workspace: &ExecutionWorkspace,
        script: Option<&str>,
    ) -> Result<(), DomainError> {
        workspace.validate_path_safety()?;

        let script = match script {
            Some(s) if !s.trim().is_empty() => s,
            _ => return Ok(()),
        };

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(script).current_dir(&workspace.path);

        let output = cmd
            .output()
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to spawn setup script: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DomainError::Validation(format!(
                "Setup script failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    async fn inspect_dirty(
        &self,
        workspace: &ExecutionWorkspace,
    ) -> Result<DirtyManifest, DomainError> {
        match &workspace.kind {
            WorkspaceKind::Git { .. } => {
                let target = Path::new(&workspace.path);
                if !target.is_dir() {
                    return Err(DomainError::Validation(format!(
                        "Git workspace is missing: {}",
                        workspace.path
                    )));
                }

                // Run git status --porcelain=v1 -z for robust null-delimited parsing
                let mut status_cmd = Command::new("git");
                status_cmd
                    .current_dir(&workspace.path)
                    .arg("status")
                    .arg("--porcelain=v1")
                    .arg("-z");

                let output = status_cmd.output().await.map_err(|e| {
                    DomainError::Validation(format!("Failed to execute git status: {}", e))
                })?;

                if !output.status.success() {
                    return Err(DomainError::Validation(format!(
                        "git status failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    )));
                }
                let bytes = output.stdout;
                let mut modified_files = Vec::new();
                let mut untracked_files = Vec::new();
                let mut deleted_files = Vec::new();

                let mut i = 0;
                while i < bytes.len() {
                    let end = bytes[i..]
                        .iter()
                        .position(|&b| b == 0)
                        .map(|pos| i + pos)
                        .unwrap_or(bytes.len());
                    let entry = String::from_utf8_lossy(&bytes[i..end]).to_string();
                    i = end + 1;

                    if entry.len() >= 3 {
                        let status = &entry[..2];
                        let file_path = entry[3..].trim().to_string();

                        if status.contains('?') {
                            untracked_files.push(file_path);
                        } else if status.contains('D') {
                            deleted_files.push(file_path);
                        } else {
                            modified_files.push(file_path);
                        }

                        // Rename entries in -z format have an extra NUL-terminated path
                        if status.starts_with('R') && i < bytes.len() {
                            let orig_end = bytes[i..]
                                .iter()
                                .position(|&b| b == 0)
                                .map(|pos| i + pos)
                                .unwrap_or(bytes.len());
                            i = orig_end + 1;
                        }
                    }
                }

                let is_dirty = !modified_files.is_empty()
                    || !untracked_files.is_empty()
                    || !deleted_files.is_empty();

                // Get HEAD commit hash
                let mut rev_cmd = Command::new("git");
                rev_cmd
                    .current_dir(&workspace.path)
                    .arg("rev-parse")
                    .arg("HEAD");
                let head_commit = rev_cmd
                    .output()
                    .await
                    .ok()
                    .filter(|out| out.status.success())
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());

                Ok(DirtyManifest {
                    is_dirty,
                    modified_files,
                    untracked_files,
                    deleted_files,
                    head_commit,
                    checked_at: chrono::Utc::now().timestamp_millis(),
                })
            }
            WorkspaceKind::Folder { path } => {
                let _exists = Path::new(path).exists();
                Ok(DirtyManifest {
                    is_dirty: false,
                    modified_files: Vec::new(),
                    untracked_files: Vec::new(),
                    deleted_files: Vec::new(),
                    head_commit: None,
                    checked_at: chrono::Utc::now().timestamp_millis(),
                })
            }
            WorkspaceKind::RemoteSsh { .. } => Ok(DirtyManifest {
                is_dirty: false,
                modified_files: Vec::new(),
                untracked_files: Vec::new(),
                deleted_files: Vec::new(),
                head_commit: None,
                checked_at: chrono::Utc::now().timestamp_millis(),
            }),
        }
    }

    async fn diff(&self, workspace: &ExecutionWorkspace) -> Result<String, DomainError> {
        match &workspace.kind {
            WorkspaceKind::Git { base_commit, .. } => {
                let target = Path::new(&workspace.path);
                if !target.is_dir() {
                    return Err(DomainError::Validation(format!(
                        "Git workspace is missing: {}",
                        workspace.path
                    )));
                }

                let mut cmd = Command::new("git");
                cmd.current_dir(&workspace.path).arg("diff");
                if let Some(base) = base_commit {
                    cmd.arg(base);
                }

                let output = cmd.output().await.map_err(|e| {
                    DomainError::Validation(format!("Failed to execute git diff: {}", e))
                })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(DomainError::Validation(format!(
                        "git diff failed: {}",
                        stderr.trim()
                    )));
                }

                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            _ => Ok(String::new()),
        }
    }

    async fn recover(&self, workspace: &ExecutionWorkspace) -> Result<bool, DomainError> {
        let target = Path::new(&workspace.path);
        if !target.exists() {
            return Ok(false);
        }

        match &workspace.kind {
            WorkspaceKind::Git { .. } => {
                let mut cmd = Command::new("git");
                cmd.current_dir(&workspace.path)
                    .arg("rev-parse")
                    .arg("--is-inside-work-tree");

                match cmd.output().await {
                    Ok(out) if out.status.success() => Ok(true),
                    _ => Ok(false),
                }
            }
            WorkspaceKind::Folder { .. } => Ok(true),
            WorkspaceKind::RemoteSsh { .. } => Ok(true),
        }
    }

    async fn teardown(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        match &workspace.kind {
            WorkspaceKind::Folder { .. } => {
                workspace.validate_path_safety()?;
                let target = Path::new(&workspace.path);
                if target.exists() {
                    // Refuse to delete shallow root directories
                    if target.components().count() <= 2 {
                        return Err(DomainError::Validation(format!(
                            "Refusing to delete shallow directory path: {}",
                            workspace.path
                        )));
                    }
                    fs::remove_dir_all(target).await.map_err(|e| {
                        DomainError::Validation(format!(
                            "Failed to remove workspace directory '{}': {}",
                            workspace.path, e
                        ))
                    })?;
                }
                Ok(())
            }
            WorkspaceKind::Git { repo_path, .. } => {
                let target_path = Path::new(&workspace.path);
                let main_repo = Path::new(repo_path);
                let is_base_repo = target_path == main_repo
                    || target_path.canonicalize().ok() == main_repo.canonicalize().ok();

                if is_base_repo {
                    // Safety: Do not remove the user's primary repository working directory!
                    return Ok(());
                }

                workspace.validate_path_safety()?;

                // B0 Safety: Check dirty state before attempting worktree removal
                let dirty = self.inspect_dirty(workspace).await?;
                if dirty.is_dirty {
                    return Err(DomainError::Conflict(format!(
                        "Refusing teardown of git worktree '{}': uncommitted or untracked changes present",
                        workspace.path
                    )));
                }

                let mut cmd = Command::new("git");
                cmd.current_dir(repo_path);
                // B0 Safety: Do NOT use --force, so git preserves modifications safely
                cmd.arg("worktree").arg("remove").arg(&workspace.path);

                let output = cmd.output().await.map_err(|e| {
                    DomainError::Validation(format!("Failed to spawn git worktree remove: {}", e))
                })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(DomainError::Validation(format!(
                        "git worktree remove failed: {}. Refusing recursive directory removal fallback.",
                        stderr.trim()
                    )));
                }

                Ok(())
            }
            WorkspaceKind::RemoteSsh { .. } => Ok(()),
        }
    }
}
