//! Workspace lifecycle use cases: create, get, list, archive, recover, inspect dirty, pick folder, steer.

use crate::api::*;
use custos_domain::WorkspaceId;
use custos_runtime::workspace::CreateWorkspaceRequest;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces_lifecycle(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_CREATE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let owner_task_id = req
                    .params
                    .get("owner_task_id")
                    .or_else(|| req.params.get("task_id"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let params: CreateWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let create_req = CreateWorkspaceRequest {
                    name: params.name,
                    kind: params.kind,
                    path: params.path,
                    lineage: params.lineage,
                    owner_task_id,
                    metadata: params.metadata,
                    setup_script: params.setup_script,
                };
                match coordinator.create_workspace(create_req).await {
                    Ok(ws) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_GET => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) => WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id param"),
                };
                match coordinator.get_workspace(&ws_id).await {
                    Ok(Some(ws)) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Workspace {ws_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_LIST => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                match coordinator.list_workspaces().await {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_ARCHIVE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let params: ArchiveWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let ws_id = WorkspaceId::new(params.workspace_id);
                match coordinator
                    .archive_workspace_with_force(&ws_id, params.delete_physical, params.force)
                    .await
                {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "archived": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_INSPECT_DIRTY => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) => WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                match coordinator.inspect_dirty(&ws_id).await {
                    Ok(dirty) => match serde_json::to_value(&dirty) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_RECOVER => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let target_id = req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let ws_id = WorkspaceId::new(id);
                        match coordinator.recover_workspace(&ws_id).await {
                            Ok(ws) => match serde_json::to_value(&ws) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            },
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => match coordinator.reconcile_all().await {
                        Ok(recovered) => match serde_json::to_value(&recovered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        },
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                }
            }
            METHOD_WORKSPACES_PICK_FOLDER => {
                #[cfg(target_os = "macos")]
                {
                    let script = if req.params.get("purpose").and_then(|v| v.as_str())
                        == Some("model")
                    {
                        "POSIX path of (choose folder with prompt \"Select Folder Containing GGUF Models\")"
                    } else {
                        "POSIX path of (choose folder with prompt \"Select Project or Repository Folder\")"
                    };
                    let out = tokio::process::Command::new("osascript")
                        .arg("-e")
                        .arg(script)
                        .output()
                        .await;

                    match out {
                        Ok(o) if o.status.success() => {
                            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                            ApiResponse::success(req.id, serde_json::json!({ "path": p }))
                        }
                        _ => ApiResponse::success(req.id, serde_json::json!({ "path": null })),
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    ApiResponse::success(req.id, serde_json::json!({ "path": null }))
                }
            }
            METHOD_WORKSPACES_AGENT_STEER => {
                let guidance = match req.params.get("guidance").and_then(|v| v.as_str()) {
                    Some(g) if !g.trim().is_empty() => g,
                    _ => return ApiResponse::error(req.id, "Missing guidance parameter"),
                };
                let harness_id = req.params.get("harness_id").and_then(|v| v.as_str());
                let run_id = req.params.get("run_id").and_then(|v| v.as_str());

                if let (Some(h_id), Some(r_id)) = (harness_id, run_id) {
                    match self.harnesses.steer(h_id, r_id, guidance).await {
                        Ok(()) => {
                            ApiResponse::success(req.id, serde_json::json!({"steered": true}))
                        }
                        Err(err) => {
                            ApiResponse::error(req.id, format!("Failed to steer harness: {err}"))
                        }
                    }
                } else if let Some(r_id) = run_id {
                    let mut steered = false;
                    for desc in self.harnesses.list() {
                        if self.harnesses.steer(&desc.id, r_id, guidance).await.is_ok() {
                            steered = true;
                            break;
                        }
                    }
                    if steered {
                        ApiResponse::success(req.id, serde_json::json!({"steered": true}))
                    } else {
                        ApiResponse::error(
                            req.id,
                            "Workspace steering is unavailable: this workspace is not bound to an active harness run",
                        )
                    }
                } else {
                    ApiResponse::error(
                        req.id,
                        "Workspace steering is unavailable: missing run_id or harness_id parameter",
                    )
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown lifecycle method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_adapters::workspace::LocalWorkspaceProvider;
    use custos_persistence::SqliteTaskStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn lifecycle_group_test_without_dispatcher() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let provider = Arc::new(LocalWorkspaceProvider::new());
        let coordinator = WorkspaceCoordinator::new(store, provider);

        let list = coordinator.list_workspaces().await.unwrap();
        assert_eq!(list.len(), 0);

        let tmp = tempfile::tempdir().unwrap();
        let ws_path = tmp.path().join("lifecycle_test_ws");
        let create_req = CreateWorkspaceRequest {
            name: "lifecycle-test".to_string(),
            kind: custos_domain::WorkspaceKind::Folder {
                path: ws_path.to_string_lossy().to_string(),
            },
            path: ws_path.to_string_lossy().to_string(),
            lineage: None,
            owner_task_id: None,
            metadata: None,
            setup_script: None,
        };
        let created = coordinator.create_workspace(create_req).await.unwrap();
        assert_eq!(created.name, "lifecycle-test");

        let fetched = coordinator.get_workspace(&created.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);

        let updated_list = coordinator.list_workspaces().await.unwrap();
        assert_eq!(updated_list.len(), 1);

        coordinator
            .archive_workspace_with_force(&created.id, true, true)
            .await
            .unwrap();
        let after_archive = coordinator.get_workspace(&created.id).await.unwrap();
        assert!(after_archive.is_some());
        assert_eq!(
            after_archive.unwrap().status,
            custos_domain::WorkspaceStatus::Archived
        );
    }
}
