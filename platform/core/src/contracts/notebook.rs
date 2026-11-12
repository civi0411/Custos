//! Notebook Driver & Execution Contracts
//!
//! Physical execution driver and storage interface for interactive Python notebook kernels.
//! Coordinates subprocess spawning, bounded output capture, cell state, and process tree lifecycle.

use async_trait::async_trait;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    async fn read_plot_png_base64(&self, _work_dir: &std::path::Path) -> Option<String> {
        None
    }
}

pub use custos_domain::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, KernelStatus, NotebookCell,
    NotebookCellType, NotebookDocument, NotebookKernelBinding, NotebookKernelState,
};

/// Persistence Port for Interactive Notebook Cells and Execution States (RFC 003 §6).
#[async_trait]
pub trait NotebookStoragePort: Send + Sync {
    async fn list_notebook_cells(
        &self,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, DomainError>;
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
    async fn save_notebook_cell(
        &self,
        cell: &NotebookCell,
    ) -> Result<(), DomainError>;
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
