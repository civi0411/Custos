//! Git Worktree Discovery & GitHub Remote Parsing Utilities

use std::path::{Path, PathBuf};

pub static GITHUB_LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn github_login_command(path: &Path) -> String {
    let executable = path.to_string_lossy().replace('\'', "'\\''");
    format!("exec '{executable}' auth login --hostname github.com --git-protocol https --web --skip-ssh-key")
}

pub fn github_auth_metadata(success: bool, stdout: &[u8]) -> serde_json::Value {
    let user = serde_json::from_slice::<serde_json::Value>(stdout).ok();
    if success
        && user
            .as_ref()
            .and_then(|u| u.get("login"))
            .and_then(|v| v.as_str())
            .is_some_and(|login| !login.trim().is_empty())
    {
        // Whitelist profile metadata; never forward arbitrary CLI output.
        let user = user.as_ref();
        return serde_json::json!({"authenticated": true, "source": "gh_cli", "installed": true, "user": {
            "login": user.and_then(|u| u.get("login")),
            "name": user.and_then(|u| u.get("name")),
            "avatar_url": user.and_then(|u| u.get("avatar_url")),
            "html_url": user.and_then(|u| u.get("html_url"))
        }});
    }
    serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true,
        "error": "GitHub account could not be verified. Connect with gh, or check network access and retry."})
}

/// Parses a Git remote URL into (host, owner, repo, avatar_url).
/// Supports SSH, HTTPS, git://, and custom enterprise GitHub hosts.
pub fn parse_github_remote(
    remote_url: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return (None, None, None, None);
    }

    let s = raw.strip_suffix(".git").unwrap_or(raw);

    // Case 1: SCP-like syntax: [user@]host:owner/repo (only when no scheme :// is present)
    if !s.contains("://") {
        if let Some((left, path)) = s.split_once(':') {
            if !left.contains('/') {
                let host = left.split('@').next_back().unwrap_or(left).trim();
                let path = path.trim().trim_start_matches('/');
                let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
                if parts.len() >= 2 {
                    let owner = parts[0].to_string();
                    let repo = parts[1..].join("/");
                    let avatar = if host == "github.com" {
                        Some(format!("https://github.com/{}.png", owner))
                    } else {
                        None
                    };
                    return (Some(host.to_string()), Some(owner), Some(repo), avatar);
                }
            }
        }
    }

    // Case 2: URL syntax: [scheme://][user[:pass]@]host[:port]/owner/repo
    let after_scheme = if let Some(idx) = s.find("://") {
        &s[idx + 3..]
    } else {
        s
    };

    let after_auth = if let Some((_, rest)) = after_scheme.split_once('@') {
        rest
    } else {
        after_scheme
    };

    if let Some((host_part, path_part)) = after_auth.split_once('/') {
        let host = host_part.split(':').next().unwrap_or(host_part).trim();
        let parts: Vec<&str> = path_part.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            let owner = parts[0].to_string();
            let repo = parts[1..].join("/");
            let avatar = if host == "github.com" {
                Some(format!("https://github.com/{}.png", owner))
            } else {
                None
            };
            return (Some(host.to_string()), Some(owner), Some(repo), avatar);
        }
    }

    (None, None, None, None)
}

/// Resolves actual gitdir and common gitdir for a path (handling both `.git` directory and `.git` file for worktrees/submodules).
pub async fn resolve_deep_git_paths(path: &Path) -> Option<(PathBuf, PathBuf, bool)> {
    let dot_git = path.join(".git");
    if !dot_git.exists() {
        return None;
    }

    if dot_git.is_dir() {
        return Some((dot_git.clone(), dot_git, false));
    }

    if dot_git.is_file() {
        if let Ok(content) = tokio::fs::read_to_string(&dot_git).await {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("gitdir:") {
                    let gitdir_str = rest.trim();
                    let gitdir_path = if Path::new(gitdir_str).is_absolute() {
                        PathBuf::from(gitdir_str)
                    } else {
                        path.join(gitdir_str)
                    };
                    let gitdir_canonical = gitdir_path.canonicalize().unwrap_or(gitdir_path);

                    let commondir_file = gitdir_canonical.join("commondir");
                    let commondir_canonical = if commondir_file.is_file() {
                        if let Ok(common_content) = tokio::fs::read_to_string(&commondir_file).await
                        {
                            let common_str = common_content.trim();
                            let common_path = if Path::new(common_str).is_absolute() {
                                PathBuf::from(common_str)
                            } else {
                                gitdir_canonical.join(common_str)
                            };
                            common_path.canonicalize().unwrap_or(common_path)
                        } else {
                            gitdir_canonical.clone()
                        }
                    } else {
                        gitdir_canonical.clone()
                    };

                    return Some((gitdir_canonical, commondir_canonical, true));
                }
            }
        }
    }

    None
}

/// Lists all worktrees for a Git repository via `git worktree list --porcelain`.
pub async fn list_git_worktrees(git_bin: &Path, repo_path: &str) -> Vec<serde_json::Value> {
    let mut worktrees = Vec::new();
    let output = match tokio::process::Command::new(git_bin)
        .current_dir(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .await
    {
        Ok(out) if out.status.success() => out,
        _ => return worktrees,
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;
    let mut current_detached = false;
    let mut is_first = true;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(p) = current_path.take() {
                worktrees.push(serde_json::json!({
                    "path": p,
                    "head": current_head.take(),
                    "branch": current_branch.take(),
                    "is_main": is_first,
                    "is_bare": current_bare,
                    "is_detached": current_detached,
                }));
                is_first = false;
            }
            current_bare = false;
            current_detached = false;
            continue;
        }

        if let Some(p) = trimmed.strip_prefix("worktree ") {
            current_path = Some(p.trim().to_string());
        } else if let Some(h) = trimmed.strip_prefix("HEAD ") {
            current_head = Some(h.trim().to_string());
        } else if let Some(b) = trimmed.strip_prefix("branch ") {
            let b_ref = b.trim();
            let short = b_ref.strip_prefix("refs/heads/").unwrap_or(b_ref);
            current_branch = Some(short.to_string());
        } else if trimmed == "bare" {
            current_bare = true;
        } else if trimmed == "detached" {
            current_detached = true;
        }
    }

    if let Some(p) = current_path.take() {
        worktrees.push(serde_json::json!({
            "path": p,
            "head": current_head.take(),
            "branch": current_branch.take(),
            "is_main": is_first,
            "is_bare": current_bare,
            "is_detached": current_detached,
        }));
    }

    worktrees
}

