//! Workspace candidate spawn, compare, and integrate use cases.

use crate::api::*;
use custos_runtime::workspace::CreateWorkspaceRequest;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces_changes(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_CANDIDATES_SPAWN => {
                let _mutation = self.candidate_mutation.lock().await;
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
                let coordinator = match self.workspace.as_ref() {
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

                match self.task_service.get_task(&task_id).await {
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
                        serde_json::json!({"batch_command_id": command_id, "task_id": task_id,
                        "candidates": recorded, "failed": [], "partial": pending || requested != recorded.len(), "needs_reconciliation": pending || requested != recorded.len(), "replayed": true}),
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

                let base_branch = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let base_branch = match base_branch {
                    Ok(out) if out.status.success() => {
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    }
                    _ => {
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
                let base_status = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["status", "--porcelain", "--untracked-files=all"])
                    .output()
                    .await;
                match base_status {
                    Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                    Ok(out) if out.status.success() => return ApiResponse::error(req.id, "Candidate creation requires a clean base checkout; uncommitted files would not be copied"),
                    _ => return ApiResponse::error(req.id, "Could not inspect candidate base checkout"),
                }

                let base_commit_hash =
                    if let Some(h) = req.params.get("base_commit_hash").and_then(|v| v.as_str()) {
                        Some(h.to_string())
                    } else {
                        let mut cmd = tokio::process::Command::new("git");
                        cmd.current_dir(&base_repo_path)
                            .arg("rev-parse")
                            .arg("HEAD");
                        cmd.output()
                            .await
                            .ok()
                            .filter(|o| o.status.success())
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
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
            METHOD_WORKSPACES_CANDIDATES_COMPARE => {
                let coordinator = match self.workspace.as_ref() {
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
            METHOD_WORKSPACES_INTEGRATE => {
                let _mutation = self.candidate_mutation.lock().await;
                let coordinator = match self.workspace.as_ref() {
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
                    let ancestry = tokio::process::Command::new("git")
                        .current_dir(&repo_path)
                        .args(["merge-base", "--is-ancestor", recorded_head, &target_ref])
                        .output()
                        .await;
                    if matches!(ancestry, Ok(ref output) if output.status.success()) {
                        let result = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                            "target_branch": intent.get("target_branch"), "merge_success": true, "merge_status": "merged_reconciled",
                            "merge_detail": "Recorded winner commit is already included in the recorded target branch; no merge was replayed", "is_stale_base": false, "loser_cleanup": []});
                        winner_ws.metadata["integration"]["result"] = result.clone();
                        if let Err(error) = coordinator.update_workspace(&winner_ws).await {
                            return ApiResponse::error(req.id, error.to_string());
                        }
                        return ApiResponse::success(req.id, result);
                    }
                    let merge_head = tokio::process::Command::new("git")
                        .current_dir(&repo_path)
                        .args(["rev-parse", "--verify", "MERGE_HEAD"])
                        .output()
                        .await;
                    let conflict = matches!(merge_head, Ok(ref output) if output.status.success());
                    return ApiResponse::success(
                        req.id,
                        serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                        "target_branch": intent.get("target_branch"), "merge_success": false, "merge_status": if conflict { "conflict_needs_resolution" } else { "uncertain" },
                        "merge_detail": "An earlier integration has no confirmed result. Inspect the target repository; the command was not replayed.", "is_stale_base": false, "loser_cleanup": []}),
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
                let checked_out = tokio::process::Command::new("git")
                    .current_dir(&repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let checked_out = match checked_out {
                    Ok(out) if out.status.success() => {
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    }
                    _ => {
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
                    let status = tokio::process::Command::new("git")
                        .current_dir(path)
                        .args(["status", "--porcelain", "--untracked-files=all"])
                        .output()
                        .await;
                    match status {
                        Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                        Ok(out) if out.status.success() => return ApiResponse::error(req.id, format!(
                            "The {label} workspace has uncommitted or untracked changes; review them before integration"
                        )),
                        _ => return ApiResponse::error(req.id, format!("Could not inspect the {label} workspace")),
                    }
                }

                let winner_checkout = tokio::process::Command::new("git")
                    .current_dir(&winner_ws.path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                match winner_checkout {
                    Ok(out)
                        if out.status.success()
                            && String::from_utf8_lossy(&out.stdout).trim() == winner_branch => {}
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Winner workspace no longer checks out its recorded branch",
                        )
                    }
                }

                // Stale-Base Drift Check
                let mut rev_cmd = tokio::process::Command::new("git");
                rev_cmd
                    .current_dir(&repo_path)
                    .arg("rev-parse")
                    .arg(&target_branch);
                let current_target_head = rev_cmd
                    .output()
                    .await
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

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

                let winner_head = match tokio::process::Command::new("git")
                    .current_dir(&winner_ws.path)
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .await
                {
                    Ok(output) if output.status.success() => {
                        String::from_utf8_lossy(&output.stdout).trim().to_string()
                    }
                    _ => return ApiResponse::error(req.id, "Could not verify winner HEAD"),
                };
                let preflight = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                    "target_branch": target_branch, "current_target_head": current_target_head, "winner_head": winner_head,
                    "winner_base_commit": winner_ws.base_commit_hash, "is_stale_base": false, "preview": true,
                    "merge_success": false, "merge_detail": "Ready for explicit integration", "merge_status": "ready"});
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
                winner_ws.metadata["integration"] = serde_json::json!({"winner_head": winner_head, "target_head": current_target_head,
                    "target_branch": target_branch, "started_at": chrono::Utc::now().to_rfc3339()});
                if let Err(error) = coordinator.update_workspace(&winner_ws).await {
                    return ApiResponse::error(req.id, error.to_string());
                }

                // Merge winner branch into target repository
                let mut merge_cmd = tokio::process::Command::new("git");
                merge_cmd
                    .current_dir(&repo_path)
                    .arg("merge")
                    .arg("--no-ff")
                    .arg("-m")
                    .arg(format!(
                        "Merge winner candidate {} ({})",
                        winner_ws.name, winner_branch
                    ))
                    .arg(&winner_branch);

                let merge_output = merge_cmd.output().await;
                let (merge_success, merge_detail, conflict_detected) = match merge_output {
                    Ok(out) => {
                        let text = String::from_utf8_lossy(&out.stdout).to_string()
                            + &String::from_utf8_lossy(&out.stderr);
                        let success = out.status.success();
                        let conflict = !success
                            && (text.contains("CONFLICT")
                                || text.contains("Automatic merge failed"));
                        (success, text.trim().to_string(), conflict)
                    }
                    Err(e) => (false, format!("Failed to spawn git merge: {e}"), false),
                };

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
            unknown => ApiResponse::error(req.id, format!("Unknown changes method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #[test]
    fn test_candidate_command_id_validation() {
        let valid_id = "cmd-123_abc";
        assert!(
            !valid_id.is_empty()
                && valid_id.len() <= 128
                && valid_id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        );

        let invalid_id = "cmd with spaces!";
        assert!(
            !invalid_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        );
    }
}
