//! File I/O operations for workspace files subsystem.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use custos_domain::{
    DomainError, ExecutionWorkspace, WorkspaceFileContent, WorkspaceFileEntry, WorkspaceFileTree,
    WriteWorkspaceFileParams,
};

use super::{resolve_safe_path, DEFAULT_MAX_FILE_BYTES, HARD_MAX_FILE_BYTES, MAX_TREE_ENTRIES};

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

pub async fn get_file_tree(
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
    let mut queue: std::collections::VecDeque<(PathBuf, usize)> =
        std::collections::VecDeque::new();
    queue.push_back((start_dir.clone(), 0));

    while let Some((curr_dir, depth)) = queue.pop_front() {
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

            let is_readonly = metadata
                .as_ref()
                .map(|m| m.permissions().readonly())
                .unwrap_or(false);

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
                if depth + 1 < depth_limit {
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

        // Push subdirectories for breadth-first expansion
        for child in dir_children {
            queue.push_back(child);
        }
    }

    // Sort: directories first, then alphabetical path
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.path.cmp(&b.path)));

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

pub async fn read_file(
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
        DomainError::Validation(format!(
            "Failed to read metadata for '{}': {}",
            relative_path, e
        ))
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

    let file = fs::File::open(&target_path).await.map_err(|e| {
        DomainError::Validation(format!("Failed to open file '{}': {}", relative_path, e))
    })?;
    let mut bytes = Vec::with_capacity(limit.min(file_len).saturating_add(1));
    file.take(limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| {
            DomainError::Validation(format!("Failed to read file '{}': {}", relative_path, e))
        })?;

    // PDFs can have long ASCII headers without NUL bytes. Preserve their bytes,
    // as well as any content containing invalid UTF-8, instead of lossy text.
    let sample = &bytes[..bytes.len().min(8192)];
    let is_binary = bytes.starts_with(b"%PDF-")
        || sample.contains(&0)
        || std::str::from_utf8(&bytes)
            .err()
            .is_some_and(|error| error.error_len().is_some());

    if is_binary {
        use base64::Engine;
        let truncated = bytes.len() > limit;
        let read_slice = if truncated {
            &bytes[..limit]
        } else {
            &bytes[..]
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(read_slice);
        return Ok(WorkspaceFileContent::binary_with_content(
            workspace.id.clone(),
            relative_path,
            b64,
            file_len,
            truncated,
        ));
    }

    let truncated = bytes.len() > limit;
    let read_slice = if truncated {
        &bytes[..limit]
    } else {
        &bytes[..]
    };
    let complete_slice = match std::str::from_utf8(read_slice) {
        Err(error) if error.error_len().is_none() => &read_slice[..error.valid_up_to()],
        _ => read_slice,
    };
    let content_str = String::from_utf8_lossy(complete_slice).to_string();

    Ok(WorkspaceFileContent::text(
        workspace.id.clone(),
        relative_path,
        content_str,
        truncated,
    ))
}

pub async fn write_file(
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
