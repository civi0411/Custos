//! Workspace inspection and file operations use cases over WorkspaceFilesPort and RepositoryGitPort.

use crate::api::*;
use custos_domain::{DomainError, WriteWorkspaceFileParams};

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces_inspection(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_DETECT => {
                let raw_path_str = req
                    .params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .unwrap_or("");
                let expanded_path = if raw_path_str.is_empty() || raw_path_str == "." {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                } else if let Some(stripped) = raw_path_str.strip_prefix("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(stripped)
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

                                    let mut sub_remote: Option<String> = None;
                                    for remote_name in ["origin", "upstream"] {
                                        if let Ok(r_out) =
                                            tokio::process::Command::new(&git_bin)
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
                                                    sub_remote = Some(r);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    let (
                                        sub_gh_host,
                                        sub_gh_owner,
                                        sub_gh_repo,
                                        sub_gh_avatar,
                                    ) = if let Some(ref r) = sub_remote {
                                        parse_github_remote(r)
                                    } else {
                                        (None, None, None, None)
                                    };

                                    let sub_worktrees = list_git_worktrees(
                                        &git_bin,
                                        &sub_dir.to_string_lossy(),
                                    )
                                    .await;

                                    sub_repositories.push(serde_json::json!({
                                        "name": sub_name,
                                        "path": sub_dir.to_string_lossy(),
                                        "is_git": true,
                                        "branch": sub_branch,
                                        "is_dirty": sub_dirty_count > 0,
                                        "dirty_count": sub_dirty_count,
                                        "remote_url": sub_remote,
                                        "github_host": sub_gh_host,
                                        "github_owner": sub_gh_owner,
                                        "github_repo": sub_gh_repo,
                                        "github_avatar_url": sub_gh_avatar,
                                        "worktrees": sub_worktrees,
                                    }));
                                }
                            }
                        }
                    }
                }

                // 2. Discover Project Type / Stack
                let name = canonical_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string();

                let project_type = if canonical_path.join("Cargo.toml").exists() {
                    "Rust Project"
                } else if canonical_path.join("package.json").exists() {
                    "Node.js / Web Project"
                } else if canonical_path.join("pyproject.toml").exists()
                    || canonical_path.join("requirements.txt").exists()
                {
                    "Python Project"
                } else if canonical_path.join("go.mod").exists() {
                    "Go Project"
                } else if is_git {
                    "Git Repository"
                } else {
                    "Generic Folder"
                };

                let kind = if is_git { "git" } else { "folder" };

                let result = serde_json::json!({
                    "name": name,
                    "kind": kind,
                    "project_type": project_type,
                    "path": canonical_path.to_string_lossy(),
                    "root_path": root_path,
                    "branch": branch,
                    "is_git": is_git,
                    "is_worktree": is_worktree,
                    "is_dirty": is_dirty,
                    "dirty_count": dirty_count,
                    "remote_url": remote_url,
                    "github_host": github_host,
                    "github_owner": github_owner,
                    "github_repo": github_repo,
                    "github_avatar_url": github_avatar_url,
                    "worktrees": worktrees,
                    "sub_repositories": sub_repositories,
                });

                ApiResponse::success(req.id, result)
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
                let result: Result<serde_json::Value, DomainError> = async {
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
                    WriteWorkspaceFileParams::new(ws.id.clone(), path, content);
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
            unknown => ApiResponse::error(req.id, format!("Unknown inspection method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_core::contracts::workspace_files::WorkspaceFilesPort;
    use custos_domain::{ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus};
    use custos_runtime::workspace::WorkspaceFilesCoordinator;

    #[tokio::test]
    async fn inspection_group_test_without_dispatcher() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ws = ExecutionWorkspace::new(
            WorkspaceId::new("ws-test-inspect"),
            "test-inspect",
            WorkspaceKind::Folder {
                path: tmp.path().to_string_lossy().to_string(),
            },
            tmp.path().to_string_lossy().to_string(),
        );
        ws.status = WorkspaceStatus::Ready;

        let coordinator = WorkspaceFilesCoordinator::new();

        // Write a test file
        let write_params = WriteWorkspaceFileParams::new(
            ws.id.clone(),
            "hello.txt",
            "Hello, inspection!".to_string(),
        );
        let write_res = coordinator.write_file(&ws, write_params)
            .await
            .unwrap();
        assert_eq!(write_res.content.len(), 18);

        // Read it back
        let content = coordinator.read_file(&ws, "hello.txt", None)
            .await
            .unwrap();
        assert_eq!(content.content, "Hello, inspection!");

        // Get file tree
        let tree = coordinator.get_file_tree(&ws, None, None)
            .await
            .unwrap();
        assert!(tree.entries.iter().any(|e| e.path == "hello.txt"));
    }
}
