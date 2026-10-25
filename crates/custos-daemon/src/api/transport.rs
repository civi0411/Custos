//! Transport and Binary Resolution Utilities for Custos Local API

use std::sync::Mutex;
use std::path::{Path, PathBuf};

pub fn default_model_probe_base_url(provider_type: &str) -> Option<&'static str> {
    match provider_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => Some("https://api.openai.com/v1"),
        "anthropic" | "claude" => Some("https://api.anthropic.com/v1"),
        "gemini" | "google" => Some("https://generativelanguage.googleapis.com"),
        "copilot" => Some("https://api.githubcopilot.com"),
        "antigravity" => Some("https://cloudcode-pa.googleapis.com"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "local" | "ollama" => Some("http://localhost:11434"),
        "lmstudio" | "lm-studio" => Some("http://127.0.0.1:1234/v1"),
        _ => None,
    }
}

pub static CAFFEINATE_STATE: Mutex<(String, Option<std::process::Child>)> =
    Mutex::new((String::new(), None));

pub fn resolve_agent_browser_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("AGENT_BROWSER_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/agent-browser");
        if local_bin.exists() {
            return Some(local_bin);
        }
        let orca_bin = home.join(
            "Project/AgentHub/orca/node_modules/agent-browser/bin/agent-browser-darwin-arm64",
        );
        if orca_bin.exists() {
            return Some(orca_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("agent-browser")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_ssh_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SSH_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in [
        "/usr/bin/ssh",
        "/usr/local/bin/ssh",
        "/opt/homebrew/bin/ssh",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("ssh")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub static GITHUB_LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn github_login_command(path: &Path) -> String {
    let executable = path.to_string_lossy().replace('\'', "'\\''");
    format!("exec '{executable}' auth login --hostname github.com --git-protocol https --web --skip-ssh-key")
}

pub fn github_auth_metadata(success: bool, stdout: &[u8]) -> serde_json::Value {
    let user = serde_json::from_slice::<serde_json::Value>(stdout).ok();
    if success && user.as_ref().and_then(|u| u.get("login")).and_then(|v| v.as_str()).is_some_and(|login| !login.trim().is_empty()) {
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

pub fn resolve_gh_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("GH_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in [
        "/opt/homebrew/bin/gh",
        "/usr/local/bin/gh",
        "/usr/bin/gh",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/gh");
        if local_bin.exists() {
            return Some(local_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("gh")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_git_binary() -> PathBuf {
    if let Ok(path) = std::env::var("GIT_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    for candidate in [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/git",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("git")
}

/// Parses a Git remote URL into (host, owner, repo, avatar_url)
/// Supports SSH, HTTPS, git://, and custom enterprise GitHub hosts.
pub fn parse_github_remote(remote_url: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return (None, None, None, None);
    }

    let s = raw.strip_suffix(".git").unwrap_or(raw);

    // Case 1: SCP-like syntax: [user@]host:owner/repo (only when no scheme :// is present)
    if !s.contains("://") {
        if let Some((left, path)) = s.split_once(':') {
            if !left.contains('/') {
                let host = left.split('@').last().unwrap_or(left).trim();
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

/// Resolves actual gitdir and common gitdir for a path (handling both .git directory and .git file for worktrees/submodules)
pub async fn resolve_deep_git_paths(path: &Path) -> Option<(PathBuf, PathBuf, bool)> {
    let dot_git = path.join(".git");
    if !dot_git.exists() {
        return None;
    }

    if dot_git.is_dir() {
        // Standard repository
        return Some((dot_git.clone(), dot_git, false));
    }

    if dot_git.is_file() {
        // Linked worktree or submodule
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
                        if let Ok(common_content) = tokio::fs::read_to_string(&commondir_file).await {
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

/// Lists all worktrees for a Git repository via `git worktree list --porcelain`
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

pub fn browser_execution_adapter_connected() -> bool {
    std::env::var("CUSTOS_BROWSER_CONNECTED")
        .map(|v| v != "0" && v != "false")
        .unwrap_or_else(|_| resolve_agent_browser_binary().is_some())
}

pub fn remote_execution_adapter_connected() -> bool {
    std::env::var("CUSTOS_REMOTE_FLEET_CONNECTED")
        .map(|v| v != "0" && v != "false")
        .unwrap_or_else(|_| resolve_ssh_binary().is_some())
}
