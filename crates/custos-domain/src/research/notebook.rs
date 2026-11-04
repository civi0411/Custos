//! Notebook & Python Kernel Domain Models
//!
//! Models authorized kernel execution, reproducible compute epochs,
//! cells, execution counts, outputs, and interrupts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotebookCellType {
    #[default]
    Code,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CellExecutionStatus {
    #[default]
    Idle,
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NotebookCell {
    pub id: String,
    pub session_id: String,
    pub cell_type: NotebookCellType,
    pub source: String,
    pub cell_index: u32,
    pub execution_count: Option<u32>,
    pub status: CellExecutionStatus,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub output_image: Option<String>,
    pub wall_ms: Option<u64>,
    pub epoch: u32,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_version: Option<u32>,
}

impl NotebookCell {
    /// Invalidates execution outputs and increments cell_version if source changes
    pub fn invalidate_output_on_edit(&mut self, new_source: &str, updated_at: i64) -> bool {
        if self.source == new_source {
            return false;
        }
        self.source = new_source.to_string();
        self.execution_count = None;
        self.status = CellExecutionStatus::Idle;
        self.stdout = None;
        self.stderr = None;
        self.output_image = None;
        self.wall_ms = None;
        self.cell_version = Some(self.cell_version.unwrap_or(0) + 1);
        self.updated_at = updated_at;
        true
    }

    /// Checks if the cell is stale relative to an expected version
    pub fn is_version_mismatch(&self, expected_version: Option<u32>) -> bool {
        match (self.cell_version, expected_version) {
            (Some(actual), Some(expected)) => actual != expected,
            _ => false,
        }
    }
}

/// Logical notebook document entity separate from session or kernel binding (RFC 003 §6 / SADE F06)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NotebookDocument {
    pub id: String,
    pub session_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Binding between a notebook document, conversation session, and kernel instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NotebookKernelBinding {
    pub notebook_id: String,
    pub session_id: String,
    pub kernel_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelStatus {
    #[default]
    Idle,
    Busy,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NotebookKernelState {
    pub session_id: String,
    pub epoch: u32,
    pub status: KernelStatus,
    pub python_version: String,
    pub execution_counter: u32,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExecuteCellParams {
    pub session_id: String,
    pub cell_id: String,
    pub code: String,
    pub workspace_id: Option<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExecuteCellResult {
    pub cell_id: String,
    pub execution_count: u32,
    pub status: CellExecutionStatus,
    pub stdout: String,
    pub stderr: String,
    pub output_image: Option<String>,
    pub wall_ms: u64,
    pub epoch: u32,
    #[serde(default)]
    pub execution_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notebook_cell_serde_roundtrip() {
        let cell = NotebookCell {
            id: "cell_101".into(),
            session_id: "sess_test".into(),
            cell_type: NotebookCellType::Code,
            source: "import math\nprint(math.sqrt(16))".into(),
            cell_index: 0,
            execution_count: Some(1),
            status: CellExecutionStatus::Success,
            stdout: Some("4.0\n".into()),
            stderr: None,
            output_image: None,
            wall_ms: Some(15),
            epoch: 1,
            updated_at: 1700000000,
            notebook_id: None,
            cell_version: None,
        };

        let json = serde_json::to_string(&cell).expect("serialize");
        let parsed: NotebookCell = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.id, "cell_101");
        assert_eq!(parsed.cell_type, NotebookCellType::Code);
        assert_eq!(parsed.execution_count, Some(1));
        assert_eq!(parsed.status, CellExecutionStatus::Success);
    }

    #[test]
    fn test_cell_invalidation_and_versioning() {
        let mut cell = NotebookCell {
            id: "c1".into(),
            session_id: "s1".into(),
            cell_type: NotebookCellType::Code,
            source: "x = 10".into(),
            cell_index: 0,
            execution_count: Some(1),
            status: CellExecutionStatus::Success,
            stdout: Some("10\n".into()),
            stderr: None,
            output_image: None,
            wall_ms: Some(5),
            epoch: 1,
            updated_at: 100,
            notebook_id: Some("nb_1".into()),
            cell_version: Some(1),
        };

        // Same source -> no invalidation
        assert!(!cell.invalidate_output_on_edit("x = 10", 110));
        assert_eq!(cell.cell_version, Some(1));
        assert_eq!(cell.status, CellExecutionStatus::Success);

        // Edit source -> invalidates output and bumps version
        assert!(cell.invalidate_output_on_edit("x = 20", 120));
        assert_eq!(cell.cell_version, Some(2));
        assert_eq!(cell.status, CellExecutionStatus::Idle);
        assert!(cell.stdout.is_none());
        assert!(cell.execution_count.is_none());
        assert_eq!(cell.updated_at, 120);

        // Version mismatch check
        assert!(cell.is_version_mismatch(Some(1)));
        assert!(!cell.is_version_mismatch(Some(2)));
        assert!(!cell.is_version_mismatch(None));
    }
}
