//! Candidate integration and merge reconciliation use case.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_integrate(&self, req: ApiRequest) -> ApiResponse {
        let _mutation = self.env.candidate_mutation.lock().await;
        let coordinator = match self.env.workspace.as_ref() {
            Some(c) => c,
            None => {
                return ApiResponse::error(
                    req.id,
                    "WorkspaceCoordinator not configured on daemon",
                )
            }
        };

        let winner_id = match req
            .params
            .get("winner_workspace_id")
            .and_then(|v| v.as_str())
        {
            Some(id) if !id.trim().is_empty() => custos_domain::WorkspaceId::new(id),
            _ => {
                return ApiResponse::error(
                    req.id,
                    "Missing 'winner_workspace_id' parameter",
                )
            }
        };

        let mut winner_ws = match coordinator.get_workspace(&winner_id).await {
            Ok(Some(ws)) => ws,
            _ => {
                return ApiResponse::error(
                    req.id,
                    format!("Winner workspace '{winner_id}' not found"),
                )
            }
        };

        let (repo_path, winner_branch) = match &winner_ws.kind {
            custos_domain::WorkspaceKind::Git {
                repo_path, branch, ..
            } => (repo_path.clone(), branch.clone()),
            _ => {
                return ApiResponse::error(
                    req.id,
                    "Winner workspace must be a Git workspace",
                )
            }
        };

        if winner_ws.owner_task_id.is_none() || !winner_ws.metadata.is_object() {
            return ApiResponse::error(
                req.id,
                "Integration requires a Task-owned candidate worktree",
            );
        }
        if let Some(intent) = winner_ws.metadata.get("integration").cloned() {
            if let Some(result) = intent.get("result").filter(|result| {
                result
                    .get("merge_success")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            }) {
                return ApiResponse::success(req.id, result.clone());
            }
            let recorded_head = intent
                .get("winner_head")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let recorded_target = intent
                .get("target_branch")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let target_ref = format!("refs/heads/{recorded_target}");
            if custos_adapters::host::workspace::git_ops::is_ancestor(
                std::path::Path::new(&repo_path),
                recorded_head,
                &target_ref,
            )
            .await
            {
                let result = serde_json::json!({
                    "winner_workspace_id": winner_id.to_string(),
                    "winner_branch": winner_branch,
                    "target_branch": intent.get("target_branch"),
                    "merge_success": true,
                    "merge_status": "merged_reconciled",
                    "merge_detail": "Recorded winner commit is already included in the recorded target branch; no merge was replayed",
                    "is_stale_base": false,
                    "loser_cleanup": [],
                });
                winner_ws.metadata["integration"]["result"] = result.clone();
                if let Err(error) = coordinator.update_workspace(&winner_ws).await {
                    return ApiResponse::error(req.id, error.to_string());
                }
                return ApiResponse::success(req.id, result);
            }
            let conflict = custos_adapters::host::workspace::git_ops::has_merge_head(
                std::path::Path::new(&repo_path),
            )
            .await;
            return ApiResponse::success(
                req.id,
                serde_json::json!({
                    "winner_workspace_id": winner_id.to_string(),
                    "winner_branch": winner_branch,
                    "target_branch": intent.get("target_branch"),
                    "merge_success": false,
                    "merge_status": if conflict { "conflict_needs_resolution" } else { "uncertain" },
                    "merge_detail": "An earlier integration has no confirmed result. Inspect the target repository; the command was not replayed.",
                    "is_stale_base": false,
                    "loser_cleanup": [],
                }),
            );
        }
        let target_branch = req
            .params
            .get("target_branch")
            .and_then(|v| v.as_str())
            .or(winner_ws.lineage.target_branch.as_deref());
        let Some(target_branch) = target_branch.map(str::to_string) else {
            return ApiResponse::error(req.id, "Candidate has no recorded target branch");
        };

        if !winner_ws.is_ready() {
            return ApiResponse::error(
                req.id,
                "Winner workspace is not ready for integration",
            );
        }

        // `git merge` modifies the branch currently checked out in repo_path,
        // not the branch named by target_branch. Refuse a mismatched checkout.
        let checked_out = match custos_adapters::host::workspace::git_ops::current_branch(
            std::path::Path::new(&repo_path),
        )
        .await
        {
            Ok(branch) => branch,
            Err(_) => {
                return ApiResponse::error(
                    req.id,
                    "Could not verify the checked-out target branch",
                )
            }
        };
        if checked_out != target_branch {
            return ApiResponse::error(req.id, format!(
                "Target checkout is on '{checked_out}', not '{target_branch}'; integration was not started"
            ));
        }

        for (label, path) in [
            ("target", repo_path.as_str()),
            ("winner", winner_ws.path.as_str()),
        ] {
            match custos_adapters::host::workspace::git_ops::is_working_tree_clean(
                std::path::Path::new(path),
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => {
                    return ApiResponse::error(
                        req.id,
                        format!(
                            "The {label} workspace has uncommitted or untracked changes; review them before integration"
                        ),
                    )
                }
                Err(_) => {
                    return ApiResponse::error(
                        req.id,
                        format!("Could not inspect the {label} workspace"),
                    )
                }
            }
        }

        match custos_adapters::host::workspace::git_ops::current_branch(std::path::Path::new(
            &winner_ws.path,
        ))
        .await
        {
            Ok(branch) if branch == winner_branch => {}
            _ => {
                return ApiResponse::error(
                    req.id,
                    "Winner workspace no longer checks out its recorded branch",
                )
            }
        }

        // Stale-Base Drift Check
        let current_target_head = custos_adapters::host::workspace::git_ops::rev_parse(
            std::path::Path::new(&repo_path),
            &target_branch,
        )
        .await;

        let (Some(ref current_head), Some(ref base_commit)) =
            (&current_target_head, &winner_ws.base_commit_hash)
        else {
            return ApiResponse::error(
                req.id,
                "Cannot verify candidate base against target HEAD",
            );
        };
        let is_stale_base = current_head != base_commit;
        if is_stale_base {
            return ApiResponse::error(req.id, format!(
                "Candidate base {base_commit} is stale relative to target {current_head}; integration was not started"
            ));
        }

        let winner_head = match custos_adapters::host::workspace::git_ops::rev_parse(
            std::path::Path::new(&winner_ws.path),
            "HEAD",
        )
        .await
        {
            Some(head) => head,
            None => return ApiResponse::error(req.id, "Could not verify winner HEAD"),
        };
        let preflight = serde_json::json!({
            "winner_workspace_id": winner_id.to_string(),
            "winner_branch": winner_branch,
            "target_branch": target_branch,
            "current_target_head": current_target_head,
            "winner_head": winner_head,
            "winner_base_commit": winner_ws.base_commit_hash,
            "is_stale_base": false,
            "preview": true,
            "merge_success": false,
            "merge_detail": "Ready for explicit integration",
            "merge_status": "ready",
        });
        if req.params.get("preview").and_then(|value| value.as_bool()) == Some(true) {
            return ApiResponse::success(req.id, preflight);
        }
        if req
            .params
            .get("expected_target_head")
            .and_then(|value| value.as_str())
            != current_target_head.as_deref()
            || req
                .params
                .get("expected_winner_head")
                .and_then(|value| value.as_str())
                != Some(&winner_head)
        {
            return ApiResponse::error(req.id, "Review a fresh integration preview before applying; expected heads are missing or changed");
        }
        winner_ws.metadata["integration"] = serde_json::json!({
            "winner_head": winner_head,
            "target_head": current_target_head,
            "target_branch": target_branch,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(error) = coordinator.update_workspace(&winner_ws).await {
            return ApiResponse::error(req.id, error.to_string());
        }

        // Merge winner branch into target repository
        let merge_msg = format!(
            "Merge winner candidate {} ({})",
            winner_ws.name, winner_branch
        );
        let outcome = custos_adapters::host::workspace::git_ops::merge_no_ff(
            std::path::Path::new(&repo_path),
            &winner_branch,
            &merge_msg,
        )
        .await;
        let (merge_success, merge_detail, conflict_detected) =
            (outcome.success, outcome.detail, outcome.conflict_detected);

        let merge_status = if merge_success {
            "merged"
        } else if conflict_detected {
            "conflict_needs_resolution"
        } else {
            "failed"
        };

        // Integration never implies permission to delete other workspaces.
        // Candidate cleanup is a separate, explicit archive operation.
        let loser_cleanup_results: Vec<serde_json::Value> = Vec::new();

        let result = serde_json::json!({
            "winner_workspace_id": winner_id.to_string(),
            "winner_branch": winner_branch,
            "target_branch": target_branch,
            "is_stale_base": is_stale_base,
            "current_target_head": current_target_head,
            "winner_base_commit": winner_ws.base_commit_hash,
            "merge_success": merge_success,
            "merge_status": merge_status,
            "conflict_detected": conflict_detected,
            "merge_detail": merge_detail,
            "loser_cleanup": loser_cleanup_results,
        });
        winner_ws.metadata["integration"]["result"] = result.clone();
        if let Err(error) = coordinator.update_workspace(&winner_ws).await {
            return ApiResponse::error(
                req.id,
                format!("Integration effect requires reconciliation: {error}"),
            );
        }
        ApiResponse::success(req.id, result)
    }
}
