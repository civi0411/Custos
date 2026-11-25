//! Stateful Notebook & Kernel Domain Models
//!
//! Pure, deterministic entities shared across workbenches (Research, Quant)
//! for interactive notebook cells, document bindings, and kernel execution states.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelStatus {
    #[default]
    Idle,
    Busy,
    Interrupted,
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
    fn edit_invalidates_output_and_tracks_version() {
        let mut cell = NotebookCell {
            id: "cell_1".into(),
            session_id: "sess_1".into(),
            source: "x = 1".into(),
            status: CellExecutionStatus::Success,
            execution_count: Some(1),
            stdout: Some("1".into()),
            cell_version: Some(2),
            ..Default::default()
        };
        assert!(!cell.invalidate_output_on_edit("x = 1", 10));
        assert!(cell.invalidate_output_on_edit("x = 2", 20));
        assert_eq!(cell.status, CellExecutionStatus::Idle);
        assert_eq!(cell.execution_count, None);
        assert_eq!(cell.stdout, None);
        assert_eq!(cell.cell_version, Some(3));
        assert!(cell.is_version_mismatch(Some(2)));
        assert!(!cell.is_version_mismatch(Some(3)));
    }
}

use crate::identity::error::DomainError;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Execution response from the underlying Python worker process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookWorkerResponse {
    pub status: String,
    pub stdout: String,
    pub stderr: String,
}

/// Abstract interactive session with an active Python kernel process.
#[async_trait]
pub trait NotebookWorkerSession: Send + Sync {
    /// Executes code within the running Python kernel and returns bounded output.
    async fn execute(
        &self,
        code: String,
        cwd: Option<String>,
    ) -> Result<NotebookWorkerResponse, DomainError>;

    /// Interrupts the worker execution and kills its active process tree.
    fn interrupt(&self);
}

/// Physical driver port for Python notebook worker process management.
#[async_trait]
pub trait NotebookDriverPort: Send + Sync {
    /// Discovers the best Python executable path on the host.
    fn discover_python_path(&self) -> PathBuf {
        PathBuf::from("python3")
    }

    /// Queries the version string for a discovered Python binary.
    fn get_python_version(&self, _python_path: &PathBuf) -> String {
        "Python 3".to_string()
    }

    /// Probes the Python environment for accelerator support (CUDA/MPS).
    async fn probe_device_environment(
        &self,
        _python_path: Option<&PathBuf>,
    ) -> Result<String, DomainError> {
        Ok("torch_unavailable".to_string())
    }

    /// Spawns a new interactive Python worker session with bounded streaming channels.
    async fn spawn_worker(
        &self,
        python_path: PathBuf,
        work_dir: PathBuf,
    ) -> Result<Box<dyn NotebookWorkerSession>, DomainError>;

    /// Reads an optional rendered `_plot.png` artifact from `work_dir` as a data URL.
    async fn read_plot_png_base64(&self, _work_dir: &Path) -> Option<String> {
        None
    }
}

/// Persistence Port for Interactive Notebook Cells and Execution States (RFC 003 §6).
#[async_trait]
pub trait NotebookStoragePort: Send + Sync {
    async fn list_notebook_cells(&self, session_id: &str)
        -> Result<Vec<NotebookCell>, DomainError>;
    async fn list_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        let cells = self.list_notebook_cells(session_id).await?;
        match notebook_id {
            Some(nb) if !nb.trim().is_empty() => Ok(cells
                .into_iter()
                .filter(|c| c.notebook_id.as_deref() == Some(nb))
                .collect()),
            _ => Ok(cells),
        }
    }
    async fn replace_notebook_cells(
        &self,
        session_id: &str,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError>;
    async fn replace_notebook_cells_scoped(
        &self,
        session_id: &str,
        _notebook_id: Option<&str>,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.replace_notebook_cells(session_id, cells).await
    }
    async fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError>;
    async fn get_notebook_kernel_state(
        &self,
        session_id: &str,
    ) -> Result<NotebookKernelState, DomainError>;
    async fn get_notebook_kernel_state_scoped(
        &self,
        session_id: &str,
        _notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.get_notebook_kernel_state(session_id).await
    }
    async fn save_notebook_kernel_state(
        &self,
        state: &NotebookKernelState,
    ) -> Result<(), DomainError>;
}
