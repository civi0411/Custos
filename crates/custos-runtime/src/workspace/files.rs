//! Workspace Files & Git Diff Runtime Subsystem
//!
//! Provides strictly scoped, fail-closed file tree browsing, file read/write,
//! and Git diff / stage / discard operations bounded to an `ExecutionWorkspace`.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use async_trait::async_trait;
use tokio::fs;
use tokio::process::Command;
use tracing::info;

use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceDiffEntry, WorkspaceDiffSummary,
    WorkspaceFileContent, WorkspaceFileDiff, WorkspaceFileEntry, WorkspaceFileTree,
    WriteWorkspaceFileParams,
};

/// Default maximum bytes to read from a single file (2 MB)
pub const DEFAULT_MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
/// Hard ceiling maximum file bytes (10 MB)
pub const HARD_MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
/// Maximum entries to return in a file tree before truncation
pub const MAX_TREE_ENTRIES: usize = 2000;

/// Standard directory names to ignore during tree traversal
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".cache",
    ".venv",
    "venv",
    "__pycache__",
    ".cargo",
    ".idea",
    ".vscode",
];

/// Resolves and validates a relative path strictly inside `base_dir`.
/// Fail-closed: returns `DomainError::Validation` if the path attempts traversal.
pub fn resolve_safe_path(base_dir: &Path, rel_path: &str) -> Result<PathBuf, DomainError> {
    let trimmed = rel_path.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err(DomainError::Validation("File path cannot be empty".into()));
    }

    let input_path = Path::new(&trimmed);

    // Reject absolute paths that don't start cleanly
    for comp in input_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(DomainError::Validation(
                    "Path traversal ('..') is strictly prohibited".into(),
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                // If input starts with root slash, treat as relative to base_dir
                continue;
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    // Strip leading slashes to safely join with base_dir
    let clean_rel = trimmed.trim_start_matches('/');
    let target = base_dir.join(clean_rel);

    // Verify canonical containment if base_dir exists
    if let Ok(canonical_base) = base_dir.canonicalize() {
        if target.exists() {
            if let Ok(canonical_target) = target.canonicalize() {
                if !canonical_target.starts_with(&canonical_base) {
                    return Err(DomainError::Validation(
                        "Access denied: resolved path escapes execution workspace boundary".into(),
                    ));
                }
            }
        } else if let Some(parent) = target.parent() {
            // Check closest existing ancestor
            let mut curr = parent;
            while !curr.exists() {
                if let Some(p) = curr.parent() {
                    curr = p;
                } else {
                    break;
                }
            }
            if let Ok(canonical_curr) = curr.canonicalize() {
                if !canonical_curr.starts_with(&canonical_base) {
                    return Err(DomainError::Validation(
                        "Access denied: target path escapes execution workspace boundary".into(),
                    ));
                }
            }
        }
    }

    Ok(target)
}

/// Runtime coordinator implementing `WorkspaceFilesPort`.
#[derive(Clone, Default)]
pub struct WorkspaceFilesCoordinator;

impl WorkspaceFilesCoordinator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkspaceFilesPort for WorkspaceFilesCoordinator {
    async fn get_file_tree(
        &self,
        workspace: &ExecutionWorkspace,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, DomainError> {
        let base_dir = Path::new(&workspace.path);
        if !base_dir.exists() {
            return Err(DomainError::NotFound {
                kind: "ExecutionWorkspacePath".into(),
                id: workspace.path.clone(),
            });
        }

        let start_dir = match relative_dir {
            Some(rel) if !rel.trim().is_empty() => resolve_safe_path(base_dir, rel)?,
            _ => base_dir.to_path_buf(),
        };

        let depth_limit = max_depth.unwrap_or(6).clamp(1, 12);
        let mut entries = Vec::new();
        let mut total_files = 0;
        let mut total_dirs = 0;
        let mut truncated = false;

        let ignored_set: HashSet<&str> = IGNORED_DIRS.iter().copied().collect();

        // Breadth-first traversal with bounded capacity
        let mut queue: Vec<(PathBuf, usize)> = vec![(start_dir.clone(), 0)];

        while let Some((curr_dir, depth)) = queue.pop() {
            if entries.len() >= MAX_TREE_ENTRIES {
                truncated = true;
                break;
            }

            let mut read_dir = match fs::read_dir(&curr_dir).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            let mut dir_children = Vec::new();

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.starts_with('.') && file_name != ".env" && file_name != ".gitignore" {
                    // Skip hidden dirs/files like .git, .DS_Store
                    continue;
                }

                if ignored_set.contains(file_name.as_str()) {
                    continue;
                }

                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                let is_dir = file_type.is_dir();
                let metadata = entry.metadata().await.ok();
                let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_at = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64);

                let is_readonly = metadata.as_ref().map(|m| m.permissions().readonly()).unwrap_or(false);

                // Compute path relative to workspace root
                let rel_path = match path.strip_prefix(base_dir) {
                    Ok(p) => p.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };

                let mut ws_entry = WorkspaceFileEntry::new(rel_path, file_name, is_dir, size_bytes);
                if let Some(m) = modified_at {
                    ws_entry = ws_entry.with_modified_at(m);
                }
                ws_entry = ws_entry.with_readonly(is_readonly);

                if is_dir {
                    total_dirs += 1;
                    if depth < depth_limit {
                        dir_children.push((path, depth + 1));
                    }
                } else {
                    total_files += 1;
                }

                entries.push(ws_entry);

                if entries.len() >= MAX_TREE_ENTRIES {
                    truncated = true;
                    break;
                }
            }

            // Push subdirectories
            for child in dir_children {
                queue.push(child);
            }
        }

        // Sort: directories first, then alphabetical path
        entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then_with(|| a.path.cmp(&b.path))
        });

        Ok(WorkspaceFileTree {
            workspace_id: workspace.id.clone(),
            root_path: workspace.path.clone(),
            relative_dir: relative_dir.map(String::from),
            entries,
            total_files,
            total_dirs,
            truncated,
        })
    }

    async fn read_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, DomainError> {
        let base_dir = Path::new(&workspace.path);
        let target_path = resolve_safe_path(base_dir, relative_path)?;

        if !target_path.exists() {
            return Err(DomainError::NotFound {
                kind: "WorkspaceFile".into(),
                id: relative_path.to_string(),
            });
        }

        let metadata = fs::metadata(&target_path).await.map_err(|e| {
            DomainError::Validation(format!("Failed to read metadata for '{}': {}", relative_path, e))
        })?;

        if metadata.is_dir() {
            return Err(DomainError::Validation(format!(
                "Path '{}' is a directory, not a file",
                relative_path
            )));
        }

        let file_len = metadata.len() as usize;
        let limit = max_bytes
            .unwrap_or(DEFAULT_MAX_FILE_BYTES)
            .min(HARD_MAX_FILE_BYTES);

        let bytes = fs::read(&target_path).await.map_err(|e| {
            DomainError::Validation(format!("Failed to read file '{}': {}", relative_path, e))
        })?;

        // Check for binary content (null bytes in first 1024 bytes)
        let sample_len = bytes.len().min(1024);
        let is_binary = bytes[..sample_len].contains(&0);

        if is_binary {
            return Ok(WorkspaceFileContent::binary(
                workspace.id.clone(),
                relative_path,
                file_len,
            ));
        }

        let truncated = bytes.len() > limit;
        let read_slice = if truncated { &bytes[..limit] } else { &bytes[..] };
        let content_str = String::from_utf8_lossy(read_slice).to_string();

        Ok(WorkspaceFileContent::text(
            workspace.id.clone(),
            relative_path,
            content_str,
            truncated,
        ))
    }

    async fn write_file(
        &self,
        workspace: &ExecutionWorkspace,
        params: WriteWorkspaceFileParams,
    ) -> Result<WorkspaceFileContent, DomainError> {
        let base_dir = Path::new(&workspace.path);
        let target_path = resolve_safe_path(base_dir, &params.path)?;

        if target_path.exists() && !params.overwrite {
            return Err(DomainError::Conflict(format!(
                "File '{}' already exists and overwrite is false",
                params.path
            )));
        }

        if params.create_parents {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    DomainError::Validation(format!(
                        "Failed to create parent directory for '{}': {}",
                        params.path, e
                    ))
                })?;
            }
        }

        fs::write(&target_path, params.content.as_bytes())
            .await
            .map_err(|e| {
                DomainError::Validation(format!("Failed to write file '{}': {}", params.path, e))
            })?;

        Ok(WorkspaceFileContent::text(
            workspace.id.clone(),
            params.path,
            params.content,
            false,
        ))
    }

    async fn get_diff(
        &self,
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
                (parts.next().map(String::from), parts.next().unwrap_or(rest).to_string())
            } else {
                (None, rest.to_string())
            };

            // Staged entry if index status is not ' ' or '?'
            if index_status != ' ' && index_status != '?' {
                let mut entry = WorkspaceDiffEntry::new(
                    &file_path,
                    index_status.to_string(),
                    true,
                    0,
                    0,
                );
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
                let mut entry = WorkspaceDiffEntry::new(
                    &file_path,
                    status_code,
                    false,
                    0,
                    0,
                );
                entry.old_path = old_path;
                entries.push(entry);
            }
        }

        // 2. Fetch numstat for unstaged & staged to compute line additions/deletions
        let numstat_unstaged = run_git(base_dir, &["diff", "--numstat"]).await.unwrap_or_default();
        let numstat_staged = run_git(base_dir, &["diff", "--cached", "--numstat"]).await.unwrap_or_default();

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
        let diff_staged = run_git(base_dir, &["diff", "--cached"]).await.unwrap_or_default();
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

    async fn get_file_diff(
        &self,
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

    async fn stage_file(
        &self,
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

    async fn unstage_file(
        &self,
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
            return Err(DomainError::Validation(format!("git restore --staged failed: {}", err)));
        }

        info!(workspace_id = %workspace.id, path = %relative_path, "Unstaged file successfully");
        Ok(())
    }

    async fn discard_file(
        &self,
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
            return Err(DomainError::Validation(format!("git restore failed: {}", err)));
        }

        info!(workspace_id = %workspace.id, path = %relative_path, "Discarded file changes successfully");
        Ok(())
    }
}

async fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .current_dir(path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .await
        .map(|out| out.status.success())
        .unwrap_or(false)
}

async fn run_git(dir: &Path, args: &[&str]) -> Result<String, DomainError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{WorkspaceId, WorkspaceKind};

    #[tokio::test]
    async fn test_resolve_safe_path_rejects_traversal() {
        let temp_dir = std::env::temp_dir().join("custos_safe_path_test");
        let _ = fs::create_dir_all(&temp_dir).await;

        let err = resolve_safe_path(&temp_dir, "../outside.txt").unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        let err = resolve_safe_path(&temp_dir, "foo/../../outside.txt").unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        let ok = resolve_safe_path(&temp_dir, "src/main.rs").unwrap();
        assert_eq!(ok, temp_dir.join("src/main.rs"));

        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_file_read_write_lifecycle() {
        let test_dir = std::env::temp_dir().join(format!("custos_ws_file_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&test_dir).await;

        let ws = ExecutionWorkspace::new(
            WorkspaceId::generate(),
            "test_ws".to_string(),
            WorkspaceKind::Folder { path: test_dir.to_string_lossy().to_string() },
            test_dir.to_string_lossy().to_string(),
        );

        let coord = WorkspaceFilesCoordinator::new();

        // 1. Write file with parent creation
        let write_params = WriteWorkspaceFileParams::new(
            ws.id.clone(),
            "nested/dir/hello.txt",
            "Hello Custos Workspace File API!\nLine 2\n",
        );
        let written = coord.write_file(&ws, write_params).await.expect("write file");
        assert_eq!(written.path, "nested/dir/hello.txt");
        assert_eq!(written.line_count, 2);
        assert!(!written.is_binary);

        // 2. Read file back
        let read = coord.read_file(&ws, "nested/dir/hello.txt", None).await.expect("read file");
        assert_eq!(read.content, "Hello Custos Workspace File API!\nLine 2\n");
        assert_eq!(read.line_count, 2);

        // 3. Get file tree
        let tree = coord.get_file_tree(&ws, None, None).await.expect("file tree");
        assert!(tree.total_files >= 1);
        assert!(tree.entries.iter().any(|e| e.path == "nested/dir/hello.txt"));

        let _ = fs::remove_dir_all(&test_dir).await;
    }
}
