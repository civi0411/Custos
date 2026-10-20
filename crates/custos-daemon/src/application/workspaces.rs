//! workspaces application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces(&self, req: ApiRequest) -> ApiResponse {
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
                    Some(id) => custos_domain::WorkspaceId::new(id),
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
                let ws_id = custos_domain::WorkspaceId::new(params.workspace_id);
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
                    Some(id) => custos_domain::WorkspaceId::new(id),
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
                        let ws_id = custos_domain::WorkspaceId::new(id);
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
            METHOD_WORKSPACES_DETECT => {
                let raw_path_str = req
                    .params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .unwrap_or("");
                let expanded_path = if raw_path_str.is_empty() || raw_path_str == "." {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                } else if raw_path_str.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&raw_path_str[2..])
                    } else {
                        std::path::PathBuf::from(raw_path_str)
                    }
                } else if raw_path_str == "~" {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(raw_path_str))
                } else {
                    std::path::PathBuf::from(raw_path_str)
                };

                let canonical_path = match expanded_path.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        if let Ok(curr) = std::env::current_dir().and_then(|c| c.canonicalize()) {
                            curr
                        } else {
                            return ApiResponse::error(
                                req.id,
                                format!(
                                    "Directory '{}' does not exist or cannot be accessed: {}",
                                    raw_path_str, e
                                ),
                            );
                        }
                    }
                };

                if !canonical_path.is_dir() {
                    return ApiResponse::error(
                        req.id,
                        format!("Path '{}' is not a directory", raw_path_str),
                    );
                }

                // 1. Deep Probe Git (files vs dirs, worktrees, commondir)
                let git_bin = resolve_git_binary();
                let deep_git_probe = resolve_deep_git_paths(&canonical_path).await;

                let mut is_git = deep_git_probe.is_some();
                let mut root_path = canonical_path.to_string_lossy().to_string();
                let mut branch: Option<String> = None;
                let mut is_dirty = false;
                let mut dirty_count = 0;
                let mut remote_url: Option<String> = None;
                let mut is_worktree = deep_git_probe
                    .as_ref()
                    .map(|(_, _, wt)| *wt)
                    .unwrap_or(false);

                // Check via git rev-parse for root work-tree toplevel
                let toplevel_res = tokio::process::Command::new(&git_bin)
                    .current_dir(&canonical_path)
                    .args(["rev-parse", "--is-inside-work-tree", "--show-toplevel"])
                    .output()
                    .await;

                if let Ok(out) = toplevel_res {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let lines: Vec<&str> = stdout.lines().collect();
                        if let Some(first) = lines.first() {
                            if *first == "true" {
                                is_git = true;
                                if let Some(tl) = lines.get(1) {
                                    root_path = tl.trim().to_string();
                                }
                            }
                        }
                    }
                }

                let mut worktrees: Vec<serde_json::Value> = Vec::new();
                let mut github_host: Option<String> = None;
                let mut github_owner: Option<String> = None;
                let mut github_repo: Option<String> = None;
                let mut github_avatar_url: Option<String> = None;

                if is_git {
                    // Get current branch
                    if let Ok(b_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                        .output()
                        .await
                    {
                        if b_out.status.success() {
                            let b_str = String::from_utf8_lossy(&b_out.stdout).trim().to_string();
                            if !b_str.is_empty() {
                                branch = Some(b_str);
                            }
                        }
                    }

                    // Check dirty status
                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["status", "--porcelain=v1"])
                        .output()
                        .await
                    {
                        if s_out.status.success() {
                            let changes = String::from_utf8_lossy(&s_out.stdout);
                            dirty_count = changes.lines().filter(|l| !l.trim().is_empty()).count();
                            is_dirty = dirty_count > 0;
                        }
                    }

                    // Get remote URL (origin first, then upstream)
                    for remote_name in ["origin", "upstream"] {
                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                            .current_dir(&root_path)
                            .args(["remote", "get-url", remote_name])
                            .output()
                            .await
                        {
                            if r_out.status.success() {
                                let r_str =
                                    String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                                if !r_str.is_empty() {
                                    remote_url = Some(r_str);
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(ref r_url) = remote_url {
                        let (gh_host, gh_owner, gh_repo, gh_avatar) = parse_github_remote(r_url);
                        github_host = gh_host;
                        github_owner = gh_owner;
                        github_repo = gh_repo;
                        github_avatar_url = gh_avatar;
                    }

                    // Check if linked worktree
                    let dir_out = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--git-dir", "--git-common-dir"])
                        .output()
                        .await;
                    if let Ok(d_out) = dir_out {
                        if d_out.status.success() {
                            let out_str = String::from_utf8_lossy(&d_out.stdout);
                            let d_lines: Vec<&str> = out_str.lines().collect();
                            if d_lines.len() >= 2 && d_lines[0] != d_lines[1] {
                                is_worktree = true;
                            }
                        }
                    }

                    // List all worktrees for this repository
                    worktrees = list_git_worktrees(&git_bin, &root_path).await;
                }

                // 1b. Scan immediate subdirectories for child Git repositories / workspaces
                let mut sub_repositories: Vec<serde_json::Value> = Vec::new();
                if let Ok(mut dir_entries) = tokio::fs::read_dir(&canonical_path).await {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                        if let Ok(file_type) = entry.file_type().await {
                            if file_type.is_dir() {
                                let sub_dir = entry.path();
                                if let Some((_sub_gitdir, _sub_commondir, _sub_is_wt)) =
                                    resolve_deep_git_paths(&sub_dir).await
                                {
                                    let sub_name = entry.file_name().to_string_lossy().to_string();
                                    let mut sub_branch = String::from("main");
                                    if let Ok(sub_b_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                                        .output()
                                        .await
                                    {
                                        if sub_b_out.status.success() {
                                            let s = String::from_utf8_lossy(&sub_b_out.stdout)
                                                .trim()
                                                .to_string();
                                            if !s.is_empty() {
                                                sub_branch = s;
                                            }
                                        }
                                    }

                                    let mut sub_dirty_count = 0;
                                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["status", "--porcelain=v1"])
                                        .output()
                                        .await
                                    {
                                        if s_out.status.success() {
                                            let changes = String::from_utf8_lossy(&s_out.stdout);
                                            sub_dirty_count = changes
                                                .lines()
                                                .filter(|l| !l.trim().is_empty())
                                                .count();
                                        }
                                    }

                                    let mut sub_remote_url: Option<String> = None;
                                    for remote_name in ["origin", "upstream"] {
                                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                                            .current_dir(&sub_dir)
                                            .args(["remote", "get-url", remote_name])
                                            .output()
                                            .await
                                        {
                                            if r_out.status.success() {
                                                let r = String::from_utf8_lossy(&r_out.stdout)
                                                    .trim()
                                                    .to_string();
                                                if !r.is_empty() {
                                                    sub_remote_url = Some(r);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    let (sub_gh_host, sub_gh_owner, sub_gh_repo, sub_gh_avatar) =
                                        if let Some(ref r) = sub_remote_url {
                                            parse_github_remote(r)
                                        } else {
                                            (None, None, None, None)
                                        };

                                    let sub_worktrees =
                                        list_git_worktrees(&git_bin, &sub_dir.to_string_lossy())
                                            .await;

                                    sub_repositories.push(serde_json::json!({
                                        "name": sub_name,
                                        "path": sub_dir.to_string_lossy().to_string(),
                                        "branch": sub_branch,
                                        "is_git": true,
                                        "is_dirty": sub_dirty_count > 0,
                                        "dirty_count": sub_dirty_count,
                                        "remote_url": sub_remote_url,
                                        "github_host": sub_gh_host,
                                        "github_owner": sub_gh_owner,
                                        "github_repo": sub_gh_repo,
                                        "github_avatar_url": sub_gh_avatar,
                                        "worktrees": sub_worktrees
                                    }));
                                }
                            }
                        }
                    }
                }

                // 2. Detect Project Type
                let root_p = std::path::Path::new(&root_path);
                let project_type = if root_p.join("Cargo.toml").exists() {
                    "Rust"
                } else if root_p.join("package.json").exists() {
                    "TypeScript / Node"
                } else if root_p.join("pyproject.toml").exists()
                    || root_p.join("requirements.txt").exists()
                {
                    "Python"
                } else if root_p.join("go.mod").exists() {
                    "Go"
                } else if root_p.join("pom.xml").exists() || root_p.join("build.gradle").exists() {
                    "Java"
                } else if is_git {
                    "Git Repository"
                } else {
                    "Folder"
                };

                let name = root_p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string());

                let result = serde_json::json!({
                    "path": canonical_path.to_string_lossy(),
                    "root_path": root_path,
                    "name": name,
                    "is_git": is_git,
                    "branch": branch,
                    "is_dirty": is_dirty,
                    "dirty_count": dirty_count,
                    "remote_url": remote_url,
                    "github_host": github_host,
                    "github_owner": github_owner,
                    "github_repo": github_repo,
                    "github_avatar_url": github_avatar_url,
                    "project_type": project_type,
                    "is_worktree": is_worktree,
                    "worktrees": worktrees,
                    "sub_repositories": sub_repositories
                });

                ApiResponse::success(req.id, result)
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
            METHOD_GITHUB_ACCOUNTS => {
                let gh = match resolve_gh_binary() {
                    Some(gh) => gh,
                    None => {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({"installed":false,"accounts":[]}),
                        )
                    }
                };
                match custos_adapters::repository_git::github_accounts(&gh).await {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GIT_REPOSITORY
            | METHOD_GIT_SWITCH
            | METHOD_GIT_FETCH
            | METHOD_GIT_ACCOUNT
            | METHOD_GIT_GITHUB => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.is_empty() => id,
                    _ => return ApiResponse::error(req.id, "An exact workspace_id is required"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(response) => return response,
                };
                if !matches!(ws.kind, custos_domain::WorkspaceKind::Git { .. }) {
                    return ApiResponse::error(req.id,"Local registered Git checkout required; remote-host Git routing is not available here");
                }
                let _guard = self.candidate_mutation.lock().await;
                let path = std::path::Path::new(&ws.path);
                let gh = resolve_gh_binary();
                let result: Result<serde_json::Value,DomainError> = async {
                    use custos_adapters::repository_git as git;
                    tracing::debug!(request_id = %req.id, workspace_id, method = %req.method, "Repository Git operation");
                    match req.method.as_str() {
                        METHOD_GIT_REPOSITORY => git::repository(path).await,
                        METHOD_GIT_ACCOUNT => {
                            let binding = req.params.get("account").filter(|v| !v.is_null()).map(|v| serde_json::from_value::<git::GitHubBinding>(v.clone()).map_err(|_| DomainError::Validation("Invalid account preference".into()))).transpose()?;
                            git::bind_account(path,gh.as_deref(),binding.as_ref()).await?;
                            git::repository(path).await
                        }
                        METHOD_GIT_GITHUB => {
                            let binding = git::account_binding(path).await?.ok_or_else(|| DomainError::Validation("Choose a repository GitHub account first".into()))?;
                            git::github_viewer(gh.as_deref().ok_or_else(|| DomainError::Validation("GitHub CLI is not installed".into()))?,&binding).await
                        }
                        METHOD_GIT_FETCH => {
                            let remote = req.params.get("remote").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit remote required".into()))?;
                            git::fetch_remote(path,gh.as_deref(),remote).await
                        }
                        METHOD_GIT_SWITCH => {
                            let branch = req.params.get("branch").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit branch required".into()))?;
                            let expected = req.params.get("expected_head").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Observed HEAD required".into()))?;
                            if self.terminal.list_sessions(Some(workspace_id)).await?.iter().any(|s| matches!(s.status,custos_domain::TerminalSessionStatus::Active)) {
                                return Err(DomainError::Conflict("Close active checkout terminals before switching, or use a new worktree".into()));
                            }
                            let runs = self.usage_runs.as_ref().ok_or_else(|| DomainError::Validation("Cannot verify active runs; branch switch unavailable".into()))?;
                            for task in self.task_service.list_tasks().await? {
                                for run in runs.list_runs_for_task(&task.id).await? {
                                    let target = run.metadata.get("execution_workspace_id").or_else(||run.metadata.get("workspace_id")).and_then(|v|v.as_str());
                                    if !run.status.is_terminal() && (target.is_none() || target==Some(workspace_id)) {
                                        return Err(DomainError::Conflict("Active or unscoped run prevents branch switching; use a separate worktree".into()));
                                    }
                                }
                            }
                            git::switch_branch(path,branch,expected).await
                        }
                        _ => Err(DomainError::Validation("Unsupported Git operation".into())),
                    }
                }.await;
                match result {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_LOGIN => {
                let _login_guard = GITHUB_LOGIN_LOCK.lock().await;
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::error(
                        req.id,
                        "Install GitHub CLI (gh) to connect. Local Git/worktrees remain available.",
                    ),
                };
                // Reuse the bounded PTY lifecycle, never the model OAuth singleton.
                let existing = self.terminal.list_sessions(Some("github-auth")).await;
                if let Ok(sessions) = existing {
                    if let Some(session) = sessions.into_iter().find(|session| {
                        matches!(session.status, custos_domain::TerminalSessionStatus::Active)
                    }) {
                        return match serde_json::to_value(session) {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        };
                    }
                }
                let command = github_login_command(&gh);
                tracing::info!(request_id = %req.id, "Starting user-requested GitHub CLI login");
                match self
                    .terminal
                    .spawn(
                        "github-auth",
                        &std::env::temp_dir(),
                        Some(&command),
                        100,
                        28,
                    )
                    .await
                {
                    Ok(session) => match serde_json::to_value(session) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_AUTH_STATUS => {
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "authenticated": false, "source": "gh_cli", "installed": false,
                                "error": "GitHub CLI (gh) is not installed. Local Git/worktrees remain available."
                            }),
                        )
                    }
                };
                // The authenticated viewer is authoritative. Never export gh's token or
                // trust a cached profile / a fragment of multi-account auth-status text.
                let mut command = tokio::process::Command::new(gh);
                command.args(["api", "--hostname", "github.com", "user"]);
                command.kill_on_drop(true);
                let result =
                    tokio::time::timeout(std::time::Duration::from_secs(10), command.output())
                        .await;
                let value = match result {
                    Ok(Ok(output)) => github_auth_metadata(output.status.success(), &output.stdout),
                    Ok(Err(_)) => {
                        serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "Could not launch GitHub CLI."})
                    }
                    Err(_) => {
                        serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "GitHub verification timed out. Check the network and retry."})
                    }
                };
                ApiResponse::success(req.id, value)
            }
            METHOD_GITHUB_VIEWER => {
                let gh_bin = match resolve_gh_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "GitHub CLI (`gh`) is not installed or not in PATH",
                        );
                    }
                };

                match tokio::process::Command::new(&gh_bin)
                    .args(["api", "user"])
                    .output()
                    .await
                {
                    Ok(out) if out.status.success() => {
                        match serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(
                                req.id,
                                format!("Invalid JSON from gh api user: {e}"),
                            ),
                        }
                    }
                    Ok(out) => {
                        let err_msg = String::from_utf8_lossy(&out.stderr);
                        ApiResponse::error(
                            req.id,
                            format!("gh api user failed: {}", err_msg.trim()),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to run gh api user: {e}")),
                }
            }
            METHOD_WORKSPACE_FILES_TREE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };
                let relative_dir = req.params.get("relative_dir").and_then(|v| v.as_str());
                let max_depth = req
                    .params
                    .get("max_depth")
                    .and_then(|v| v.as_u64())
                    .map(|d| d as usize);

                match self.files.get_file_tree(&ws, relative_dir, max_depth).await {
                    Ok(tree) => match serde_json::to_value(&tree) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_READ => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let max_bytes = req
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .map(|b| b as usize);
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.read_file(&ws, path, max_bytes).await {
                    Ok(content) => match serde_json::to_value(&content) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_WRITE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c.to_string(),
                    None => return ApiResponse::error(req.id, "Missing content parameter"),
                };
                let create_parents = req
                    .params
                    .get("create_parents")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let overwrite = req
                    .params
                    .get("overwrite")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                let mut params =
                    custos_domain::WriteWorkspaceFileParams::new(ws.id.clone(), path, content);
                params.create_parents = create_parents;
                params.overwrite = overwrite;

                match self.files.write_file(&ws, params).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_diff(&ws, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_file_diff(&ws, path, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_STAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.stage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "staged": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_UNSTAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.unstage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "unstaged": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_DISCARD => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.discard_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "discarded": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_COMMIT => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) if !m.trim().is_empty() => m,
                    _ => return ApiResponse::error(req.id, "Missing message parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.commit(&ws, message).await {
                    Ok(out) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "committed": true, "output": out }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
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
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
