//! Repository Snapshot Provenance
//!
//! Captures the immutable identity of a worktree snapshot, HEAD commit,
//! and dirty worktree modifications to ensure absolute context reproducibility.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Immutable pointer to a specific repository workspace state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSnapshotRef {
    /// Logical repository identifier (e.g. "custos", "app-backend").
    pub repo_id: String,
    /// Worktree or branch workspace identifier (e.g. "main", "wt-agent-42").
    pub worktree_id: String,
    /// Full SHA-1/SHA-256 hash of the Git HEAD commit.
    pub head_commit: String,
    /// Relative path -> SHA-256 hash of uncommitted/dirty workspace files.
    pub dirty_hashes: BTreeMap<String, String>,
    /// Generation version counter of the derived index.
    pub index_generation: u64,
}

impl RepoSnapshotRef {
    pub fn new(
        repo_id: impl Into<String>,
        worktree_id: impl Into<String>,
        head_commit: impl Into<String>,
    ) -> Self {
        Self {
            repo_id: repo_id.into(),
            worktree_id: worktree_id.into(),
            head_commit: head_commit.into(),
            dirty_hashes: BTreeMap::new(),
            index_generation: 1,
        }
    }

    pub fn with_dirty_file(
        mut self,
        rel_path: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Self {
        self.dirty_hashes.insert(rel_path.into(), sha256.into());
        self
    }

    pub fn is_clean(&self) -> bool {
        self.dirty_hashes.is_empty()
    }

    /// Check if a specific file has changed or is dirty compared to this snapshot.
    pub fn is_file_dirty(&self, rel_path: &str, current_hash: Option<&str>) -> bool {
        match (self.dirty_hashes.get(rel_path), current_hash) {
            (Some(expected), Some(actual)) => expected != actual,
            (Some(_), None) => true,
            (None, Some(_)) => true, // was clean, now modified
            (None, None) => false,
        }
    }
}
