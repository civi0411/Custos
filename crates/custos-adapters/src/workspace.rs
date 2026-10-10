//! Local & Worktree Workspace Host Adapter
//!
//! Handles physical directory allocations, Git worktree provisioning,
//! post-create setup scripts, and environment teardown.

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
            return Err(DomainError::Validation("Workspace path cannot be empty".into()));
        }

        match kind {
            WorkspaceKind::Folder { .. } => Ok(()),
            WorkspaceKind::Git { repo_path, branch, .. } => {
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
            WorkspaceKind::RemoteSsh { host, remote_path, .. } => {
                if host.trim().is_empty() {
                    return Err(DomainError::Validation("Remote SSH host cannot be empty".into()));
                }
                if remote_path.trim().is_empty() {
                    return Err(DomainError::Validation("Remote SSH path cannot be empty".into()));
                }
                Ok(())
            }
        }
    }

    async fn provision(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        workspace.validate_path_safety()?;

        match &workspace.kind {
            WorkspaceKind::Folder { .. } => {
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

                let branch_exists = check_branch_cmd
                    .output()
                    .await
                    .map(|out| out.status.success())
                    .unwrap_or(false);

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
                    if let Some(commit) = base_commit {
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
            WorkspaceKind::RemoteSsh { host, .. } => {
                Err(DomainError::Validation(format!(
                    "Remote SSH provisioning for host '{}' not yet supported",
                    host
                )))
            }
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

        let output = cmd.output().await.map_err(|e| {
            DomainError::Validation(format!("Failed to spawn setup script: {}", e))
        })?;

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
                if !target.exists() {
                    return Ok(DirtyManifest {
                        is_dirty: false,
                        modified_files: Vec::new(),
                        untracked_files: Vec::new(),
                        deleted_files: Vec::new(),
                        head_commit: None,
                        checked_at: chrono::Utc::now().timestamp_millis(),
                    });
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
                if !target.exists() {
                    return Ok(String::new());
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
        workspace.validate_path_safety()?;

        match &workspace.kind {
            WorkspaceKind::Folder { .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::WorkspaceId;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_folder_workspace_provision_setup_teardown() {
        let tmp = tempdir().unwrap();
        let folder_path = tmp.path().join("research_run");
        let path_str = folder_path.to_str().unwrap().to_string();

        let provider = LocalWorkspaceProvider::new();
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "research-experiment-1",
            WorkspaceKind::Folder {
                path: path_str.clone(),
            },
            path_str.clone(),
        );

        // 1. Validate
        assert!(provider.validate(&ws.kind, &ws.path).await.is_ok());

        // 2. Provision
        assert!(!folder_path.exists());
        provider.provision(&ws).await.unwrap();
        assert!(folder_path.exists());
        assert!(folder_path.is_dir());

        // 3. Setup hook
        let setup_script = "echo 'ready' > status.txt";
        provider.setup(&ws, Some(setup_script)).await.unwrap();
        assert!(folder_path.join("status.txt").exists());

        // 4. Teardown
        provider.teardown(&ws).await.unwrap();
        assert!(!folder_path.exists());
    }

    #[tokio::test]
    async fn test_validate_empty_path_fails() {
        let provider = LocalWorkspaceProvider::new();
        let kind = WorkspaceKind::Folder { path: "".into() };
        assert!(provider.validate(&kind, "  ").await.is_err());
    }

    async fn run_git_cmd(args: &[&str], cwd: &Path) {
        let mut c = Command::new("git");
        c.args(args).current_dir(cwd);
        c.output().await.unwrap();
    }

    #[tokio::test]
    async fn test_git_worktree_provision_and_teardown() {
        let tmp = tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).await.unwrap();

        run_git_cmd(&["init"], &repo_dir).await;
        run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
        run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
        let dummy_file = repo_dir.join("README.md");
        fs::write(&dummy_file, b"initial").await.unwrap();
        run_git_cmd(&["add", "README.md"], &repo_dir).await;
        run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

        let worktree_dir = tmp.path().join("worktree_1");
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "git-worktree-test",
            WorkspaceKind::Git {
                repo_path: repo_dir.to_str().unwrap().into(),
                branch: "feat-branch".into(),
                base_commit: None,
            },
            worktree_dir.to_str().unwrap(),
        );

        let provider = LocalWorkspaceProvider::new();
        provider.validate(&ws.kind, &ws.path).await.unwrap();
        provider.provision(&ws).await.unwrap();

        assert!(worktree_dir.exists());
        assert!(worktree_dir.join("README.md").exists());

        // Teardown
        provider.teardown(&ws).await.unwrap();
        assert!(!worktree_dir.exists());
    }

    #[tokio::test]
    async fn test_git_worktree_teardown_refuses_when_dirty() {
        let tmp = tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).await.unwrap();

        run_git_cmd(&["init"], &repo_dir).await;
        run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
        run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
        let dummy_file = repo_dir.join("README.md");
        fs::write(&dummy_file, b"initial").await.unwrap();
        run_git_cmd(&["add", "README.md"], &repo_dir).await;
        run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

        let worktree_dir = tmp.path().join("worktree_dirty");
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "git-worktree-dirty-test",
            WorkspaceKind::Git {
                repo_path: repo_dir.to_str().unwrap().into(),
                branch: "dirty-branch".into(),
                base_commit: None,
            },
            worktree_dir.to_str().unwrap(),
        );

        let provider = LocalWorkspaceProvider::new();
        provider.validate(&ws.kind, &ws.path).await.unwrap();
        provider.provision(&ws).await.unwrap();

        assert!(worktree_dir.exists());

        // Introduce untracked dirty file
        let dirty_file = worktree_dir.join("scratch.txt");
        fs::write(&dirty_file, b"uncommitted changes").await.unwrap();

        // Teardown must be refused safely
        let res = provider.teardown(&ws).await;
        assert!(res.is_err());
        assert!(worktree_dir.exists());
        assert!(dirty_file.exists());
    }

    #[tokio::test]
    async fn test_git_worktree_provision_refuses_resetting_existing_branch_with_base_commit() {
        let tmp = tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).await.unwrap();

        run_git_cmd(&["init"], &repo_dir).await;
        run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
        run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
        let dummy_file = repo_dir.join("README.md");
        fs::write(&dummy_file, b"initial").await.unwrap();
        run_git_cmd(&["add", "README.md"], &repo_dir).await;
        run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

        // Create an existing branch "feature-existing"
        run_git_cmd(&["branch", "feature-existing"], &repo_dir).await;

        let worktree_dir = tmp.path().join("worktree_clobber");
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "git-worktree-clobber-test",
            WorkspaceKind::Git {
                repo_path: repo_dir.to_str().unwrap().into(),
                branch: "feature-existing".into(),
                base_commit: Some("HEAD".into()),
            },
            worktree_dir.to_str().unwrap(),
        );

        let provider = LocalWorkspaceProvider::new();
        // Allocation must be refused because feature-existing already exists and base_commit was specified
        let res = provider.provision(&ws).await;
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("refusing to reset branch to base_commit"));
    }

    #[tokio::test]
    async fn test_git_worktree_diff_and_porcelain_z() {
        let tmp = tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).await.unwrap();

        run_git_cmd(&["init"], &repo_dir).await;
        run_git_cmd(&["config", "user.name", "TestUser"], &repo_dir).await;
        run_git_cmd(&["config", "user.email", "test@test.com"], &repo_dir).await;
        let dummy_file = repo_dir.join("README.md");
        fs::write(&dummy_file, b"line 1\n").await.unwrap();
        run_git_cmd(&["add", "README.md"], &repo_dir).await;
        run_git_cmd(&["commit", "-m", "initial commit"], &repo_dir).await;

        let worktree_dir = tmp.path().join("worktree_diff");
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "git-worktree-diff-test",
            WorkspaceKind::Git {
                repo_path: repo_dir.to_str().unwrap().into(),
                branch: "diff-branch".into(),
                base_commit: None,
            },
            worktree_dir.to_str().unwrap(),
        );

        let provider = LocalWorkspaceProvider::new();
        provider.provision(&ws).await.unwrap();

        // Modify README in worktree
        let wt_file = worktree_dir.join("README.md");
        fs::write(&wt_file, b"line 1\nline 2 added\n").await.unwrap();

        // Inspect dirty
        let dirty = provider.inspect_dirty(&ws).await.unwrap();
        assert!(dirty.is_dirty);
        assert_eq!(dirty.modified_files, vec!["README.md"]);

        // Diff
        let diff = provider.diff(&ws).await.unwrap();
        assert!(diff.contains("+line 2 added"));
    }

    #[tokio::test]
    async fn test_folder_workspace_teardown_refuses_shallow_path() {
        let provider = LocalWorkspaceProvider::new();
        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "root-dir-test",
            WorkspaceKind::Folder {
                path: "/tmp".into(),
            },
            "/tmp",
        );

        let res = provider.teardown(&ws).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("shallow"));
    }
}

