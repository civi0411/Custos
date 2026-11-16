//! Python Kernel Coordinator
//!
//! Manages authorized Python execution kernels with epoch tracking,
//! execution counters, interrupts, resets, and output persistence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use custos_domain::DomainError;
use custos_core::contracts::notebook::{
    ExecuteCellResult, KernelStatus, NotebookDriverPort, NotebookKernelState,
    NotebookStoragePort, NotebookWorkerSession,
};

pub(crate) mod execution;

#[cfg(test)]
mod tests;

pub(crate) struct ActiveKernel {
    pub(crate) epoch: u32,
    pub(crate) execution_counter: u32,
    pub(crate) status: KernelStatus,
    pub(crate) python_path: PathBuf,
    pub(crate) worker: Option<Arc<dyn NotebookWorkerSession>>,
    pub(crate) work_dir: Option<PathBuf>,
    pub(crate) recent_requests: HashMap<String, ExecuteCellResult>,
    pub(crate) active_lease_id: Option<String>,
}

#[derive(Clone)]
pub struct PythonKernelCoordinator {
    pub(crate) driver: Arc<dyn NotebookDriverPort>,
    pub(crate) kernels: Arc<RwLock<HashMap<String, ActiveKernel>>>,
    pub(crate) admission: Option<Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>>,
}

impl PythonKernelCoordinator {
    pub fn new(driver: Arc<dyn NotebookDriverPort>) -> Self {
        Self {
            driver,
            kernels: Arc::new(RwLock::new(HashMap::new())),
            admission: None,
        }
    }

    pub fn with_admission_coordinator(
        mut self,
        admission: Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>,
    ) -> Self {
        self.admission = Some(admission);
        self
    }

    pub fn kernel_key(session_id: &str, notebook_id: Option<&str>) -> String {
        match notebook_id {
            Some(nb) if !nb.trim().is_empty() => format!("{session_id}::{nb}"),
            _ => session_id.to_string(),
        }
    }

    pub async fn get_status(
        &self,
        session_id: &str,
        repo: Option<&(dyn NotebookStoragePort + Send + Sync)>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.get_status_scoped(session_id, None, repo).await
    }

    pub async fn get_status_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        repo: Option<&(dyn NotebookStoragePort + Send + Sync)>,
    ) -> Result<NotebookKernelState, DomainError> {
        let key = Self::kernel_key(session_id, notebook_id);
        let mut kernels = self.kernels.write().await;
        if let Some(active) = kernels.get(&key) {
            let ver = self.driver.get_python_version(&active.python_path);
            let state = NotebookKernelState {
                session_id: session_id.to_string(),
                notebook_id: notebook_id.map(|s| s.to_string()),
                epoch: active.epoch,
                status: active.status,
                python_version: ver,
                execution_counter: active.execution_counter,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                ..Default::default()
            };
            if let Some(r) = repo {
                let _ = r.save_notebook_kernel_state(&state).await;
            }
            Ok(state)
        } else if let Some(r) = repo {
            let mut state = r
                .get_notebook_kernel_state_scoped(session_id, notebook_id)
                .await?;
            if let Some(nb) = notebook_id {
                state.notebook_id = Some(nb.to_string());
            }
            // A restarted daemon has no Python memory from the recorded epoch.
            if state.execution_counter > 0 || state.status != KernelStatus::Idle {
                state.epoch += 1;
                state.execution_counter = 0;
                state.status = KernelStatus::Idle;
                state.updated_at = chrono::Utc::now().timestamp();
                r.save_notebook_kernel_state(&state).await?;
            }
            let py = self.driver.discover_python_path();
            kernels.insert(
                key,
                ActiveKernel {
                    epoch: state.epoch,
                    execution_counter: state.execution_counter,
                    status: state.status,
                    python_path: py,
                    worker: None,
                    work_dir: None,
                    recent_requests: HashMap::new(),
                    active_lease_id: None,
                },
            );
            Ok(state)
        } else {
            let py = self.driver.discover_python_path();
            let ver = self.driver.get_python_version(&py);
            let state = NotebookKernelState {
                session_id: session_id.to_string(),
                notebook_id: notebook_id.map(|s| s.to_string()),
                epoch: 1,
                status: KernelStatus::Idle,
                python_version: ver,
                execution_counter: 0,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
                ..Default::default()
            };
            kernels.insert(
                key,
                ActiveKernel {
                    epoch: 1,
                    execution_counter: 0,
                    status: KernelStatus::Idle,
                    python_path: py,
                    worker: None,
                    work_dir: None,
                    recent_requests: HashMap::new(),
                    active_lease_id: None,
                },
            );
            Ok(state)
        }
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<bool, DomainError> {
        self.interrupt_scoped(session_id, None).await
    }

    pub async fn interrupt_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<bool, DomainError> {
        let key = Self::kernel_key(session_id, notebook_id);
        let mut kernels = self.kernels.write().await;
        let mut lease_to_release = None;
        if let Some(k) = kernels.get_mut(&key) {
            lease_to_release = k.active_lease_id.take();
            if let Some(worker) = k.worker.take() {
                worker.interrupt();
                k.epoch += 1;
                k.execution_counter = 0;
                k.work_dir = None;
                k.status = KernelStatus::Interrupted;
                drop(kernels);
                if let Some(lease_id) = lease_to_release {
                    if let Some(ref adm) = self.admission {
                        let _ = adm.release_lease(&lease_id).await;
                    }
                }
                return Ok(true);
            }
        }
        drop(kernels);
        if let Some(lease_id) = lease_to_release {
            if let Some(ref adm) = self.admission {
                let _ = adm.release_lease(&lease_id).await;
            }
        }
        Ok(false)
    }

    /// Terminates all active Python worker processes and releases compute leases to reclaim memory/CPU.
    pub async fn shutdown_all(&self) {
        let drained: Vec<_> = {
            let mut kernels = self.kernels.write().await;
            kernels.drain().collect()
        };
        for (_, mut kernel) in drained {
            if let Some(worker) = kernel.worker.take() {
                worker.interrupt();
            }
            if let Some(lease_id) = kernel.active_lease_id.take() {
                if let Some(ref adm) = self.admission {
                    let _ = adm.release_lease(&lease_id).await;
                }
            }
        }
    }

    pub async fn reset(
        &self,
        session_id: &str,
        repo: Option<&(dyn NotebookStoragePort + Send + Sync)>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.reset_scoped(session_id, None, repo).await
    }

    pub async fn reset_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        repo: Option<&(dyn NotebookStoragePort + Send + Sync)>,
    ) -> Result<NotebookKernelState, DomainError> {
        let key = Self::kernel_key(session_id, notebook_id);
        let py = self.driver.discover_python_path();
        let ver = self.driver.get_python_version(&py);
        let mut kernels = self.kernels.write().await;
        let entry = kernels.entry(key).or_insert_with(|| ActiveKernel {
            epoch: 1,
            execution_counter: 0,
            status: KernelStatus::Idle,
            python_path: py.clone(),
            worker: None,
            work_dir: None,
            recent_requests: HashMap::new(),
            active_lease_id: None,
        });

        let lease_to_release = entry.active_lease_id.take();

        // Terminate active worker actor process if any
        if let Some(worker) = entry.worker.take() {
            worker.interrupt();
        }
        entry.work_dir = None;
        entry.recent_requests.clear();

        entry.epoch += 1;
        entry.execution_counter = 0;
        entry.status = KernelStatus::Idle;

        let state = NotebookKernelState {
            session_id: session_id.to_string(),
            notebook_id: notebook_id.map(|s| s.to_string()),
            epoch: entry.epoch,
            status: KernelStatus::Idle,
            python_version: ver,
            execution_counter: 0,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            ..Default::default()
        };
        drop(kernels);

        if let Some(lease_id) = lease_to_release {
            if let Some(ref adm) = self.admission {
                let _ = adm.release_lease(&lease_id).await;
            }
        }

        if let Some(r) = repo {
            let _ = r.save_notebook_kernel_state(&state).await;
        }

        Ok(state)
    }

    /// Bounded diagnostic probe for kernel accelerator availability (GPU-B §5)
    pub async fn probe_kernel_device_environment(
        &self,
        python_path: Option<&PathBuf>,
    ) -> Result<String, DomainError> {
        self.driver.probe_device_environment(python_path).await
    }
}
