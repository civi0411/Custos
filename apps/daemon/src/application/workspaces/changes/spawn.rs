//! Candidate worktree spawning use case.

use crate::api::*;
use custos_runtime::workspace::CreateWorkspaceRequest;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_candidates_spawn(&self, req: ApiRequest) -> ApiResponse {
        let _mutation = self.env.candidate_mutation.lock().await;
        let command_id = req
            .params
            .get("command_id")
            .and_then(|value| value.as_str())
            .unwrap_or(&req.id)
            .to_string();
        if command_id.is_empty()
            || command_id.len() > 128
            || !command_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
        {
            return ApiResponse::error(
                req.id,
                "Candidate command_id must be an identifier of at most 128 characters",
            );
        }
        let request_hash = blake3::hash(req.params.to_string().as_bytes())
            .to_hex()
            .to_string();
        let coordinator = match self.env.workspace.as_ref() {
            Some(c) => c,
            None => {
                return ApiResponse::error(
                    req.id,
                    "WorkspaceCoordinator not configured on daemon",
                )
            }
        };

        let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
            Some(id)
                if !id.trim().is_empty()
                    && id.trim().len() <= 128
                    && id
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')) =>
            {
                id.trim().to_string()
            }
            _ => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
        };

        match self.kernel.task_service.get_task(&task_id).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return ApiResponse::error(req.id, format!("Task '{task_id}' not found"))
            }
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        }

        let recorded = match coordinator.list_workspaces().await {
            Ok(workspaces) => workspaces
                .into_iter()
                .filter(|workspace| {
                    workspace
                        .metadata
                        .get("batch_command_id")
                        .and_then(|value| value.as_str())
                        == Some(&command_id)
                })
                .collect::<Vec<_>>(),
            Err(error) => return ApiResponse::error(req.id, error.to_string()),
        };
        if !recorded.is_empty() {
            if recorded.iter().any(|workspace| {
                workspace.owner_task_id.as_deref() != Some(&task_id)
                    || workspace
                        .metadata
                        .get("batch_request_hash")
                        .and_then(|value| value.as_str())
                        != Some(&request_hash)
            }) {
                return ApiResponse::error(
                    req.id,
                    "This command_id already belongs to a different candidate request",
                );
            }
            let requested = req
                .params
                .get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(2)
                .clamp(1, 8) as usize;
            let pending = recorded.iter().any(|workspace| !workspace.is_ready());
            return ApiResponse::success(
                req.id,
                serde_json::json!({
                    "batch_command_id": command_id,
                    "task_id": task_id,
                    "candidates": recorded,
                    "failed": [],
                    "partial": pending || requested != recorded.len(),
                    "needs_reconciliation": pending || requested != recorded.len(),
                    "replayed": true,
                }),
            );
        }

        let count = req
            .params
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;
        let count = count.clamp(1, 8);

        let base_repo_path = if let Some(base_ws_id) =
            req.params.get("base_workspace_id").and_then(|v| v.as_str())
        {
            match coordinator
                .get_workspace(&custos_domain::WorkspaceId::new(base_ws_id))
                .await
            {
                Ok(Some(ws)) if ws.is_ready() => ws.path,
                _ => {
                    return ApiResponse::error(
                        req.id,
                        format!("Base workspace '{base_ws_id}' not found"),
                    )
                }
            }
        } else if let Some(repo_path) = req.params.get("repo_path").and_then(|v| v.as_str())
        {
            repo_path.to_string()
        } else {
            return ApiResponse::error(
                req.id,
                "Either 'base_workspace_id' or 'repo_path' must be provided",
            );
        };

        let base_branch = match custos_adapters::host::workspace::git_ops::current_branch(
            std::path::Path::new(&base_repo_path),
        )
        .await
        {
            Ok(branch) => branch,
            Err(_) => {
                return ApiResponse::error(
                    req.id,
                    "Candidate creation requires a Git checkout",
                )
            }
        };
        if base_branch.is_empty() {
            return ApiResponse::error(
                req.id,
                "Candidate creation requires a named base branch",
            );
        }
        match custos_adapters::host::workspace::git_ops::is_working_tree_clean(
            std::path::Path::new(&base_repo_path),
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                return ApiResponse::error(
                    req.id,
                    "Candidate creation requires a clean base checkout; uncommitted files would not be copied",
                )
            }
            Err(_) => {
                return ApiResponse::error(req.id, "Could not inspect candidate base checkout")
            }
        }

        let base_commit_hash =
            if let Some(h) = req.params.get("base_commit_hash").and_then(|v| v.as_str()) {
                Some(h.to_string())
            } else {
                custos_adapters::host::workspace::git_ops::rev_parse(
                    std::path::Path::new(&base_repo_path),
                    "HEAD",
                )
                .await
            };

        let mut spawned_workspaces = Vec::new();
        let mut failed_workspaces = Vec::new();
        let batch_command_id = command_id;
        let base_path = std::path::Path::new(&base_repo_path);
        let (Some(parent), Some(repo_name)) = (base_path.parent(), base_path.file_name())
        else {
            return ApiResponse::error(
                req.id,
                "Candidate base must have a named parent directory",
            );
        };
        for i in 1..=count {
            let branch = format!("custos/candidate/{task_id}/{i}");
            let candidate_path = parent
                .join(".custos-worktrees")
                .join(repo_name)
                .join(&task_id)
                .join(format!("candidate-{}", i))
                .to_string_lossy()
                .to_string();

            let metadata = serde_json::json!({
                "candidate_index": i,
                "task_id": task_id,
                "branch": branch,
                "base_commit_hash": base_commit_hash,
                "batch_command_id": &batch_command_id,
                "batch_request_hash": &request_hash,
            });

            let create_req = CreateWorkspaceRequest {
                name: format!("candidate-{}-{}", task_id, i),
                kind: custos_domain::WorkspaceKind::Git {
                    repo_path: base_repo_path.clone(),
                    branch: branch.clone(),
                    base_commit: base_commit_hash.clone(),
                },
                path: candidate_path,
                lineage: Some(custos_domain::WorkspaceLineage {
                    parent_workspace_id: req
                        .params
                        .get("base_workspace_id")
                        .and_then(|v| v.as_str())
                        .map(custos_domain::WorkspaceId::new),
                    base_commit: base_commit_hash.clone(),
                    target_branch: Some(base_branch.clone()),
                    head_commit: None,
                }),
                owner_task_id: Some(task_id.clone()),
                metadata: Some(metadata),
                setup_script: req
                    .params
                    .get("setup_script")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            };

            match coordinator.create_workspace(create_req).await {
                Ok(ws) => spawned_workspaces.push(ws),
                Err(e) => {
                    failed_workspaces.push(serde_json::json!({
                        "candidate_index": i,
                        "branch": branch,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        if spawned_workspaces.is_empty() && !failed_workspaces.is_empty() {
            return ApiResponse::error(
                req.id,
                format!(
                    "Failed to create candidate worktrees: {:?}",
                    failed_workspaces
                ),
            );
        }

        let candidates_val = serde_json::to_value(&spawned_workspaces).unwrap_or_default();
        ApiResponse::success(
            req.id,
            serde_json::json!({
                "batch_command_id": batch_command_id,
                "task_id": task_id,
                "base_repo_path": base_repo_path,
                "base_commit_hash": base_commit_hash,
                "candidates": candidates_val,
                "failed": failed_workspaces,
                "partial": !failed_workspaces.is_empty(),
            }),
        )
    }
}
