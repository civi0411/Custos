//! Git operations for workspace files subsystem.

use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use tracing::info;

use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceDiffEntry, WorkspaceDiffSummary, WorkspaceFileDiff,
};

use super::resolve_safe_path;

pub async fn get_diff(
    workspace: &ExecutionWorkspace,
    _staged: Option<bool>,
) -> Result<WorkspaceDiffSummary, DomainError> {
    let base_dir = Path::new(&workspace.path);
    if !base_dir.exists() {
        return Err(DomainError::NotFound {
            kind: "ExecutionWorkspacePath".into(),
            id: workspace.path.clone(),
        });
    }

    // Check if workspace is a git repository
    let is_git = is_git_repo(base_dir).await;
    if !is_git {
        return Ok(WorkspaceDiffSummary::clean(workspace.id.clone()));
    }

    // 1. Git status --porcelain=v1
    let status_out = Command::new("git")
        .current_dir(base_dir)
        .arg("status")
        .arg("--porcelain=v1")
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git status failed: {}", e)))?;

    let status_str = String::from_utf8_lossy(&status_out.stdout);
    let mut entries = Vec::new();

    for line in status_str.lines() {
        if line.len() < 3 {
            continue;
        }
        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let rest = line[3..].trim();

        let (old_path, file_path) = if rest.contains(" -> ") {
            let mut parts = rest.split(" -> ");
            (
                parts.next().map(String::from),
                parts.next().unwrap_or(rest).to_string(),
            )
        } else {
            (None, rest.to_string())
        };

        // Staged entry if index status is not ' ' or '?'
        if index_status != ' ' && index_status != '?' {
            let mut entry =
                WorkspaceDiffEntry::new(&file_path, index_status.to_string(), true, 0, 0);
            entry.old_path = old_path.clone();
            entries.push(entry);
        }

        // Unstaged entry
        if worktree_status != ' ' {
            let status_code = if index_status == '?' && worktree_status == '?' {
                "??".to_string()
            } else {
                worktree_status.to_string()
            };
            let mut entry = WorkspaceDiffEntry::new(&file_path, status_code, false, 0, 0);
            entry.old_path = old_path;
            entries.push(entry);
        }
    }

    // 2. Fetch numstat for unstaged & staged to compute line additions/deletions
    let numstat_unstaged = run_git(base_dir, &["diff", "--numstat"])
        .await
        .unwrap_or_default();
    let numstat_staged = run_git(base_dir, &["diff", "--cached", "--numstat"])
        .await
        .unwrap_or_default();

    let mut total_additions = 0;
    let mut total_deletions = 0;

    for line in numstat_unstaged.lines().chain(numstat_staged.lines()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let adds = parts[0].parse::<usize>().unwrap_or(0);
            let dels = parts[1].parse::<usize>().unwrap_or(0);
            let path = parts[2];

            total_additions += adds;
            total_deletions += dels;

            for entry in entries.iter_mut() {
                if entry.path == path {
                    entry.additions = adds;
                    entry.deletions = dels;
                }
            }
        }
    }

    // 3. Full raw diff (combining staged and unstaged unified diffs)
    let diff_unstaged = run_git(base_dir, &["diff"]).await.unwrap_or_default();
    let diff_staged = run_git(base_dir, &["diff", "--cached"])
        .await
        .unwrap_or_default();
    let raw_diff = if diff_staged.is_empty() {
        diff_unstaged
    } else if diff_unstaged.is_empty() {
        diff_staged
    } else {
        format!("{}\n{}", diff_staged, diff_unstaged)
    };

    // 4. Branch & HEAD commit
    let branch = run_git(base_dir, &["branch", "--show-current"])
        .await
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let head_hash = run_git(base_dir, &["rev-parse", "HEAD"])
        .await
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let is_clean = entries.is_empty();

    Ok(WorkspaceDiffSummary {
        workspace_id: workspace.id.clone(),
        head_hash,
        base_hash: workspace.base_commit_hash.clone(),
        branch,
        files: entries,
        raw_diff,
        total_additions,
        total_deletions,
        is_clean,
    })
}

pub async fn get_file_diff(
    workspace: &ExecutionWorkspace,
    relative_path: &str,
    staged: Option<bool>,
) -> Result<WorkspaceFileDiff, DomainError> {
    let base_dir = Path::new(&workspace.path);
    let _ = resolve_safe_path(base_dir, relative_path)?;

    let is_staged = staged.unwrap_or(false);
    let mut args = vec!["diff"];
    if is_staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(relative_path);

    let diff_text = run_git(base_dir, &args).await.unwrap_or_default();

    Ok(WorkspaceFileDiff {
        workspace_id: workspace.id.clone(),
        path: relative_path.to_string(),
        diff: diff_text,
        is_staged,
    })
}

pub async fn stage_file(
    workspace: &ExecutionWorkspace,
    relative_path: &str,
) -> Result<(), DomainError> {
    let base_dir = Path::new(&workspace.path);
    let _ = resolve_safe_path(base_dir, relative_path)?;

    let out = Command::new("git")
        .current_dir(base_dir)
        .arg("add")
        .arg("--")
        .arg(relative_path)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git add failed: {}", e)))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(DomainError::Validation(format!("git add failed: {}", err)));
    }

    info!(workspace_id = %workspace.id, path = %relative_path, "Staged file successfully");
    Ok(())
}

pub async fn unstage_file(
    workspace: &ExecutionWorkspace,
    relative_path: &str,
) -> Result<(), DomainError> {
    let base_dir = Path::new(&workspace.path);
    let _ = resolve_safe_path(base_dir, relative_path)?;

    let out = Command::new("git")
        .current_dir(base_dir)
        .arg("restore")
        .arg("--staged")
        .arg("--")
        .arg(relative_path)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git restore --staged failed: {}", e)))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(DomainError::Validation(format!(
            "git restore --staged failed: {}",
            err
        )));
    }

    info!(workspace_id = %workspace.id, path = %relative_path, "Unstaged file successfully");
    Ok(())
}

pub async fn discard_file(
    workspace: &ExecutionWorkspace,
    relative_path: &str,
) -> Result<(), DomainError> {
    let base_dir = Path::new(&workspace.path);
    let target_path = resolve_safe_path(base_dir, relative_path)?;

    // Check if untracked in git status
    let status_out = Command::new("git")
        .current_dir(base_dir)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--")
        .arg(relative_path)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git status failed: {}", e)))?;

    let status_str = String::from_utf8_lossy(&status_out.stdout);
    let is_untracked = status_str.starts_with("??");

    if is_untracked {
        if target_path.is_dir() {
            fs::remove_dir_all(&target_path).await.map_err(|e| {
                DomainError::Validation(format!("Failed to remove untracked directory: {}", e))
            })?;
        } else if target_path.exists() {
            fs::remove_file(&target_path).await.map_err(|e| {
                DomainError::Validation(format!("Failed to remove untracked file: {}", e))
            })?;
        }
        info!(workspace_id = %workspace.id, path = %relative_path, "Removed untracked file");
        return Ok(());
    }

    // Tracked file: git restore
    let out = Command::new("git")
        .current_dir(base_dir)
        .arg("restore")
        .arg("--")
        .arg(relative_path)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git restore failed: {}", e)))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(DomainError::Validation(format!(
            "git restore failed: {}",
            err
        )));
    }

    info!(workspace_id = %workspace.id, path = %relative_path, "Discarded file changes successfully");
    Ok(())
}

pub async fn commit(
    workspace: &ExecutionWorkspace,
    message: &str,
) -> Result<String, DomainError> {
    let base_dir = Path::new(&workspace.path);
    if message.trim().is_empty() {
        return Err(DomainError::Validation(
            "Commit message cannot be empty".into(),
        ));
    }

    let staged = Command::new("git")
        .current_dir(base_dir)
        .args(["diff", "--cached", "--quiet"])
        .status()
        .await
        .map_err(|error| {
            DomainError::Validation(format!("Cannot inspect staged changes: {error}"))
        })?;
    match staged.code() {
        Some(1) => {}
        Some(0) => {
            return Err(DomainError::Validation(
                "No staged changes. Stage the intended files before committing.".into(),
            ))
        }
        _ => {
            return Err(DomainError::Validation(
                "Cannot inspect staged changes".into(),
            ))
        }
    }

    let out = Command::new("git")
        .current_dir(base_dir)
        .arg("commit")
        .arg("-m")
        .arg(message)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git commit failed: {}", e)))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(DomainError::Validation(format!(
            "git commit failed: {}",
            err
        )));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    info!(workspace_id = %workspace.id, "Committed git changes successfully: {}", stdout);
    Ok(stdout)
}

pub async fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .current_dir(path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .await
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub async fn run_git(dir: &Path, args: &[&str]) -> Result<String, DomainError> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .await
        .map_err(|e| DomainError::Validation(format!("git command failed: {}", e)))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(DomainError::Validation(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}
