//! Local Git worktree provisioning and removal implementing [`RepositoryGitPort`].

use custos_core::contracts::workspace::RepositoryGitPort;
use custos_domain::DomainError;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct LocalRepositoryGitAdapter;

impl LocalRepositoryGitAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl RepositoryGitPort for LocalRepositoryGitAdapter {
    fn add_worktree(&self, repo_dir: &Path, worktree_path: &Path) -> Result<(), DomainError> {
        let output = std::process::Command::new("git")
            .current_dir(repo_dir)
            .args([
                "worktree",
                "add",
                "--detach",
                &worktree_path.to_string_lossy(),
                "HEAD",
            ])
            .output()
            .map_err(|e| {
                DomainError::Validation(format!("Failed to start Git worktree provisioning: {e}"))
            })?;
        if !output.status.success() {
            return Err(DomainError::Validation(format!(
                "Git worktree provisioning failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn remove_worktree(&self, repo_dir: &Path, worktree_path: &Path) -> Result<(), DomainError> {
        let output = std::process::Command::new("git")
            .current_dir(repo_dir)
            .args(["worktree", "remove", &worktree_path.to_string_lossy()])
            .output()
            .map_err(|e| DomainError::Validation(format!("Git worktree cleanup failed: {e}")))?;
        if !output.status.success() {
            return Err(DomainError::Validation(format!(
                "Worktree was kept for review: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn diff_head(&self, worktree_path: &Path) -> Result<String, DomainError> {
        let output = std::process::Command::new("git")
            .current_dir(worktree_path)
            .args(["diff", "HEAD"])
            .output()
            .map_err(|e| DomainError::Validation(format!("Git diff failed: {e}")))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
