//! Async host Git operations for workspace inspection, candidate spawning, and integration.

use custos_domain::DomainError;
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

use super::super::binaries::resolve_git_binary;
use super::git_discovery::{list_git_worktrees, parse_github_remote, resolve_deep_git_paths};

#[derive(Debug, Clone)]
pub struct GitMergeOutcome {
    pub success: bool,
    pub detail: String,
    pub conflict_detected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubRepositoryInfo {
    pub name: String,
    pub path: String,
    pub is_git: bool,
    pub branch: String,
    pub is_dirty: bool,
    pub dirty_count: usize,
    pub remote_url: Option<String>,
    pub github_host: Option<String>,
    pub github_owner: Option<String>,
    pub github_repo: Option<String>,
    pub github_avatar_url: Option<String>,
    pub worktrees: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct DetectedGitRepository {
    pub root_path: String,
    pub branch: Option<String>,
    pub is_git: bool,
    pub is_worktree: bool,
    pub is_dirty: bool,
    pub dirty_count: usize,
    pub remote_url: Option<String>,
    pub github_host: Option<String>,
    pub github_owner: Option<String>,
    pub github_repo: Option<String>,
    pub github_avatar_url: Option<String>,
    pub worktrees: Vec<serde_json::Value>,
    pub sub_repositories: Vec<SubRepositoryInfo>,
}

/// Returns the currently checked-out branch name (`git branch --show-current`).
pub async fn current_branch(repo_dir: &Path) -> Result<String, DomainError> {
    let out = Command::new("git")
        .current_dir(repo_dir)
        .args(["branch", "--show-current"])
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to run git branch: {e}")))?;
    if !out.status.success() {
        return Err(DomainError::Validation(
            "Workspace is not a valid Git checkout".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Returns `Ok(true)` when working tree and untracked files are clean, `Ok(false)` when dirty.
pub async fn is_working_tree_clean(repo_dir: &Path) -> Result<bool, DomainError> {
    let out = Command::new("git")
        .current_dir(repo_dir)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("Could not inspect workspace git status: {e}")))?;
    if !out.status.success() {
        return Err(DomainError::Validation(
            "Could not inspect workspace git status".into(),
        ));
    }
    Ok(out.stdout.is_empty())
}

/// Resolves a Git revision string (`HEAD`, branch name, etc.) to a commit hash.
pub async fn rev_parse(repo_dir: &Path, rev: &str) -> Option<String> {
    Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", rev])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Checks whether `commit` is an ancestor of `target_ref`.
pub async fn is_ancestor(repo_dir: &Path, commit: &str, target_ref: &str) -> bool {
    let out = Command::new("git")
        .current_dir(repo_dir)
        .args(["merge-base", "--is-ancestor", commit, target_ref])
        .output()
        .await;
    matches!(out, Ok(ref o) if o.status.success())
}

/// Merges `branch` into the current checkout using `--no-ff`.
pub async fn merge_no_ff(repo_dir: &Path, branch: &str, message: &str) -> GitMergeOutcome {
    let out = Command::new("git")
        .current_dir(repo_dir)
        .args(["merge", "--no-ff", "-m", message, branch])
        .output()
        .await;
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout).to_string()
                + &String::from_utf8_lossy(&o.stderr);
            let success = o.status.success();
            let conflict_detected = !success
                && (text.contains("CONFLICT") || text.contains("Automatic merge failed"));
            GitMergeOutcome {
                success,
                detail: text.trim().to_string(),
                conflict_detected,
            }
        }
        Err(e) => GitMergeOutcome {
            success: false,
            detail: format!("Failed to spawn git merge: {e}"),
            conflict_detected: false,
        },
    }
}

async fn read_git_remote(git_bin: &Path, dir: &Path) -> Option<String> {
    for remote_name in ["origin", "upstream"] {
        if let Ok(r_out) = Command::new(git_bin)
            .current_dir(dir)
            .args(["remote", "get-url", remote_name])
            .output()
            .await
        {
            if r_out.status.success() {
                let r = String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                if !r.is_empty() {
                    return Some(r);
                }
            }
        }
    }
    None
}

async fn read_dirty_count(git_bin: &Path, dir: &Path) -> usize {
    if let Ok(s_out) = Command::new(git_bin)
        .current_dir(dir)
        .args(["status", "--porcelain=v1"])
        .output()
        .await
    {
        if s_out.status.success() {
            return String::from_utf8_lossy(&s_out.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
        }
    }
    0
}

/// Probes a canonical directory path for Git repository status, worktrees, and child repositories.
pub async fn detect_repository(canonical_path: &Path) -> DetectedGitRepository {
    let git_bin = resolve_git_binary();
    let deep_git_probe = resolve_deep_git_paths(canonical_path).await;

    let mut is_git = deep_git_probe.is_some();
    let mut root_path = canonical_path.to_string_lossy().to_string();
    let mut branch: Option<String> = None;
    let mut is_dirty = false;
    let mut dirty_count = 0;
    let mut remote_url: Option<String> = None;
    let mut is_worktree = deep_git_probe.as_ref().map(|(_, _, wt)| *wt).unwrap_or(false);

    if let Ok(out) = Command::new(&git_bin)
        .current_dir(canonical_path)
        .args(["rev-parse", "--is-inside-work-tree", "--show-toplevel"])
        .output()
        .await
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.first() == Some(&"true") {
                is_git = true;
                if let Some(tl) = lines.get(1) {
                    root_path = tl.trim().to_string();
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
        let root_dir = Path::new(&root_path);
        if let Ok(b_out) = Command::new(&git_bin)
            .current_dir(root_dir)
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

        dirty_count = read_dirty_count(&git_bin, root_dir).await;
        is_dirty = dirty_count > 0;
        remote_url = read_git_remote(&git_bin, root_dir).await;
        if let Some(ref r_url) = remote_url {
            (github_host, github_owner, github_repo, github_avatar_url) =
                parse_github_remote(r_url);
        }

        if let Ok(d_out) = Command::new(&git_bin)
            .current_dir(root_dir)
            .args(["rev-parse", "--git-dir", "--git-common-dir"])
            .output()
            .await
        {
            if d_out.status.success() {
                let out_str = String::from_utf8_lossy(&d_out.stdout);
                let d_lines: Vec<&str> = out_str.lines().collect();
                if d_lines.len() >= 2 && d_lines[0] != d_lines[1] {
                    is_worktree = true;
                }
            }
        }

        worktrees = list_git_worktrees(&git_bin, &root_path).await;
    }

    let mut sub_repositories = Vec::new();
    if let Ok(mut dir_entries) = tokio::fs::read_dir(canonical_path).await {
        while let Ok(Some(entry)) = dir_entries.next_entry().await {
            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_dir() {
                    let sub_dir = entry.path();
                    if resolve_deep_git_paths(&sub_dir).await.is_some() {
                        let sub_name = entry.file_name().to_string_lossy().to_string();
                        let mut sub_branch = String::from("main");
                        if let Ok(sub_b_out) = Command::new(&git_bin)
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
                        let sub_dirty_count = read_dirty_count(&git_bin, &sub_dir).await;
                        let sub_remote = read_git_remote(&git_bin, &sub_dir).await;
                        let (sub_gh_host, sub_gh_owner, sub_gh_repo, sub_gh_avatar) = sub_remote
                            .as_deref()
                            .map(parse_github_remote)
                            .unwrap_or((None, None, None, None));
                        let sub_worktrees =
                            list_git_worktrees(&git_bin, &sub_dir.to_string_lossy()).await;

                        sub_repositories.push(SubRepositoryInfo {
                            name: sub_name,
                            path: sub_dir.to_string_lossy().to_string(),
                            is_git: true,
                            branch: sub_branch,
                            is_dirty: sub_dirty_count > 0,
                            dirty_count: sub_dirty_count,
                            remote_url: sub_remote,
                            github_host: sub_gh_host,
                            github_owner: sub_gh_owner,
                            github_repo: sub_gh_repo,
                            github_avatar_url: sub_gh_avatar,
                            worktrees: sub_worktrees,
                        });
                    }
                }
            }
        }
    }

    DetectedGitRepository {
        root_path,
        branch,
        is_git,
        is_worktree,
        is_dirty,
        dirty_count,
        remote_url,
        github_host,
        github_owner,
        github_repo,
        github_avatar_url,
        worktrees,
        sub_repositories,
    }
}

/// Checks if a repository currently has an in-progress `MERGE_HEAD` (unresolved conflict).
pub async fn has_merge_head(repo_path: &Path) -> bool {
    let Ok(output) = Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "--verify", "MERGE_HEAD"])
        .output()
        .await
    else {
        return false;
    };
    output.status.success()
}

