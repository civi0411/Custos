//! Candidate comparison use case.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_candidates_compare(&self, req: ApiRequest) -> ApiResponse {
        let coordinator = match self.env.workspace.as_ref() {
            Some(c) => c,
            None => {
                return ApiResponse::error(
                    req.id,
                    "WorkspaceCoordinator not configured on daemon",
                )
            }
        };

        let target_workspaces: Vec<custos_domain::ExecutionWorkspace> = if let Some(
            ws_ids,
        ) =
            req.params.get("workspace_ids").and_then(|v| v.as_array())
        {
            let mut list = Vec::new();
            for val in ws_ids {
                if let Some(id_str) = val.as_str() {
                    match coordinator
                        .get_workspace(&custos_domain::WorkspaceId::new(id_str))
                        .await
                    {
                        Ok(Some(ws)) => list.push(ws),
                        Ok(None) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace '{id_str}' not found"),
                            )
                        }
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to retrieve workspace '{id_str}': {e}"),
                            )
                        }
                    }
                }
            }
            if list.len() != ws_ids.len() {
                return ApiResponse::error(req.id, "Invalid workspace ID in comparison");
            }
            list
        } else if let Some(task_id) = req.params.get("task_id").and_then(|v| v.as_str()) {
            let all = match coordinator.list_workspaces().await {
                Ok(all) => all,
                Err(error) => return ApiResponse::error(req.id, error.to_string()),
            };
            all.into_iter()
                .filter(|w| w.owner_task_id.as_deref() == Some(task_id))
                .collect()
        } else {
            return ApiResponse::error(
                req.id,
                "Either 'workspace_ids' or 'task_id' must be provided",
            );
        };

        if target_workspaces.is_empty() {
            return ApiResponse::error(
                req.id,
                "No candidate workspaces found for comparison",
            );
        }

        let baseline = &target_workspaces[0];
        let Some(owner_task) = baseline.owner_task_id.as_deref() else {
            return ApiResponse::error(
                req.id,
                "Comparison requires Task-owned candidate worktrees",
            );
        };
        let Some(base_commit) = baseline
            .base_commit_hash
            .as_deref()
            .filter(|hash| !hash.is_empty())
        else {
            return ApiResponse::error(
                req.id,
                "Comparison requires a recorded base commit",
            );
        };
        let requested_task = req.params.get("task_id").and_then(|value| value.as_str());
        let mut selected_ids = std::collections::HashSet::new();
        for candidate in &target_workspaces {
            if !selected_ids.insert(candidate.id.to_string())
                || !candidate.is_ready()
                || candidate.owner_task_id.as_deref() != Some(owner_task)
                || requested_task.is_some_and(|task| task != owner_task)
                || candidate.base_commit_hash.as_deref() != Some(base_commit)
            {
                return ApiResponse::error(req.id, "Candidates must be distinct ready worktrees of one Task and base commit");
            }
        }

        let first_repo = match &target_workspaces[0].kind {
            custos_domain::WorkspaceKind::Git { repo_path, .. } => repo_path.clone(),
            _ => return ApiResponse::error(req.id, format!("Workspace '{}' is not a Git workspace; candidates must be Git worktrees", target_workspaces[0].id)),
        };

        for ws in &target_workspaces {
            match &ws.kind {
                custos_domain::WorkspaceKind::Git { repo_path, .. } => {
                    if *repo_path != first_repo {
                        return ApiResponse::error(
                            req.id,
                            format!("Candidate workspace '{}' belongs to repository '{}', but baseline repository is '{}'", ws.id, repo_path, first_repo)
                        );
                    }
                }
                _ => {
                    return ApiResponse::error(
                        req.id,
                        format!("Workspace '{}' is not a Git workspace", ws.id),
                    );
                }
            }
        }

        let mut candidate_comparisons = Vec::new();
        for ws in target_workspaces {
            let dirty_res = coordinator.inspect_dirty(&ws.id).await;
            let diff_res = coordinator.diff(&ws.id).await;

            let (dirty, dirty_err) = match dirty_res {
                Ok(d) => (Some(d), None),
                Err(e) => (None, Some(e.to_string())),
            };
            let (diff_text, diff_err) = match diff_res {
                Ok(d) => (Some(d), None),
                Err(e) => (None, Some(e.to_string())),
            };

            let branch = match &ws.kind {
                custos_domain::WorkspaceKind::Git { branch, .. } => branch.clone(),
                _ => "main".to_string(),
            };

            let inspection_ok = dirty_err.is_none() && diff_err.is_none();
            let is_ready_for_integration = inspection_ok && ws.is_ready();

            candidate_comparisons.push(serde_json::json!({
                "workspace_id": ws.id.to_string(),
                "name": ws.name,
                "path": ws.path,
                "branch": branch,
                "base_commit_hash": ws.base_commit_hash,
                "owner_task_id": ws.owner_task_id,
                "is_dirty": dirty.as_ref().map(|d| d.is_dirty).unwrap_or(false),
                "modified_count": dirty.as_ref().map(|d| d.modified_files.len()).unwrap_or(0),
                "untracked_count": dirty.as_ref().map(|d| d.untracked_files.len()).unwrap_or(0),
                "deleted_count": dirty.as_ref().map(|d| d.deleted_files.len()).unwrap_or(0),
                "modified_files": dirty.as_ref().map(|d| d.modified_files.clone()).unwrap_or_default(),
                "untracked_files": dirty.as_ref().map(|d| d.untracked_files.clone()).unwrap_or_default(),
                "deleted_files": dirty.as_ref().map(|d| d.deleted_files.clone()).unwrap_or_default(),
                "diff": diff_text.unwrap_or_default(),
                "dirty_error": dirty_err,
                "diff_error": diff_err,
                "inspection_error": if !inspection_ok { Some("Failed to inspect candidate worktree status or diff") } else { None },
                "is_ready_for_integration": is_ready_for_integration,
                "status": ws.status.to_string(),
            }));
        }

        ApiResponse::success(
            req.id,
            serde_json::json!({
                "count": candidate_comparisons.len(),
                "candidates": candidate_comparisons,
            }),
        )
    }
}
