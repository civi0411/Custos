//! Local & Worktree Workspace Host Adapter
//!
//! Handles physical directory allocations, Git worktree provisioning,
//! post-create setup scripts, and environment teardown.

use async_trait::async_trait;
use custos_core::contracts::workspace::WorkspaceProvider;
use custos_domain::{DomainError, ExecutionWorkspace, WorkspaceKind};
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

                // Run git worktree add
                let mut cmd = Command::new("git");
                cmd.current_dir(repo_path);
                cmd.arg("worktree").arg("add").arg("-B").arg(branch).arg(&workspace.path);

                if let Some(commit) = base_commit {
                    cmd.arg(commit);
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

    async fn teardown(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        match &workspace.kind {
            WorkspaceKind::Folder { .. } => {
                let target = Path::new(&workspace.path);
                if target.exists() {
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
                let mut cmd = Command::new("git");
                cmd.current_dir(repo_path);
                cmd.arg("worktree").arg("remove").arg("--force").arg(&workspace.path);

                let output = cmd.output().await;
                if let Ok(out) = output {
                    if !out.status.success() {
                        let target = Path::new(&workspace.path);
                        if target.exists() {
                            let _ = fs::remove_dir_all(target).await;
                        }
                    }
                } else {
                    let target = Path::new(&workspace.path);
                    if target.exists() {
                        let _ = fs::remove_dir_all(target).await;
                    }
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
}
