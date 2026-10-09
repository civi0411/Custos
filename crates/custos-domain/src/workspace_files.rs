//! Workspace Files and Git Diffs Domain Models
//!
//! Models safe file tree exploration, file read/write buffers, and Git diff inspection
//! strictly scoped to an `ExecutionWorkspace`.

use serde::{Deserialize, Serialize};
use crate::WorkspaceId;

/// Represents a single file or directory inside an execution workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    /// Relative path within the workspace (forward-slash delimited, e.g. "src/lib.rs")
    pub path: String,
    /// Base name of the file or directory
    pub name: String,
    /// Whether this entry is a directory
    pub is_dir: bool,
    /// Size in bytes (0 for directories)
    pub size_bytes: u64,
    /// Last modification timestamp in epoch milliseconds, if available
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    /// Whether this file is marked read-only on disk
    pub is_readonly: bool,
}

impl WorkspaceFileEntry {
    pub fn new(path: impl Into<String>, name: impl Into<String>, is_dir: bool, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            is_dir,
            size_bytes,
            modified_at: None,
            is_readonly: false,
        }
    }

    pub fn with_modified_at(mut self, modified_at: i64) -> Self {
        self.modified_at = Some(modified_at);
        self
    }

    pub fn with_readonly(mut self, is_readonly: bool) -> Self {
        self.is_readonly = is_readonly;
        self
    }
}

/// Hierarchical or flattened file tree representation for an execution workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileTree {
    pub workspace_id: WorkspaceId,
    pub root_path: String,
    pub relative_dir: Option<String>,
    pub entries: Vec<WorkspaceFileEntry>,
    pub total_files: usize,
    pub total_dirs: usize,
    pub truncated: bool,
}

impl WorkspaceFileTree {
    pub fn empty(workspace_id: WorkspaceId, root_path: impl Into<String>) -> Self {
        Self {
            workspace_id,
            root_path: root_path.into(),
            relative_dir: None,
            entries: Vec::new(),
            total_files: 0,
            total_dirs: 0,
            truncated: false,
        }
    }
}

/// Content and metadata for a specific file read from an execution workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileContent {
    pub workspace_id: WorkspaceId,
    pub path: String,
    pub content: String,
    pub is_binary: bool,
    pub size_bytes: usize,
    pub truncated: bool,
    pub line_count: usize,
}

impl WorkspaceFileContent {
    pub fn text(
        workspace_id: WorkspaceId,
        path: impl Into<String>,
        content: impl Into<String>,
        truncated: bool,
    ) -> Self {
        let content_str = content.into();
        let line_count = if content_str.is_empty() {
            0
        } else {
            content_str.lines().count()
        };
        let size_bytes = content_str.len();

        Self {
            workspace_id,
            path: path.into(),
            content: content_str,
            is_binary: false,
            size_bytes,
            truncated,
            line_count,
        }
    }

    pub fn binary(
        workspace_id: WorkspaceId,
        path: impl Into<String>,
        size_bytes: usize,
    ) -> Self {
        Self {
            workspace_id,
            path: path.into(),
            content: String::new(),
            is_binary: true,
            size_bytes,
            truncated: false,
            line_count: 0,
        }
    }
}

/// Parameters for writing or modifying a file inside an execution workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteWorkspaceFileParams {
    pub workspace_id: WorkspaceId,
    pub path: String,
    pub content: String,
    #[serde(default = "default_true")]
    pub create_parents: bool,
    #[serde(default = "default_true")]
    pub overwrite: bool,
}

fn default_true() -> bool {
    true
}

impl WriteWorkspaceFileParams {
    pub fn new(workspace_id: WorkspaceId, path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            workspace_id,
            path: path.into(),
            content: content.into(),
            create_parents: true,
            overwrite: true,
        }
    }
}

/// Individual file entry in a Git diff summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiffEntry {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// Status code: "M" (modified), "A" (added), "D" (deleted), "R" (renamed), "??" (untracked)
    pub status: String,
    pub is_staged: bool,
    pub additions: usize,
    pub deletions: usize,
}

impl WorkspaceDiffEntry {
    pub fn new(
        path: impl Into<String>,
        status: impl Into<String>,
        is_staged: bool,
        additions: usize,
        deletions: usize,
    ) -> Self {
        Self {
            path: path.into(),
            old_path: None,
            status: status.into(),
            is_staged,
            additions,
            deletions,
        }
    }
}

/// Full Git diff summary and patch inspection scoped to an execution workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiffSummary {
    pub workspace_id: WorkspaceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub files: Vec<WorkspaceDiffEntry>,
    pub raw_diff: String,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub is_clean: bool,
}

impl WorkspaceDiffSummary {
    pub fn clean(workspace_id: WorkspaceId) -> Self {
        Self {
            workspace_id,
            head_hash: None,
            base_hash: None,
            branch: None,
            files: Vec::new(),
            raw_diff: String::new(),
            total_additions: 0,
            total_deletions: 0,
            is_clean: true,
        }
    }
}

/// Diff for a specific single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileDiff {
    pub workspace_id: WorkspaceId,
    pub path: String,
    pub diff: String,
    pub is_staged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_file_content_text_properties() {
        let ws_id = WorkspaceId::generate();
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let file_content = WorkspaceFileContent::text(ws_id.clone(), "src/main.rs", content, false);

        assert_eq!(file_content.workspace_id, ws_id);
        assert_eq!(file_content.path, "src/main.rs");
        assert_eq!(file_content.line_count, 3);
        assert!(!file_content.is_binary);
        assert!(!file_content.truncated);
        assert_eq!(file_content.size_bytes, content.len());
    }

    #[test]
    fn test_workspace_diff_summary_clean() {
        let ws_id = WorkspaceId::generate();
        let summary = WorkspaceDiffSummary::clean(ws_id);
        assert!(summary.is_clean);
        assert_eq!(summary.total_additions, 0);
        assert_eq!(summary.total_deletions, 0);
        assert!(summary.files.is_empty());
    }

    #[test]
    fn test_write_params_serialization() {
        let ws_id = WorkspaceId::generate();
        let params = WriteWorkspaceFileParams::new(ws_id.clone(), "test.txt", "hello");
        let json = serde_json::to_string(&params).expect("serialize");
        let deserialized: WriteWorkspaceFileParams = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.path, "test.txt");
        assert!(deserialized.create_parents);
        assert!(deserialized.overwrite);
    }
}
