//! Repository Intelligence Coordinator
//!
//! Orchestrates the 4-layer intelligence architecture:
//! 1. Snapshot identity & provenance capture (RepoSnapshotRef)
//! 2. Multi-tier query dispatch (Exact / Lexical -> Structural graph expansion)
//! 3. Stale detection (verifies dirty file hashes against the snapshot)
//! 4. Security & token-bounded context compilation

use super::graph::CodeGraph;
use super::scanner::WorkspaceScanner;
use custos_core::repo::RepoSecurityPolicy;
use custos_domain::repo::{
    ConfidenceLevel, CoverageMetrics, MissingReason, QueryResult, RepoSnapshotRef, SourceSpan,
    SymbolMatch,
};
use custos_domain::DomainError;
use std::path::PathBuf;

/// Coordinates repository intelligence queries, snapshot tracking, and stale detection.
pub struct RepoCoordinator {
    workspace_root: PathBuf,
    security_policy: RepoSecurityPolicy,
    graph: CodeGraph,
    scanner: WorkspaceScanner,
}

impl RepoCoordinator {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            security_policy: RepoSecurityPolicy::default(),
            graph: CodeGraph::new(),
            scanner: WorkspaceScanner::new(),
        }
    }

    pub fn with_security_policy(mut self, policy: RepoSecurityPolicy) -> Self {
        self.security_policy = policy;
        self
    }

    /// Access the underlying code graph.
    pub fn graph(&self) -> &CodeGraph {
        &self.graph
    }

    /// Mutable access to the underlying code graph for population.
    pub fn graph_mut(&mut self) -> &mut CodeGraph {
        &mut self.graph
    }

    /// Resolve real Git HEAD commit from workspace root if it is a Git repository.
    fn resolve_head_commit(workspace_root: &std::path::Path) -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(workspace_root)
            .output()
            .ok()?;
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !commit.is_empty() {
                return Some(commit);
            }
        }
        None
    }

    /// Capture current snapshot identity for the workspace.
    pub fn current_snapshot(&self, repo_id: &str, worktree_id: &str) -> RepoSnapshotRef {
        let head_commit = Self::resolve_head_commit(&self.workspace_root)
            .unwrap_or_else(|| "uncommitted_workspace".to_string());
        let mut snapshot = RepoSnapshotRef::new(repo_id, worktree_id, head_commit);

        if let Ok(inventory) = self.scanner.scan_inventory(&self.workspace_root) {
            for entry in inventory {
                if let Ok(content) = std::fs::read(&entry.absolute_path) {
                    let hash = blake3::hash(&content).to_hex().to_string();
                    snapshot = snapshot.with_dirty_file(&entry.relative_path, hash);
                }
            }
        }
        snapshot
    }

    /// Check if a given file has become stale compared to a query snapshot.
    pub fn is_stale(&self, snapshot: &RepoSnapshotRef, rel_path: &str) -> bool {
        let abs_path = self.workspace_root.join(rel_path);
        if !abs_path.exists() {
            return true;
        }
        if let Ok(content) = std::fs::read(&abs_path) {
            let current_hash = blake3::hash(&content).to_hex().to_string();
            snapshot.is_file_dirty(rel_path, Some(&current_hash))
        } else {
            true
        }
    }

    /// Query symbol definitions with provenance and security validation.
    pub fn find_symbol(
        &self,
        snapshot: &RepoSnapshotRef,
        symbol_name: &str,
    ) -> Result<QueryResult<Vec<SymbolMatch>>, DomainError> {
        let mut matches = Vec::new();
        let mut missing_reasons = Vec::new();

        let nodes = self.graph.find_by_name(symbol_name);

        for node in &nodes {
            if let Some(rel_path) = &node.file_path {
                let line_num = node.line_number.unwrap_or(1);
                let span =
                    SourceSpan::new(rel_path.clone(), 0, 100, line_num, line_num + 5, "digest");

                // Check security boundary
                if let Err(e) = self.security_policy.validate_span(&span) {
                    missing_reasons.push(MissingReason::AccessDenied {
                        path: rel_path.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }

                // Check staleness
                if self.is_stale(snapshot, rel_path) {
                    missing_reasons.push(MissingReason::StaleSource {
                        path: rel_path.clone(),
                    });
                }

                matches.push(SymbolMatch {
                    name: node.kind.display_name().to_string(),
                    kind: node.kind.type_name().to_string(),
                    span,
                    confidence: ConfidenceLevel::SyntacticCandidate,
                    container: None,
                    signature: None,
                    docstring: None,
                });
            }
        }

        let total_nodes = self.graph.node_count();
        let coverage = CoverageMetrics {
            files_indexed: total_nodes,
            total_files: total_nodes,
            coverage_ratio: if total_nodes > 0 { 1.0 } else { 0.0 },
            has_unindexed_changes: !snapshot.is_clean(),
        };

        Ok(QueryResult {
            data: matches,
            snapshot: snapshot.clone(),
            confidence: ConfidenceLevel::SyntacticCandidate,
            missing_reasons,
            coverage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::repo_intelligence::graph::{CodeNode, NodeKind, SymbolId};

    #[test]
    fn test_repo_coordinator_find_symbol_and_stale_detection() {
        let temp_dir =
            std::env::temp_dir().join(format!("custos_coord_{}", custos_domain::new_id("coord")));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("lib.rs");
        std::fs::write(&file_path, "pub struct TaskWorker;\n").unwrap();

        let mut coordinator = RepoCoordinator::new(temp_dir.clone());
        let sym_id = SymbolId::symbol_node("test", "TaskWorker");
        coordinator.graph_mut().add_node(CodeNode {
            id: sym_id,
            kind: NodeKind::Struct {
                name: "TaskWorker".into(),
                visibility: "pub".into(),
                doc_comment: None,
            },
            file_path: Some("lib.rs".into()),
            line_number: Some(1),
            importance_score: 1.0,
        });

        let snapshot = coordinator.current_snapshot("test_repo", "main");
        assert!(snapshot.dirty_hashes.contains_key("lib.rs"));

        // File is fresh with current snapshot
        assert!(!coordinator.is_stale(&snapshot, "lib.rs"));

        let res = coordinator.find_symbol(&snapshot, "TaskWorker").unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.confidence, ConfidenceLevel::SyntacticCandidate);
        assert!(res.missing_reasons.is_empty());

        // Modify file -> becomes stale
        std::fs::write(&file_path, "pub struct TaskWorkerModified;\n").unwrap();
        assert!(coordinator.is_stale(&snapshot, "lib.rs"));

        let res2 = coordinator.find_symbol(&snapshot, "TaskWorker").unwrap();
        assert_eq!(res2.missing_reasons.len(), 1);
        assert!(matches!(
            res2.missing_reasons[0],
            MissingReason::StaleSource { .. }
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_repo_coordinator_git_head_resolution() {
        let temp_dir =
            std::env::temp_dir().join(format!("custos_git_coord_{}", custos_domain::new_id("gitcoord")));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Initialize Git repo and make initial commit
        let _ = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&temp_dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.name", "TestUser"])
            .current_dir(&temp_dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&temp_dir)
            .output();

        let readme = temp_dir.join("README.md");
        std::fs::write(&readme, "# Test Repo\n").unwrap();
        let _ = std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&temp_dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&temp_dir)
            .output();

        let coordinator = RepoCoordinator::new(temp_dir.clone());
        let snapshot = coordinator.current_snapshot("test_git_repo", "main");

        // HEAD commit must be a valid 40-character hex commit SHA, not placeholder
        assert_ne!(snapshot.head_commit, "snapshot_head");
        assert_ne!(snapshot.head_commit, "uncommitted_workspace");
        assert_eq!(snapshot.head_commit.len(), 40);
        assert!(snapshot.head_commit.chars().all(|c| c.is_ascii_hexdigit()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
