//! Workspace Lease and Isolated Worktree Manager (RFC 004 §2C, G10)
//!
//! Provides isolated working directories for concurrent worker nodes (T4 Worktree)
//! to prevent concurrent write collisions before integration merge.

use custos_domain::ids::new_id;
use custos_domain::DomainError;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Sovereign workspace lease granting an isolated execution worktree to a node.
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

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Releases and marks the lease inactive.
    pub fn release(&mut self) -> Result<(), DomainError> {
        self.active = false;
        if self.lease_path.exists() {
            let _ = fs::remove_dir_all(&self.lease_path);
        }
        Ok(())
    }

    /// Scans the isolated lease path for modified or newly created files relative to lease root.
    pub fn collect_modified_files(&self) -> Result<Vec<PathBuf>, DomainError> {
        let mut files = Vec::new();
        if !self.lease_path.exists() {
            return Ok(files);
        }
        Self::scan_dir(&self.lease_path, &self.lease_path, &mut files)?;
        Ok(files)
    }

    fn scan_dir(root: &Path, current: &Path, out: &mut Vec<PathBuf>) -> Result<(), DomainError> {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_dir(root, &path, out)?;
                } else if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_path_buf());
                }
            }
        }
        Ok(())
    }
}

/// Thread-safe manager for allocating, tracking, and merging workspace leases.
#[derive(Clone)]
pub struct WorkspaceLeaseManager {
    base_dir: PathBuf,
    active_leases: Arc<Mutex<HashMap<String, WorkspaceLease>>>,
}

impl WorkspaceLeaseManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            active_leases: Arc::new(Mutex::new(HashMap::new())),
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

        fs::create_dir_all(&lease_path).map_err(|e| {
            DomainError::Validation(format!(
                "Failed to create workspace lease dir at {:?}: {}",
                lease_path, e
            ))
        })?;

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
    /// enforcing that all changes are within the permitted write prefix.
    pub fn merge_lease(
        &self,
        lease: &WorkspaceLease,
        permitted_prefix: Option<&str>,
    ) -> Result<Vec<PathBuf>, DomainError> {
        let modified = lease.collect_modified_files()?;
        let mut merged = Vec::new();

        for rel_path in &modified {
            let rel_str = rel_path.to_string_lossy();
            if let Some(prefix) = permitted_prefix {
                let clean_prefix = prefix.trim_start_matches('/').trim_end_matches("/**").trim_end_matches("/*");
                if !rel_str.starts_with(clean_prefix) {
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

    /// Releases a lease by ID and cleans up its storage.
    pub fn release_lease(&self, lease_id: &str) -> Result<(), DomainError> {
        let mut lock = self.active_leases.lock().map_err(|e| {
            DomainError::Validation(format!("WorkspaceLeaseManager poisoned: {}", e))
        })?;
        if let Some(mut lease) = lock.remove(lease_id) {
            lease.release()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Release lease
        manager.release_lease(&lease_a.lease_id).unwrap();
        assert!(!lease_a.lease_path.exists());
    }
}
