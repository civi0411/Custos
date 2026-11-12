use std::sync::Arc;

use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::WorkspaceCoordinator;

use crate::api::*;

#[tokio::test]
async fn test_workspace_api_dispatch_lifecycle() {
    use custos_adapters::workspace::LocalWorkspaceProvider;
    use custos_domain::{ExecutionWorkspace, WorkspaceStatus};
    use custos_persistence::SqliteTaskStore;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let workspace_provider = Arc::new(LocalWorkspaceProvider::new());
    let coordinator = Arc::new(WorkspaceCoordinator::new(store.clone(), workspace_provider));

    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_workspace(coordinator);

    let tmp = tempfile::tempdir().unwrap();
    let ws_path = tmp.path().join("api_test_ws");
    let path_str = ws_path.to_str().unwrap().to_string();

    // 1. Create Workspace
    let create_req = ApiRequest {
        id: "req_ws_1".into(),
        method: METHOD_WORKSPACES_CREATE.into(),
        params: serde_json::json!({
            "name": "api-workspace",
            "kind": {
                "type": "folder",
                "path": path_str
            },
            "path": path_str,
            "metadata": {
                "domain": "assistant"
            }
        }),
    };
    let create_resp = dispatcher.handle_request(create_req).await;
    assert!(
        create_resp.error.is_none(),
        "Error: {:?}",
        create_resp.error
    );
    let created_ws: ExecutionWorkspace =
        serde_json::from_value(create_resp.result.unwrap()).unwrap();
    assert_eq!(created_ws.name, "api-workspace");
    assert_eq!(created_ws.status, WorkspaceStatus::Ready);
    assert!(ws_path.exists());

    // 2. Get Workspace
    let get_req = ApiRequest {
        id: "req_ws_2".into(),
        method: METHOD_WORKSPACES_GET.into(),
        params: serde_json::json!({
            "workspace_id": created_ws.id.as_str()
        }),
    };
    let get_resp = dispatcher.handle_request(get_req).await;
    assert!(get_resp.error.is_none());
    let loaded_ws: ExecutionWorkspace = serde_json::from_value(get_resp.result.unwrap()).unwrap();
    assert_eq!(loaded_ws.id, created_ws.id);

    // 3. List Workspaces
    let list_req = ApiRequest {
        id: "req_ws_3".into(),
        method: METHOD_WORKSPACES_LIST.into(),
        params: serde_json::json!({}),
    };
    let list_resp = dispatcher.handle_request(list_req).await;
    assert!(list_resp.error.is_none());
    let workspaces: Vec<ExecutionWorkspace> =
        serde_json::from_value(list_resp.result.unwrap()).unwrap();
    assert_eq!(workspaces.len(), 1);

    // 3b. Inspect dirty manifest
    let inspect_req = ApiRequest {
        id: "req_ws_inspect".into(),
        method: METHOD_WORKSPACES_INSPECT_DIRTY.into(),
        params: serde_json::json!({
            "workspace_id": created_ws.id.as_str()
        }),
    };
    let inspect_resp = dispatcher.handle_request(inspect_req).await;
    assert!(inspect_resp.is_success());
    let dirty: custos_domain::DirtyManifest =
        serde_json::from_value(inspect_resp.result.unwrap()).unwrap();
    assert!(!dirty.is_dirty);

    // 3c. Recover workspace
    let recover_req = ApiRequest {
        id: "req_ws_recover".into(),
        method: METHOD_WORKSPACES_RECOVER.into(),
        params: serde_json::json!({
            "workspace_id": created_ws.id.as_str()
        }),
    };
    let recover_resp = dispatcher.handle_request(recover_req).await;
    assert!(recover_resp.is_success());

    // 4. Archive Workspace (with delete_physical: true)
    let archive_req = ApiRequest {
        id: "req_ws_4".into(),
        method: METHOD_WORKSPACES_ARCHIVE.into(),
        params: serde_json::json!({
            "workspace_id": created_ws.id.as_str(),
            "delete_physical": true,
            "force": false
        }),
    };
    let archive_resp = dispatcher.handle_request(archive_req).await;
    assert!(archive_resp.error.is_none());
    assert!(!ws_path.exists());
}

#[tokio::test]
async fn test_workspace_files_api_dispatch() {
    use custos_persistence::SqliteTaskStore;

    let temp_dir = std::env::temp_dir().join(format!("custos_ws_api_files_{}", std::process::id()));
    let _ = tokio::fs::create_dir_all(&temp_dir).await;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));

    let ws_provider = Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(store, ws_provider));
    let ws = ws_coord
        .create_workspace(CreateWorkspaceRequest {
            name: "files_test_ws".into(),
            kind: custos_domain::WorkspaceKind::Folder {
                path: temp_dir.to_string_lossy().to_string(),
            },
            path: temp_dir.to_string_lossy().to_string(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        })
        .await
        .expect("create workspace");

    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_workspace(ws_coord);

    // 1. Write file
    let write_resp = dispatcher
        .handle_request(ApiRequest {
            id: "wf_1".into(),
            method: METHOD_WORKSPACE_FILES_WRITE.into(),
            params: serde_json::json!({
                "workspace_id": ws.id.to_string(),
                "path": "src/hello.rs",
                "content": "pub fn hello() -> &'static str { \"world\" }\n",
            }),
        })
        .await;
    assert!(write_resp.is_success());
    let written: custos_domain::WorkspaceFileContent =
        serde_json::from_value(write_resp.result.unwrap()).unwrap();
    assert_eq!(written.path, "src/hello.rs");
    assert_eq!(written.line_count, 1);

    // 2. Read file
    let read_resp = dispatcher
        .handle_request(ApiRequest {
            id: "wf_2".into(),
            method: METHOD_WORKSPACE_FILES_READ.into(),
            params: serde_json::json!({
                "workspace_id": ws.id.to_string(),
                "path": "src/hello.rs",
            }),
        })
        .await;
    assert!(read_resp.is_success());
    let read: custos_domain::WorkspaceFileContent =
        serde_json::from_value(read_resp.result.unwrap()).unwrap();
    assert!(read.content.contains("pub fn hello"));

    // 3. Tree
    let tree_resp = dispatcher
        .handle_request(ApiRequest {
            id: "wf_3".into(),
            method: METHOD_WORKSPACE_FILES_TREE.into(),
            params: serde_json::json!({
                "workspace_id": ws.id.to_string(),
            }),
        })
        .await;
    assert!(tree_resp.is_success());
    let tree: custos_domain::WorkspaceFileTree =
        serde_json::from_value(tree_resp.result.unwrap()).unwrap();
    assert!(tree.entries.iter().any(|e| e.path == "src/hello.rs"));

    // 4. Diff (folder ws is not git repo, returns clean summary)
    let diff_resp = dispatcher
        .handle_request(ApiRequest {
            id: "wf_4".into(),
            method: METHOD_WORKSPACE_DIFF.into(),
            params: serde_json::json!({
                "workspace_id": ws.id.to_string(),
            }),
        })
        .await;
    assert!(diff_resp.is_success());
    let diff: custos_domain::WorkspaceDiffSummary =
        serde_json::from_value(diff_resp.result.unwrap()).unwrap();
    assert!(diff.is_clean);

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}
