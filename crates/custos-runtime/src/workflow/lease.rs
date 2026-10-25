//! Workspace Lease & Isolation Runtime (RFC 006 / OrCa Worktree Model)
//!
//! Enforces worktree isolation per DAG task node to ensure absolute zero-cross-contamination.
//! Changes from tasks are committed back to the main workspace via bounded write prefixes.

use custos_core::contracts::workspace::RepositoryGitPort;
use custos_domain::{new_id, DomainError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Represents an isolated execution worktree bound to a specific Task and Node ID.
#[derive(Debug, Clone)]
pub struct WorkspaceLease {
    pub lease_id: String,
    pub task_id: String,
    pub node_id: String,
    pub base_path: PathBuf,
    pub lease_path: PathBuf,
    pub active: bool,
}

impl WorkspaceLease {
    pub fn lease_id(&self) -> &str {
        &self.lease_id
    }

    pub fn lease_path(&self) -> &Path {
        &self.lease_path
    }

    pub fn path(&self) -> &Path {
        &self.lease_path
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Releases and marks the lease inactive using a default no-op git adapter.
    pub fn release(&mut self) -> Result<(), DomainError> {
        self.release_with_git(&NoopGitPort)
    }

    /// Releases and marks the lease inactive using the provided Git port.
    pub fn release_with_git(&mut self, git_port: &dyn RepositoryGitPort) -> Result<(), DomainError> {
        let expected_parent = self.base_path.join(".custos").join("worktrees");
        if !self.lease_path.starts_with(&expected_parent) || self.lease_path == expected_parent {
            return Err(DomainError::Validation(format!(
                "Refusing to release lease outside managed worktree directory: {:?}",
                self.lease_path
            )));
        }
        if self.lease_path.exists() {
            if self.lease_path.join(".git").exists() {
                git_port.remove_worktree(&self.base_path, &self.lease_path)?;
            } else {
                // Non-Git leases can only be removed when empty. Never erase files
                // merely because a worker stopped or cancellation was requested.
                fs::remove_dir(&self.lease_path).map_err(|e| {
                    DomainError::Validation(format!(
                        "Non-empty lease was kept for review at {:?}: {e}",
                        self.lease_path
                    ))
                })?;
            }
        }
        self.active = false;
        Ok(())
    }

    /// Scans the isolated lease path for modified or newly created files relative to lease root.
    pub fn collect_modified_files(&self) -> Result<Vec<PathBuf>, DomainError> {
        let mut modified = Vec::new();
        self.scan_dir(&self.lease_path, &mut modified)?;
        Ok(modified)
    }

    fn scan_dir(&self, current: &Path, acc: &mut Vec<PathBuf>) -> Result<(), DomainError> {
        if !current.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(current)
            .map_err(|e| DomainError::Validation(format!("Failed to read dir {:?}: {}", current, e)))?
        {
            let entry = entry.map_err(|e| DomainError::Validation(e.to_string()))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            if name_str == ".git" || name_str == ".custos" || name_str == "target" {
                continue;
            }

            if path.is_dir() {
                self.scan_dir(&path, acc)?;
            } else {
                if let Ok(rel) = path.strip_prefix(&self.lease_path) {
                    acc.push(rel.to_path_buf());
                }
            }
        }
        Ok(())
    }
}

/// No-op Git operations implementation for non-git environments or default tests.
#[derive(Clone, Default)]
pub struct NoopGitPort;

impl RepositoryGitPort for NoopGitPort {
    fn add_worktree(&self, _repo_dir: &Path, worktree_path: &Path) -> Result<(), DomainError> {
        fs::create_dir_all(worktree_path.join(".git"))
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    fn remove_worktree(&self, _repo_dir: &Path, worktree_path: &Path) -> Result<(), DomainError> {
        let _ = fs::remove_dir_all(worktree_path);
        Ok(())
    }
}

/// Thread-safe manager for allocating, tracking, and merging workspace leases.
#[derive(Clone)]
pub struct WorkspaceLeaseManager {
    base_dir: PathBuf,
    active_leases: Arc<Mutex<HashMap<String, WorkspaceLease>>>,
    git_port: Arc<dyn RepositoryGitPort>,
}

impl WorkspaceLeaseManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self::with_git_port(base_dir, Arc::new(NoopGitPort))
    }

    pub fn with_git_port(
        base_dir: impl Into<PathBuf>,
        git_port: Arc<dyn RepositoryGitPort>,
    ) -> Self {
        Self {
            base_dir: base_dir.into(),
            active_leases: Arc::new(Mutex::new(HashMap::new())),
            git_port,
        }
    }

    /// Allocates an isolated lease directory for a specific task node.
    pub fn acquire_lease(
        &self,
        task_id: &str,
        node_id: &str,
    ) -> Result<WorkspaceLease, DomainError> {
        let lease_id = new_id("wls");
        let lease_path = self
            .base_dir
            .join(".custos")
            .join("worktrees")
            .join(&lease_id);

        let git_dir = self.base_dir.join(".git");
        let mut worktree_created = false;
        if git_dir.exists() {
            if let Some(parent) = lease_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            self.git_port.add_worktree(&self.base_dir, &lease_path)?;
            worktree_created = true;
        }

        if !worktree_created {
            fs::create_dir_all(&lease_path).map_err(|e| {
                DomainError::Validation(format!(
                    "Failed to create workspace lease dir at {:?}: {}",
                    lease_path, e
                ))
            })?;
        }

        let lease = WorkspaceLease {
            lease_id: lease_id.clone(),
            task_id: task_id.to_string(),
            node_id: node_id.to_string(),
            base_path: self.base_dir.clone(),
            lease_path,
            active: true,
        };

        let mut lock = self.active_leases.lock().map_err(|e| {
            DomainError::Validation(format!("WorkspaceLeaseManager poisoned: {}", e))
        })?;
        lock.insert(lease_id, lease.clone());

        Ok(lease)
    }

    /// Merges modified files from an isolated worktree back into the base workspace,
    /// enforcing that all changes are strictly within the permitted write prefix
    /// declared by the Graph node (Gate 5 Isolation Defense).
    pub fn merge_lease(
        &self,
        lease: &WorkspaceLease,
        permitted_prefix: Option<&str>,
    ) -> Result<Vec<PathBuf>, DomainError> {
        if !lease.active {
            return Err(DomainError::Validation(format!(
                "Cannot merge inactive lease '{}'",
                lease.lease_id
            )));
        }

        let modified = lease.collect_modified_files()?;
        let mut merged = Vec::new();

        for rel_path in &modified {
            // Guard: Traversal check
            for comp in rel_path.components() {
                if matches!(comp, std::path::Component::ParentDir) {
                    return Err(DomainError::Validation(format!(
                        "Path traversal ('..') detected in lease path: {:?}",
                        rel_path
                    )));
                }
            }

            let rel_str = rel_path.to_string_lossy();
            if let Some(prefix) = permitted_prefix {
                if prefix.contains("..") {
                    return Err(DomainError::Validation(format!(
                        "Path traversal forbidden in permitted prefix: {}",
                        prefix
                    )));
                }

                let clean_prefix = prefix
                    .trim_start_matches('/')
                    .trim_end_matches("/**")
                    .trim_end_matches("/*");

                // Strict boundary check to prevent prefix-collision (e.g. "src_other" matching "src")
                let matches_prefix = if clean_prefix.is_empty() {
                    true
                } else if rel_str == clean_prefix {
                    true
                } else if rel_str.starts_with(&format!("{}/", clean_prefix)) {
                    true
                } else {
                    false
                };

                if !matches_prefix {
                    return Err(DomainError::Validation(format!(
                        "Gate 5 Violation: Worktree lease '{}' attempted to write to unauthorized path '{}' outside permitted prefix '{}'",
                        lease.lease_id, rel_str, prefix
                    )));
                }
            }

            let src_file = lease.lease_path.join(rel_path);
            let dst_file = self.base_dir.join(rel_path);

            if let Some(parent) = dst_file.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    DomainError::Validation(format!("Failed to create parent dir: {}", e))
                })?;
            }

            fs::copy(&src_file, &dst_file).map_err(|e| {
                DomainError::Validation(format!(
                    "Failed to merge file from {:?} to {:?}: {}",
                    src_file, dst_file, e
                ))
            })?;

            merged.push(rel_path.clone());
        }

        Ok(merged)
    }

    /// Retrieves a clone of an active lease by ID, if present.
    pub fn get_lease(&self, lease_id: &str) -> Option<WorkspaceLease> {
        let lock = self.active_leases.lock().ok()?;
        lock.get(lease_id).cloned()
    }

    /// Releases a lease by ID and cleans up its storage.
    pub fn release_lease(&self, lease_id: &str) -> Result<(), DomainError> {
        let mut lock = self.active_leases.lock().map_err(|e| {
            DomainError::Validation(format!("WorkspaceLeaseManager poisoned: {}", e))
        })?;
        if let Some(lease) = lock.get_mut(lease_id) {
            lease.release_with_git(&*self.git_port)?;
            lock.remove(lease_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFailingGitPort;

    impl RepositoryGitPort for MockFailingGitPort {
        fn add_worktree(&self, _repo_dir: &Path, worktree_path: &Path) -> Result<(), DomainError> {
            fs::create_dir_all(worktree_path.join(".git")).unwrap();
            Ok(())
        }

        fn remove_worktree(
            &self,
            _repo_dir: &Path,
            _worktree_path: &Path,
        ) -> Result<(), DomainError> {
            Err(DomainError::Validation(
                "Worktree was kept for review: untracked files present".into(),
            ))
        }
    }

    #[test]
    fn test_acquire_and_merge_workspace_lease() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = WorkspaceLeaseManager::new(temp_dir.path());

        // Acquire lease for node A
        let lease_a = manager.acquire_lease("task_1", "node_a").unwrap();
        assert!(lease_a.lease_path.exists());
        assert!(lease_a.is_active());

        // Write file inside lease A
        let file_a = lease_a.lease_path.join("src").join("lib.rs");
        fs::create_dir_all(file_a.parent().unwrap()).unwrap();
        fs::write(&file_a, "pub fn hello() -> bool { true }").unwrap();

        // Merge lease with permitted prefix "src"
        let merged = manager.merge_lease(&lease_a, Some("src/**")).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], PathBuf::from("src/lib.rs"));

        // Verify file exists in base workspace
        let target_file = temp_dir.path().join("src").join("lib.rs");
        assert!(target_file.exists());
        let content = fs::read_to_string(&target_file).unwrap();
        assert_eq!(content, "pub fn hello() -> bool { true }");

        // Attempting to merge outside prefix fails
        let illegal_file = lease_a.lease_path.join("etc").join("passwd");
        fs::create_dir_all(illegal_file.parent().unwrap()).unwrap();
        fs::write(&illegal_file, "forbidden").unwrap();

        let err = manager.merge_lease(&lease_a, Some("src/**"));
        assert!(err.is_err(), "Must reject write outside permitted prefix");

        // A completed worker must not erase files that still require review.
        let release = manager.release_lease(&lease_a.lease_id);
        assert!(release.is_err());
        assert!(lease_a.lease_path.exists());
        assert!(manager.get_lease(&lease_a.lease_id).is_some());
    }

    #[test]
    fn test_prefix_collision_and_traversal_protection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = WorkspaceLeaseManager::new(temp_dir.path());

        let lease = manager.acquire_lease("task_1", "node_sec").unwrap();

        // 1. Prefix collision: file in "src_other/file.rs" must NOT match prefix "src"
        let collision_file = lease.lease_path.join("src_other").join("file.rs");
        fs::create_dir_all(collision_file.parent().unwrap()).unwrap();
        fs::write(&collision_file, "malicious").unwrap();

        let err_collision = manager.merge_lease(&lease, Some("src"));
        assert!(
            err_collision.is_err(),
            "Must prevent prefix collision (src_other vs src)"
        );

        // 2. Traversal in prefix: "src/../../etc" must be rejected
        let err_traversal_prefix = manager.merge_lease(&lease, Some("src/../../etc"));
        assert!(
            err_traversal_prefix.is_err(),
            "Must reject traversal in prefix"
        );
    }

    #[test]
    fn dirty_git_worktree_survives_release() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp_dir.path().join(".git")).unwrap();
        fs::write(temp_dir.path().join("README.md"), "baseline").unwrap();

        let manager = WorkspaceLeaseManager::with_git_port(
            temp_dir.path(),
            Arc::new(MockFailingGitPort),
        );
        let lease = manager.acquire_lease("task", "worker").unwrap();
        let new_file = lease.lease_path.join("untracked.txt");
        fs::write(&new_file, "keep this result").unwrap();

        assert!(manager.release_lease(&lease.lease_id).is_err());
        assert_eq!(fs::read_to_string(&new_file).unwrap(), "keep this result");
        assert!(manager.get_lease(&lease.lease_id).is_some());
    }
}
