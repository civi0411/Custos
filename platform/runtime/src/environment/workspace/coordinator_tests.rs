use super::*;
use async_trait::async_trait;
use custos_persistence::SqliteTaskStore;
use std::sync::Mutex;

struct MockWorkspaceProvider {
    should_fail_provision: bool,
    should_fail_setup: bool,
    should_fail_teardown: bool,
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
            should_fail_teardown: false,
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
                vec!["platform/core/src/lib.rs".to_string()]
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
        if self.should_fail_teardown {
            return Err(DomainError::Conflict("Physical cleanup failed".into()));
        }
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

    let err = coordinator.archive_workspace(&ws.id, true).await;
    assert!(err.is_err());
    assert!(err
        .unwrap_err()
        .to_string()
        .contains("contains uncommitted changes"));

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

    let manifest = coordinator.inspect_dirty(&ws.id).await.unwrap();
    assert_eq!(manifest.head_commit.as_deref(), Some("commit_abc123"));

    let recovered = coordinator.recover_workspace(&ws.id).await.unwrap();
    assert_eq!(recovered.status, WorkspaceStatus::Ready);
    assert!(recovered.dirty_manifest.is_some());

    let all_recovered = coordinator.reconcile_all().await.unwrap();
    assert_eq!(all_recovered.len(), 1);
}

#[tokio::test]
async fn failed_physical_cleanup_does_not_archive_workspace() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let mut provider = MockWorkspaceProvider::new();
    provider.should_fail_teardown = true;
    let coordinator = WorkspaceCoordinator::new(store, Arc::new(provider));
    let workspace = coordinator
        .create_workspace(CreateWorkspaceRequest {
            name: "cleanup-retry".into(),
            kind: WorkspaceKind::Folder {
                path: "/tmp/cleanup-retry".into(),
            },
            path: "/tmp/cleanup-retry".into(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        })
        .await
        .unwrap();
    assert!(coordinator
        .archive_workspace(&workspace.id, true)
        .await
        .is_err());
    let retained = coordinator
        .get_workspace(&workspace.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained.status, WorkspaceStatus::Ready);
}
