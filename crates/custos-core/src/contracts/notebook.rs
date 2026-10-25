//! Notebook Driver Port Contract
//!
//! Physical execution driver interface for Python notebook kernels.
//! Coordinates subprocess spawning, bounded output capture, and process tree lifecycle.

use async_trait::async_trait;
use custos_domain::DomainError;
use std::path::PathBuf;

/// Execution response from the underlying Python worker process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Spawns a new interactive Python worker session with bounded streaming channels.
    async fn spawn_worker(
        &self,
        python_path: PathBuf,
        work_dir: PathBuf,
    ) -> Result<Box<dyn NotebookWorkerSession>, DomainError>;
}
