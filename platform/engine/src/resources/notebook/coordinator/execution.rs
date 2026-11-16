//! Notebook cell execution logic.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use custos_domain::DomainError;
use custos_core::contracts::notebook::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, KernelStatus, NotebookCell,
    NotebookCellType, NotebookStoragePort, NotebookWorkerSession,
};

use super::{ActiveKernel, PythonKernelCoordinator};

impl PythonKernelCoordinator {
    pub async fn execute_cell(
        &self,
        params: ExecuteCellParams,
        repo: Option<&(dyn NotebookStoragePort + Send + Sync)>,
    ) -> Result<ExecuteCellResult, DomainError> {
        let key = Self::kernel_key(&params.session_id, params.notebook_id.as_deref());

        // C3: Duplicate request ID does not run twice; return cached result immediately
        if let Some(ref req_id) = params.execution_request_id {
            let kernels = self.kernels.read().await;
            if let Some(k) = kernels.get(&key) {
                if let Some(cached) = k.recent_requests.get(req_id) {
                    tracing::info!(%req_id, "Duplicate execution request id received; returning cached result");
                    return Ok(cached.clone());
                }
            }
        }

        // Validate cell version before execution if specified and saved
        if let Some(r) = repo {
            let cells = r
                .list_notebook_cells_scoped(&params.session_id, params.notebook_id.as_deref())
                .await?;
            if let Some(saved) = cells.iter().find(|c| c.id == params.cell_id) {
                if let Some(expected_v) = params.cell_version {
                    if let Some(actual_v) = saved.cell_version {
                        if expected_v != actual_v {
                            return Err(DomainError::Validation(format!(
                                "Cell version mismatch: requested {expected_v}, but saved cell has {actual_v}"
                            )));
                        }
                    }
                }
            }
        }

        // SADE GPU-A / GPU-B: Resource admission check
        let mut acquired_lease = None;
        if let Some(ref dev_id) = params.device_id {
            if let Some(ref adm) = self.admission {
                let req = custos_domain::WorkloadAdmissionRequest {
                    workload_kind: custos_domain::ComputeWorkloadKind::NotebookKernel,
                    owner_id: key.clone(),
                    preferred_device_id: Some(dev_id.clone()),
                    require_exclusive: false,
                    required_memory_mb: None,
                    lane: Some(custos_domain::ExecutionLane::Interactive),
                };
                match adm.admit_workload(&req).await? {
                    custos_domain::WorkloadAdmissionVerdict::Admitted { lease } => {
                        acquired_lease = Some(lease.lease_id);
                    }
                    custos_domain::WorkloadAdmissionVerdict::Queued { reason, .. } => {
                        return Err(DomainError::Conflict(format!(
                            "Compute device '{dev_id}' busy: {reason}"
                        )));
                    }
                    custos_domain::WorkloadAdmissionVerdict::Rejected { reason } => {
                        return Err(DomainError::Validation(format!(
                            "Compute admission for device '{dev_id}' rejected: {reason}"
                        )));
                    }
                }
            }
        }

        let python_path = self.driver.discover_python_path();
        let work_dir = if let Some(ref cwd) = params.cwd {
            PathBuf::from(cwd)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let (epoch, execution_count, worker_session) = {
            let mut kernels = self.kernels.write().await;
            let kernel = kernels.entry(key.clone()).or_insert_with(|| ActiveKernel {
                epoch: 1,
                execution_counter: 0,
                status: KernelStatus::Idle,
                python_path: python_path.clone(),
                worker: None,
                work_dir: None,
                recent_requests: HashMap::new(),
                active_lease_id: None,
            });

            if let Some(lease_id) = acquired_lease {
                kernel.active_lease_id = Some(lease_id);
            }

            if kernel.status == KernelStatus::Busy {
                return Err(DomainError::Validation(
                    "A cell is already running in this session".into(),
                ));
            }
            if kernel
                .work_dir
                .as_ref()
                .is_some_and(|previous| previous != &work_dir)
            {
                return Err(DomainError::Validation(
                    "Reset this session's Python worker before changing execution workspace".into(),
                ));
            }

            let session = match &kernel.worker {
                Some(w) => w.clone(),
                None => {
                    let w = self
                        .driver
                        .spawn_worker(python_path.clone(), work_dir.clone())
                        .await?;
                    let session_arc: Arc<dyn NotebookWorkerSession> = Arc::from(w);
                    kernel.worker = Some(session_arc.clone());
                    session_arc
                }
            };

            kernel.work_dir = Some(work_dir.clone());
            kernel.execution_counter += 1;
            kernel.status = KernelStatus::Busy;
            (kernel.epoch, kernel.execution_counter, session)
        };

        let start = Instant::now();
        let worker_res = worker_session
            .execute(params.code.clone(), params.cwd.clone())
            .await;
        let wall_ms = start.elapsed().as_millis() as u64;

        {
            let mut kernels = self.kernels.write().await;
            if let Some(k) = kernels.get_mut(&key) {
                if k.epoch != epoch {
                    return Err(DomainError::Validation(
                        "Execution was interrupted or reset; its output is stale".into(),
                    ));
                }
                k.status = KernelStatus::Idle;
            }
        }

        let resp = worker_res?;

        let status = if resp.status == "success" {
            CellExecutionStatus::Success
        } else {
            CellExecutionStatus::Error
        };

        const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
        let mut stdout = resp.stdout;
        if stdout.len() > MAX_OUTPUT_BYTES {
            let mut end = MAX_OUTPUT_BYTES;
            while !stdout.is_char_boundary(end) {
                end = end.saturating_sub(1);
            }
            stdout.truncate(end);
            stdout.push_str("\n[Output truncated: exceeded 4MB limit]");
        }

        let mut stderr = resp.stderr;
        if stderr.len() > MAX_OUTPUT_BYTES {
            let mut end = MAX_OUTPUT_BYTES;
            while !stderr.is_char_boundary(end) {
                end = end.saturating_sub(1);
            }
            stderr.truncate(end);
            stderr.push_str("\n[Stderr truncated: exceeded 4MB limit]");
        }

        // Check for plot image in work_dir with size bound via NotebookDriverPort
        let output_image = self.driver.read_plot_png_base64(&work_dir).await;

        if let Some(r) = repo {
            let cells = r
                .list_notebook_cells_scoped(&params.session_id, params.notebook_id.as_deref())
                .await?;
            let cell = NotebookCell {
                id: params.cell_id.clone(),
                session_id: params.session_id.clone(),
                cell_type: NotebookCellType::Code,
                source: params.code.clone(),
                cell_index: cells
                    .iter()
                    .find(|saved| saved.id == params.cell_id)
                    .map(|saved| saved.cell_index)
                    .unwrap_or(0),
                execution_count: Some(execution_count),
                status,
                stdout: Some(stdout.clone()),
                stderr: if stderr.is_empty() {
                    None
                } else {
                    Some(stderr.clone())
                },
                output_image: output_image.clone(),
                wall_ms: Some(wall_ms),
                epoch,
                updated_at: chrono::Utc::now().timestamp(),
                notebook_id: params.notebook_id.clone(),
                cell_version: params.cell_version,
            };
            r.save_notebook_cell(&cell).await?;
        }

        // No recipe execution record is fabricated for an ordinary notebook cell.
        let execution_record_id = None;

        let result = ExecuteCellResult {
            cell_id: params.cell_id,
            execution_count,
            status,
            stdout,
            stderr,
            output_image,
            wall_ms,
            epoch,
            execution_record_id,
            notebook_id: params.notebook_id,
            kernel_id: params.kernel_id,
            cell_version: params.cell_version,
            execution_request_id: params.execution_request_id,
            device_id: params.device_id,
        };

        if let Some(ref req_id) = result.execution_request_id {
            let mut kernels = self.kernels.write().await;
            if let Some(k) = kernels.get_mut(&key) {
                if k.recent_requests.len() >= 50 {
                    k.recent_requests.clear();
                }
                k.recent_requests.insert(req_id.clone(), result.clone());
            }
        }

        Ok(result)
    }
}
