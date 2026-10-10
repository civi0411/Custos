//! ExecutionWorkspace Domain Model
//!
//! Represents an isolated workspace lifecycle (Worktree, Folder, or Remote SSH)
//! managed across Custos workbenches (Engineering, Research, Assistant).
//! Pure, deterministic domain models with ZERO I/O side effects.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

/// Strongly-typed Workspace Identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(new_id("ws"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for WorkspaceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Physical nature of the workspace environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkspaceKind {
    /// Git worktree isolated from a parent repository
    Git {
        repo_path: String,
        branch: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_commit: Option<String>,
    },
    /// Simple filesystem folder (Research datasets, notes, Assistant connectors)
    Folder {
        path: String,
    },
    /// Remote SSH workspace environment
    RemoteSsh {
        host: String,
        remote_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<u16>,
    },
}

/// Lifecycle status of an ExecutionWorkspace
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    /// Workspace creation requested; physical directory or worktree is being provisioned
    Initializing,
    /// Workspace successfully provisioned and ready for agent runs or tool execution
    Ready,
    /// Physical provisioning or post-create setup hook failed
    SetupFailed { reason: String },
    /// Workspace execution concluded or archived
    Archived,
}

impl WorkspaceStatus {
    pub fn can_transition_to(&self, next: &WorkspaceStatus) -> bool {
        match (self, next) {
            // From Initializing can become Ready or SetupFailed
            (WorkspaceStatus::Initializing, WorkspaceStatus::Ready) => true,
            (WorkspaceStatus::Initializing, WorkspaceStatus::SetupFailed { .. }) => true,

            // From Ready can become Archived
            (WorkspaceStatus::Ready, WorkspaceStatus::Archived) => true,

            // From SetupFailed can retry (Initializing) or be Archived
            (WorkspaceStatus::SetupFailed { .. }, WorkspaceStatus::Initializing) => true,
            (WorkspaceStatus::SetupFailed { .. }, WorkspaceStatus::Archived) => true,

            // Archived is terminal
            (WorkspaceStatus::Archived, _) => false,

            // All other transitions are invalid
            _ => false,
        }
    }
}

/// Lineage tracking parent relationships, base commits, and branch targeting
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorkspaceLineage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_workspace_id: Option<WorkspaceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_commit: Option<String>,
}

/// Detailed manifest of uncommitted changes and HEAD status in a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DirtyManifest {
    pub is_dirty: bool,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub deleted_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_commit: Option<String>,
    pub checked_at: i64,
}

/// Sovereign ExecutionWorkspace entity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWorkspace {
    pub id: WorkspaceId,
    pub name: String,
    pub kind: WorkspaceKind,
    /// Resolved target path on host filesystem or remote environment
    pub path: String,
    pub status: WorkspaceStatus,
    pub lineage: WorkspaceLineage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_commit_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dirty_manifest: Option<DirtyManifest>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

impl ExecutionWorkspace {
    pub fn new(
        id: WorkspaceId,
        name: impl Into<String>,
        kind: WorkspaceKind,
        path: impl Into<String>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id,
            name: name.into(),
            kind,
            path: path.into(),
            status: WorkspaceStatus::Initializing,
            lineage: WorkspaceLineage::default(),
            owner_task_id: None,
            base_commit_hash: None,
            dirty_manifest: None,
            metadata: serde_json::json!({}),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn with_lineage(mut self, lineage: WorkspaceLineage) -> Self {
        self.lineage = lineage;
        self
    }

    pub fn with_owner_task(mut self, task_id: impl Into<String>) -> Self {
        self.owner_task_id = Some(task_id.into());
        self
    }

    pub fn with_base_commit_hash(mut self, hash: impl Into<String>) -> Self {
        self.base_commit_hash = Some(hash.into());
        self
    }

    pub fn with_dirty_manifest(mut self, manifest: DirtyManifest) -> Self {
        self.dirty_manifest = Some(manifest);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status, WorkspaceStatus::Ready)
    }

    pub fn is_archived(&self) -> bool {
        matches!(self.status, WorkspaceStatus::Archived)
    }

    pub fn transition_to(&mut self, next: WorkspaceStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(&next) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", next),
            });
        }
        self.status = next;
        self.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn update_head_commit(&mut self, commit: impl Into<String>) {
        self.lineage.head_commit = Some(commit.into());
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// Validates path safety, prohibiting directory traversal ('..'), system root targeting,
    /// and repo-worktree path collision.
    pub fn validate_path_safety(&self) -> Result<(), DomainError> {
        let trimmed_path = self.path.trim();
        if trimmed_path.is_empty() {
            return Err(DomainError::Validation("Workspace path cannot be empty".into()));
        }

        // Check for path traversal components in workspace path
        for comp in std::path::Path::new(trimmed_path).components() {
            if matches!(comp, std::path::Component::ParentDir) {
                return Err(DomainError::Validation(format!(
                    "Path traversal ('..') is prohibited in workspace path: {}",
                    self.path
                )));
            }
        }

        // Prohibit targeting root or critical system directories
        let prohibited = ["/", "/bin", "/sbin", "/usr", "/etc", "/System", "/dev", "/proc", "/sys"];
        for forbidden in &prohibited {
            if trimmed_path == *forbidden {
                return Err(DomainError::Validation(format!(
                    "Workspace path cannot target system root or critical directory: {}",
                    self.path
                )));
            }
        }

        match &self.kind {
            WorkspaceKind::Git {
                repo_path,
                branch,
                ..
            } => {
                let trimmed_repo = repo_path.trim();
                if trimmed_repo.is_empty() {
                    return Err(DomainError::Validation("Git repo path cannot be empty".into()));
                }
                for comp in std::path::Path::new(trimmed_repo).components() {
                    if matches!(comp, std::path::Component::ParentDir) {
                        return Err(DomainError::Validation(format!(
                            "Path traversal ('..') is prohibited in git repo path: {}",
                            repo_path
                        )));
                    }
                }
                let trimmed_branch = branch.trim();
                if trimmed_branch.is_empty() {
                    return Err(DomainError::Validation("Git branch name cannot be empty".into()));
                }
                if trimmed_branch.contains("..") || trimmed_branch.starts_with('/') {
                    return Err(DomainError::Validation(format!(
                        "Invalid git branch name: {}",
                        branch
                    )));
                }
                if trimmed_path == trimmed_repo {
                    return Err(DomainError::Validation(
                        "Target worktree path cannot be identical to repository root".into(),
                    ));
                }
            }
            WorkspaceKind::Folder { path } => {
                let trimmed_folder = path.trim();
                if trimmed_folder.is_empty() {
                    return Err(DomainError::Validation("Folder path cannot be empty".into()));
                }
                for comp in std::path::Path::new(trimmed_folder).components() {
                    if matches!(comp, std::path::Component::ParentDir) {
                        return Err(DomainError::Validation(format!(
                            "Path traversal ('..') is prohibited in folder path: {}",
                            path
                        )));
                    }
                }
            }
            WorkspaceKind::RemoteSsh {
                host,
                remote_path,
                ..
            } => {
                if host.trim().is_empty() {
                    return Err(DomainError::Validation("Remote SSH host cannot be empty".into()));
                }
                if remote_path.trim().is_empty() {
                    return Err(DomainError::Validation("Remote SSH path cannot be empty".into()));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_id_generation() {
        let id = WorkspaceId::generate();
        assert!(id.as_str().starts_with("ws_"));
        assert_eq!(id.to_string(), id.as_str());
    }

    #[test]
    fn test_workspace_lifecycle_transitions() {
        let mut ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "feature-worktree",
            WorkspaceKind::Git {
                repo_path: "/repos/custos".into(),
                branch: "feat/worktree".into(),
                base_commit: Some("abcdef123".into()),
            },
            "/tmp/worktrees/feat-worktree",
        );

        assert_eq!(ws.status, WorkspaceStatus::Initializing);
        assert!(!ws.is_ready());

        // Valid transition: Initializing -> Ready
        assert!(ws.transition_to(WorkspaceStatus::Ready).is_ok());
        assert!(ws.is_ready());

        // Invalid transition: Ready -> Initializing
        assert!(ws.transition_to(WorkspaceStatus::Initializing).is_err());

        // Valid transition: Ready -> Archived
        assert!(ws.transition_to(WorkspaceStatus::Archived).is_ok());
        assert!(ws.is_archived());

        // Terminal state: Archived cannot transition anywhere
        assert!(ws.transition_to(WorkspaceStatus::Ready).is_err());
    }

    #[test]
    fn test_workspace_setup_failed_and_retry() {
        let mut ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "research-dataset",
            WorkspaceKind::Folder {
                path: "/data/experiments".into(),
            },
            "/data/experiments",
        );

        // Fail setup
        assert!(ws
            .transition_to(WorkspaceStatus::SetupFailed {
                reason: "Disk full".into()
            })
            .is_ok());

        // Retry: SetupFailed -> Initializing
        assert!(ws.transition_to(WorkspaceStatus::Initializing).is_ok());

        // Success: Initializing -> Ready
        assert!(ws.transition_to(WorkspaceStatus::Ready).is_ok());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let ws = ExecutionWorkspace::new(
            WorkspaceId::new("ws_test123"),
            "test-workspace",
            WorkspaceKind::Git {
                repo_path: "/custos".into(),
                branch: "main".into(),
                base_commit: None,
            },
            "/worktrees/test123",
        )
        .with_lineage(WorkspaceLineage {
            parent_workspace_id: Some(WorkspaceId::new("ws_parent")),
            base_commit: Some("1234567".into()),
            target_branch: Some("main".into()),
            head_commit: None,
        })
        .with_metadata(serde_json::json!({
            "domain": "engineering",
            "owner": "user_1"
        }));

        let json = serde_json::to_string(&ws).expect("Serialize");
        let deserialized: ExecutionWorkspace = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(ws, deserialized);
        assert_eq!(deserialized.metadata["domain"], "engineering");
    }

    #[test]
    fn test_validate_path_safety_guards() {
        // Traversal in path
        let ws_traversal = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "bad-ws",
            WorkspaceKind::Folder {
                path: "/safe/path".into(),
            },
            "/safe/path/../../etc/passwd",
        );
        assert!(ws_traversal.validate_path_safety().is_err());

        // Root path targeting
        let ws_root = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "root-ws",
            WorkspaceKind::Folder {
                path: "/".into(),
            },
            "/",
        );
        assert!(ws_root.validate_path_safety().is_err());

        // Git repo identical to worktree path
        let ws_collision = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "collision-ws",
            WorkspaceKind::Git {
                repo_path: "/repo/custos".into(),
                branch: "feat".into(),
                base_commit: None,
            },
            "/repo/custos",
        );
        assert!(ws_collision.validate_path_safety().is_err());

        // Valid workspace
        let ws_valid = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "valid-ws",
            WorkspaceKind::Git {
                repo_path: "/repo/custos".into(),
                branch: "feat-1".into(),
                base_commit: None,
            },
            "/repo/custos/.custos/worktrees/feat-1",
        );
        assert!(ws_valid.validate_path_safety().is_ok());
    }
}

