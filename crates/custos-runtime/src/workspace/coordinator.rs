//! ExecutionWorkspace Coordinator
//!
//! Orchestrates the end-to-end lifecycle of ExecutionWorkspaces across
//! the 3 Custos workbenches (Engineering/Git, Research/Folders, Assistant/Connectors).
//!
//! Implements the lessons learned from OrCa:
//! - Pre-mutation state persistence (record 'Initializing' before disk mutations)
//! - Explicit failure tracking ('SetupFailed') rather than blind retries
//! - Clean separation of host provisioning (WorkspaceProvider) and storage (WorkspaceRepository).

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use custos_core::contracts::workspace::{WorkspaceProvider, WorkspaceRepository};
use custos_domain::{
    DirtyManifest, DomainError, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage,
    WorkspaceStatus,
};

/// Request parameters for creating an ExecutionWorkspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub kind: WorkspaceKind,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<WorkspaceLineage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_script: Option<String>,
}

/// Core runtime coordinator for execution workspaces
#[derive(Clone)]
pub struct WorkspaceCoordinator {
    repository: Arc<dyn WorkspaceRepository>,
    provider: Arc<dyn WorkspaceProvider>,
}

impl WorkspaceCoordinator {
    pub fn new(
        repository: Arc<dyn WorkspaceRepository>,
        provider: Arc<dyn WorkspaceProvider>,
    ) -> Self {
        Self {
            repository,
            provider,
        }
    }

    /// Creates and provisions a new ExecutionWorkspace.
    ///
    /// Lifecycle guarantee:
    /// 1. Validate environment & paths.
    /// 2. Persist workspace record with state `Initializing`.
    /// 3. Call WorkspaceProvider to physically create directory / worktree.
    /// 4. Execute optional setup hook inside provisioned workspace.
    /// 5. On success, transition to `Ready` and persist.
    /// 6. On failure, transition to `SetupFailed` with error reason and persist.
    pub async fn create_workspace(
        &self,
        req: CreateWorkspaceRequest,
    ) -> Result<ExecutionWorkspace, DomainError> {
        info!(
            workspace_name = %req.name,
            workspace_path = %req.path,
            "Initiating ExecutionWorkspace creation"
        );

        // Step 1: Pre-validation
        self.provider.validate(&req.kind, &req.path).await?;

        let ws_id = WorkspaceId::generate();
        let mut ws = ExecutionWorkspace::new(ws_id.clone(), req.name, req.kind, req.path);

        if let Some(lineage) = req.lineage {
            ws = ws.with_lineage(lineage);
        }
        if let Some(owner) = req.owner_task_id {
            ws = ws.with_owner_task(owner);
        }
        if let Some(metadata) = req.metadata {
            ws = ws.with_metadata(metadata);
        }

        // Step 2: Persist 'Initializing' state before external mutations (OrCa lesson)
        self.repository.save_workspace(&ws).await?;

        // Step 3: Physically provision the workspace environment
        if let Err(err) = self.provider.provision(&ws).await {
            error!(
                workspace_id = %ws_id,
                error = %err,
                "Workspace physical provisioning failed"
            );
            let reason = err.to_string();
            let _ = self
                .repository
                .update_workspace_status(
                    &ws_id,
                    WorkspaceStatus::SetupFailed {
                        reason: reason.clone(),
                    },
                )
                .await;
            return Err(DomainError::Validation(format!(
                "Workspace provisioning failed: {}",
                reason
            )));
        }

        // Step 4: Run post-creation setup hooks if specified
        if let Some(script) = &req.setup_script {
            info!(workspace_id = %ws_id, "Executing workspace setup hook");
            if let Err(err) = self.provider.setup(&ws, Some(script)).await {
                warn!(
                    workspace_id = %ws_id,
                    error = %err,
                    "Workspace setup script execution failed"
                );
                let reason = err.to_string();
                let _ = self
                    .repository
                    .update_workspace_status(
                        &ws_id,
                        WorkspaceStatus::SetupFailed {
                            reason: reason.clone(),
                        },
                    )
                    .await;
                return Err(DomainError::Validation(format!(
                    "Workspace setup hook failed: {}",
                    reason
                )));
            }
        }

        // Step 5: Capture initial Git / host baseline and mark Ready
        let dirty = self.provider.inspect_dirty(&ws).await.unwrap_or_default();
        if let Some(head) = dirty.head_commit.as_ref() {
            ws = ws.with_base_commit_hash(head);
        }
        ws = ws.with_dirty_manifest(dirty);

        ws.transition_to(WorkspaceStatus::Ready)?;
        self.repository.save_workspace(&ws).await?;

        info!(workspace_id = %ws_id, "ExecutionWorkspace successfully provisioned and Ready");
        Ok(ws)
    }

    /// Fetches an existing workspace by its ID.
    pub async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        self.repository.get_workspace(id).await
    }

    /// Lists all workspaces across domains.
    pub async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        self.repository.list_workspaces().await
    }

    /// Inspects the dirty status and uncommitted changes of a workspace.
    pub async fn inspect_dirty(
        &self,
        id: &WorkspaceId,
    ) -> Result<DirtyManifest, DomainError> {
        let ws = self.repository.get_workspace(id).await?.ok_or_else(|| {
            DomainError::NotFound {
                kind: "ExecutionWorkspace".into(),
                id: id.to_string(),
            }
        })?;
        self.provider.inspect_dirty(&ws).await
    }

    /// Recovers a workspace by verifying its host environment and reconciling status.
    pub async fn recover_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<ExecutionWorkspace, DomainError> {
        let mut ws = self.repository.get_workspace(id).await?.ok_or_else(|| {
            DomainError::NotFound {
                kind: "ExecutionWorkspace".into(),
                id: id.to_string(),
            }
        })?;

        let is_valid = self.provider.recover(&ws).await.unwrap_or(false);
        if is_valid {
            let dirty = self.provider.inspect_dirty(&ws).await.unwrap_or_default();
            ws = ws.with_dirty_manifest(dirty);
            if matches!(
                ws.status,
                WorkspaceStatus::Initializing | WorkspaceStatus::SetupFailed { .. }
            ) {
                ws.transition_to(WorkspaceStatus::Ready)?;
                self.repository
                    .update_workspace_status(id, WorkspaceStatus::Ready)
                    .await?;
            }
            self.repository.save_workspace(&ws).await?;
            info!(workspace_id = %id, "ExecutionWorkspace successfully recovered to Ready");
        } else {
            let failed_status = WorkspaceStatus::SetupFailed {
                reason: "Physical directory or Git worktree is missing or corrupt".to_string(),
            };
            ws.status = failed_status.clone();
            self.repository
                .update_workspace_status(id, failed_status)
                .await?;
            warn!(workspace_id = %id, "ExecutionWorkspace recovery failed: host path invalid");
        }
        Ok(ws)
    }

    /// Reconciles all active workspaces against their physical host state.
    pub async fn reconcile_all(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        let all = self.repository.list_workspaces().await?;
        let mut recovered = Vec::new();
        for ws in all {
            if !ws.is_archived() {
                if let Ok(rec) = self.recover_workspace(&ws.id).await {
                    recovered.push(rec);
                }
            }
        }
        Ok(recovered)
    }

    /// Archives a workspace, optionally tearing down physical directories / worktrees.
    pub async fn archive_workspace(
        &self,
        id: &WorkspaceId,
        delete_physical: bool,
    ) -> Result<(), DomainError> {
        self.archive_workspace_with_force(id, delete_physical, false).await
    }

    /// Archives a workspace with dirty protection: if delete_physical is true and workspace is dirty,
    /// returns an error unless force is explicitly true.
    pub async fn archive_workspace_with_force(
        &self,
        id: &WorkspaceId,
        delete_physical: bool,
        force: bool,
    ) -> Result<(), DomainError> {
        info!(workspace_id = %id, delete_physical, force, "Archiving ExecutionWorkspace");
        let ws = self.repository.get_workspace(id).await?.ok_or_else(|| {
            DomainError::NotFound {
                kind: "ExecutionWorkspace".into(),
                id: id.to_string(),
            }
        })?;

        if delete_physical {
            if !force {
                let dirty = self.provider.inspect_dirty(&ws).await.unwrap_or_default();
                if dirty.is_dirty {
                    return Err(DomainError::Conflict(format!(
                        "Workspace '{}' contains uncommitted changes ({} modified, {} untracked, {} deleted). Set force=true to discard.",
                        ws.name,
                        dirty.modified_files.len(),
                        dirty.untracked_files.len(),
                        dirty.deleted_files.len()
                    )));
                }
            }

            if let Err(e) = self.provider.teardown(&ws).await {
                warn!(
                    workspace_id = %id,
                    error = %e,
                    "Failed to teardown physical workspace during archive"
                );
            }
        }

        self.repository
            .update_workspace_status(id, WorkspaceStatus::Archived)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_persistence::SqliteTaskStore;
    use std::sync::Mutex;

    struct MockWorkspaceProvider {
        should_fail_provision: bool,
        should_fail_setup: bool,
        is_dirty: bool,
        recover_result: bool,
        provision_calls: Mutex<usize>,
        teardown_calls: Mutex<usize>,
    }

    impl MockWorkspaceProvider {
        fn new() -> Self {
            Self {
                should_fail_provision: false,
                should_fail_setup: false,
                is_dirty: false,
                recover_result: true,
                provision_calls: Mutex::new(0),
                teardown_calls: Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl WorkspaceProvider for MockWorkspaceProvider {
        async fn validate(&self, _kind: &WorkspaceKind, path: &str) -> Result<(), DomainError> {
            if path.trim().is_empty() {
                return Err(DomainError::Validation("Empty path".into()));
            }
            Ok(())
        }

        async fn provision(&self, _workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
            *self.provision_calls.lock().unwrap() += 1;
            if self.should_fail_provision {
                return Err(DomainError::Validation("Disk write error".into()));
            }
            Ok(())
        }

        async fn setup(
            &self,
            _workspace: &ExecutionWorkspace,
            _script: Option<&str>,
        ) -> Result<(), DomainError> {
            if self.should_fail_setup {
                return Err(DomainError::Validation("Script syntax error".into()));
            }
            Ok(())
        }

        async fn inspect_dirty(
            &self,
            _workspace: &ExecutionWorkspace,
        ) -> Result<DirtyManifest, DomainError> {
            Ok(DirtyManifest {
                is_dirty: self.is_dirty,
                modified_files: if self.is_dirty {
                    vec!["crates/lib.rs".to_string()]
                } else {
                    vec![]
                },
                untracked_files: vec![],
                deleted_files: vec![],
                head_commit: Some("commit_abc123".to_string()),
                checked_at: 1000,
            })
        }

        async fn recover(&self, _workspace: &ExecutionWorkspace) -> Result<bool, DomainError> {
            Ok(self.recover_result)
        }

        async fn teardown(&self, _workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
            *self.teardown_calls.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_coordinator_create_workspace_success() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let provider = Arc::new(MockWorkspaceProvider::new());
        let coordinator = WorkspaceCoordinator::new(store.clone(), provider.clone());

        let req = CreateWorkspaceRequest {
            name: "experiment-alpha".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/exp-alpha".into(),
            },
            path: "/tmp/exp-alpha".into(),
            lineage: None,
            owner_task_id: None,
            metadata: Some(serde_json::json!({ "domain": "research" })),
            setup_script: Some("echo hello".into()),
        };

        let ws = coordinator.create_workspace(req).await.unwrap();
        assert_eq!(ws.name, "experiment-alpha");
        assert_eq!(ws.status, WorkspaceStatus::Ready);
        assert_eq!(ws.metadata["domain"], "research");

        // Verify state was persisted in DB
        let loaded = coordinator.get_workspace(&ws.id).await.unwrap().unwrap();
        assert_eq!(loaded.status, WorkspaceStatus::Ready);
        assert_eq!(*provider.provision_calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_provision_failure_records_setup_failed() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let mut mock = MockWorkspaceProvider::new();
        mock.should_fail_provision = true;
        let provider = Arc::new(mock);
        let coordinator = WorkspaceCoordinator::new(store.clone(), provider.clone());

        let req = CreateWorkspaceRequest {
            name: "failing-ws".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/fail".into(),
            },
            path: "/tmp/fail".into(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        };

        let result = coordinator.create_workspace(req).await;
        assert!(result.is_err());

        // Verify DB recorded failure rather than disappearing
        let list = coordinator.list_workspaces().await.unwrap();
        assert_eq!(list.len(), 1);
        match &list[0].status {
            WorkspaceStatus::SetupFailed { reason } => {
                assert!(reason.contains("Disk write error"));
            }
            other => panic!("Expected SetupFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_coordinator_archive_workspace() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let provider = Arc::new(MockWorkspaceProvider::new());
        let coordinator = WorkspaceCoordinator::new(store.clone(), provider.clone());

        let req = CreateWorkspaceRequest {
            name: "to-archive".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/archive-me".into(),
            },
            path: "/tmp/archive-me".into(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        };

        let ws = coordinator.create_workspace(req).await.unwrap();
        coordinator.archive_workspace(&ws.id, true).await.unwrap();

        let updated = coordinator.get_workspace(&ws.id).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkspaceStatus::Archived);
        assert_eq!(*provider.teardown_calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_dirty_protection_on_archive() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let mut mock = MockWorkspaceProvider::new();
        mock.is_dirty = true;
        let provider = Arc::new(mock);
        let coordinator = WorkspaceCoordinator::new(store.clone(), provider.clone());

        let req = CreateWorkspaceRequest {
            name: "dirty-worktree".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/dirty-ws".into(),
            },
            path: "/tmp/dirty-ws".into(),
            lineage: None,
            owner_task_id: Some("task_123".into()),
            metadata: None,
            setup_script: None,
        };

        let ws = coordinator.create_workspace(req).await.unwrap();
        assert_eq!(ws.owner_task_id.as_deref(), Some("task_123"));

        // Archiving with delete_physical = true must be rejected when workspace has uncommitted changes
        let err = coordinator.archive_workspace(&ws.id, true).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("contains uncommitted changes"));

        // Passing force = true explicitly bypasses dirty guard and archives
        let res = coordinator
            .archive_workspace_with_force(&ws.id, true, true)
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_recovery_and_reconciliation() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let provider = Arc::new(MockWorkspaceProvider::new());
        let coordinator = WorkspaceCoordinator::new(store.clone(), provider.clone());

        let req = CreateWorkspaceRequest {
            name: "recovering-ws".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/rec-ws".into(),
            },
            path: "/tmp/rec-ws".into(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        };

        let ws = coordinator.create_workspace(req).await.unwrap();

        // 1. Inspect dirty manifest
        let manifest = coordinator.inspect_dirty(&ws.id).await.unwrap();
        assert_eq!(manifest.head_commit.as_deref(), Some("commit_abc123"));

        // 2. Recover workspace
        let recovered = coordinator.recover_workspace(&ws.id).await.unwrap();
        assert_eq!(recovered.status, WorkspaceStatus::Ready);
        assert!(recovered.dirty_manifest.is_some());

        // 3. Reconcile all
        let all_recovered = coordinator.reconcile_all().await.unwrap();
        assert_eq!(all_recovered.len(), 1);
    }
}
