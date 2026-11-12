use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn detect_workspace(&self, req: ApiRequest) -> ApiResponse {
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

        let detected = custos_adapters::host::workspace::git_ops::detect_repository(&canonical_path).await;

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
        } else if detected.is_git {
            "Git Repository"
        } else {
            "Generic Folder"
        };

        let kind = if detected.is_git { "git" } else { "folder" };

        let result = serde_json::json!({
            "name": name,
            "kind": kind,
            "project_type": project_type,
            "path": canonical_path.to_string_lossy(),
            "root_path": detected.root_path,
            "branch": detected.branch,
            "is_git": detected.is_git,
            "is_worktree": detected.is_worktree,
            "is_dirty": detected.is_dirty,
            "dirty_count": detected.dirty_count,
            "remote_url": detected.remote_url,
            "github_host": detected.github_host,
            "github_owner": detected.github_owner,
            "github_repo": detected.github_repo,
            "github_avatar_url": detected.github_avatar_url,
            "worktrees": detected.worktrees,
            "sub_repositories": detected.sub_repositories,
        });

        ApiResponse::success(req.id, result)
    }
}
