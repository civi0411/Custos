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
}
