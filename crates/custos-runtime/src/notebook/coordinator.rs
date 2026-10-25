//! Python Kernel Coordinator
//!
//! Manages authorized Python execution kernels with epoch tracking,
//! execution counters, interrupts, resets, and output persistence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use custos_core::contracts::notebook::{NotebookDriverPort, NotebookWorkerSession};
use custos_core::contracts::storage::NotebookStoragePort;
use custos_domain::{
    CellExecutionStatus, DomainError, ExecuteCellParams, ExecuteCellResult, KernelStatus,
    NotebookCell, NotebookCellType, NotebookKernelState,
};

struct ActiveKernel {
    epoch: u32,
    execution_counter: u32,
    status: KernelStatus,
    python_path: PathBuf,
    worker: Option<Arc<dyn NotebookWorkerSession>>,
    work_dir: Option<PathBuf>,
    recent_requests: HashMap<String, ExecuteCellResult>,
}

#[derive(Clone)]
pub struct PythonKernelCoordinator {
    driver: Arc<dyn NotebookDriverPort>,
    kernels: Arc<RwLock<HashMap<String, ActiveKernel>>>,
}

impl PythonKernelCoordinator {
    pub fn new(driver: Arc<dyn NotebookDriverPort>) -> Self {
        Self {
            driver,
            kernels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Discovers system Python binary
    pub fn discover_python_path() -> PathBuf {
        let candidates = [
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
            "/usr/bin/python3",
            "python3",
            "python",
        ];
        for candidate in candidates {
            if let Ok(output) = std::process::Command::new(candidate)
                .arg("--version")
                .output()
            {
                if output.status.success() {
                    return PathBuf::from(candidate);
                }
            }
        }
        PathBuf::from("python3")
    }

    /// Retrieves Python version string
    pub fn get_python_version(python_path: &PathBuf) -> String {
        if let Ok(output) = std::process::Command::new(python_path)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                if !s.trim().is_empty() {
                    return s.trim().to_string();
                }
                let err_s = String::from_utf8_lossy(&output.stderr);
                if !err_s.trim().is_empty() {
                    return err_s.trim().to_string();
                }
            }
        }
        "Python 3.x".to_string()
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
            let ver = Self::get_python_version(&active.python_path);
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
            let mut state = r.get_notebook_kernel_state(session_id).await?;
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
            let py = Self::discover_python_path();
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
                },
            );
            Ok(state)
        } else {
            let py = Self::discover_python_path();
            let ver = Self::get_python_version(&py);
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
                },
            );
            Ok(state)
        }
    }

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

        let python_path = Self::discover_python_path();
        let work_dir = if let Some(ref cwd) = params.cwd {
            PathBuf::from(cwd)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let (epoch, execution_count, worker_session) = {
            let mut kernels = self.kernels.write().await;
            let kernel = kernels
                .entry(key.clone())
                .or_insert_with(|| ActiveKernel {
                    epoch: 1,
                    execution_counter: 0,
                    status: KernelStatus::Idle,
                    python_path: python_path.clone(),
                    worker: None,
                    work_dir: None,
                    recent_requests: HashMap::new(),
                });

            if kernel.status == KernelStatus::Busy {
                return Err(DomainError::Validation("A cell is already running in this session".into()));
            }
            if kernel.work_dir.as_ref().is_some_and(|previous| previous != &work_dir) {
                return Err(DomainError::Validation("Reset this session's Python worker before changing execution workspace".into()));
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
                    return Err(DomainError::Validation("Execution was interrupted or reset; its output is stale".into()));
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

        // Check for plot image in work_dir with size bound (8MB)
        let mut output_image: Option<String> = None;
        let possible_plot = work_dir.join("_plot.png");
        if possible_plot.exists() {
            if let Ok(meta) = tokio::fs::metadata(&possible_plot).await {
                if meta.len() <= 8 * 1024 * 1024 {
                    if let Ok(bytes) = tokio::fs::read(&possible_plot).await {
                        output_image = Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
                    }
                } else {
                    tracing::warn!(path = %possible_plot.display(), "Plot image exceeds 8MB limit; omitted from inline response");
                }
            }
        }

        if let Some(r) = repo {
            let cells = r.list_notebook_cells(&params.session_id).await?;
            let cell = NotebookCell {
                id: params.cell_id.clone(),
                session_id: params.session_id.clone(),
                cell_type: NotebookCellType::Code,
                source: params.code.clone(),
                cell_index: cells.iter()
                    .find(|saved| saved.id == params.cell_id).map(|saved| saved.cell_index).unwrap_or(0),
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
        if let Some(k) = kernels.get_mut(&key) {
            if let Some(worker) = k.worker.take() {
                worker.interrupt();
                k.epoch += 1;
                k.execution_counter = 0;
                k.work_dir = None;
                k.status = KernelStatus::Interrupted;
                return Ok(true);
            }
        }
        Ok(false)
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
        let py = Self::discover_python_path();
        let ver = Self::get_python_version(&py);
        let mut kernels = self.kernels.write().await;
        let entry = kernels
            .entry(key)
            .or_insert_with(|| ActiveKernel {
                epoch: 1,
                execution_counter: 0,
                status: KernelStatus::Idle,
                python_path: py.clone(),
                worker: None,
                work_dir: None,
                recent_requests: HashMap::new(),
            });

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

        if let Some(r) = repo {
            let _ = r.save_notebook_kernel_state(&state).await;
        }

        Ok(state)
    }
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        let _ = out.write_char(table[((n >> 18) & 63) as usize] as char);
        let _ = out.write_char(table[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            let _ = out.write_char(table[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            let _ = out.write_char(table[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_coordinator() -> PythonKernelCoordinator {
        PythonKernelCoordinator::new(Arc::new(custos_adapters::notebook::LocalPythonDriver))
    }

    #[tokio::test]
    async fn restarting_coordinator_advances_persisted_worker_epoch() {
        let store = custos_persistence::SqliteTaskStore::new_in_memory().unwrap();
        let repository = store.research();
        repository.save_notebook_kernel_state(&NotebookKernelState { session_id: "restart".into(), epoch: 4,
            status: KernelStatus::Busy, python_version: "Python".into(), execution_counter: 7,
            created_at: 0, updated_at: 0, ..Default::default() }).unwrap();
        let coordinator = test_coordinator();
        let status = coordinator.get_status("restart", Some(repository as &dyn NotebookStoragePort)).await.unwrap();
        assert_eq!(status.epoch, 5);
        assert_eq!(status.status, KernelStatus::Idle);
        assert_eq!(status.execution_counter, 0);
        assert_eq!(coordinator.get_status("restart", Some(repository as &dyn NotebookStoragePort)).await.unwrap().epoch, 5);
    }

    #[tokio::test]
    async fn interrupt_releases_busy_worker_and_rejects_stale_result() {
        let coordinator = test_coordinator();
        let temp = tempfile::tempdir().unwrap();
        let params = ExecuteCellParams { session_id: "interrupt-session".into(), cell_id: "cell".into(),
            code: "import time; time.sleep(30)".into(), workspace_id: None,
            cwd: Some(temp.path().to_string_lossy().into()), task_id: None, ..Default::default() };
        let running = { let coordinator = coordinator.clone(); let params = params.clone();
            tokio::spawn(async move { coordinator.execute_cell(params, None).await }) };
        for _ in 0..100 {
            if coordinator.kernels.read().await.get("interrupt-session").is_some_and(|kernel| kernel.status == KernelStatus::Busy) { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(coordinator.execute_cell(params.clone(), None).await.is_err(), "concurrent writer must not queue another execution");
        assert!(coordinator.interrupt("interrupt-session").await.unwrap());
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), running).await.unwrap().unwrap();
        assert!(result.is_err());
        let mut next = params; next.code = "print(42)".into();
        let output = coordinator.execute_cell(next, None).await.unwrap();
        assert_eq!(output.stdout.trim(), "42");
        assert_eq!(output.epoch, 2);
        coordinator.reset("interrupt-session", None).await.unwrap();
    }

    #[tokio::test]
    async fn test_python_kernel_execution_and_epoch_reset() {
        let coordinator = test_coordinator();

        // 1. Initial status
        let initial_state = coordinator.get_status("test_sess", None).await.unwrap();
        assert_eq!(initial_state.epoch, 1);
        assert_eq!(initial_state.execution_counter, 0);

        // 2. Execute simple print cell
        let res = coordinator
            .execute_cell(
                ExecuteCellParams {
                    session_id: "test_sess".into(),
                    cell_id: "cell_1".into(),
                    code: "print('Custos Kernel Epoch Test')".into(),
                    workspace_id: None,
                    cwd: None,
                    task_id: None,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(res.status, CellExecutionStatus::Success);
        assert_eq!(res.execution_count, 1);
        assert_eq!(res.epoch, 1);
        assert!(res.stdout.contains("Custos Kernel Epoch Test"));

        // 3. Reset kernel increments epoch and clears execution count
        let reset_state = coordinator.reset("test_sess", None).await.unwrap();
        assert_eq!(reset_state.epoch, 2);
        assert_eq!(reset_state.execution_counter, 0);

        // 4. Next execution runs in epoch 2 with execution count 1
        let res2 = coordinator
            .execute_cell(
                ExecuteCellParams {
                    session_id: "test_sess".into(),
                    cell_id: "cell_2".into(),
                    code: "x = 42\nprint(f'x is {x}')".into(),
                    workspace_id: None,
                    cwd: None,
                    task_id: None,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(res2.status, CellExecutionStatus::Success);
        assert_eq!(res2.epoch, 2);
        assert_eq!(res2.execution_count, 1);
        assert!(res2.stdout.contains("x is 42"));

        // 5. Test variable persistence in subsequent cell
        let res3 = coordinator
            .execute_cell(
                ExecuteCellParams {
                    session_id: "test_sess".into(),
                    cell_id: "cell_3".into(),
                    code: "print(f'persisted x: {x * 2}')".into(),
                    workspace_id: None,
                    cwd: None,
                    task_id: None,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(res3.status, CellExecutionStatus::Success);
        assert_eq!(res3.epoch, 2);
        assert_eq!(res3.execution_count, 2);
        assert!(res3.stdout.contains("persisted x: 84"));
    }

    #[tokio::test]
    async fn test_multiple_notebooks_in_same_session_are_isolated() {
        let coordinator = test_coordinator();

        let s1 = coordinator.get_status_scoped("sess-1", Some("nb-a"), None).await.unwrap();
        assert_eq!(s1.notebook_id.as_deref(), Some("nb-a"));
        assert_eq!(s1.execution_counter, 0);

        let s2 = coordinator.get_status_scoped("sess-1", Some("nb-b"), None).await.unwrap();
        assert_eq!(s2.notebook_id.as_deref(), Some("nb-b"));
        assert_eq!(s2.execution_counter, 0);

        // Reset nb-a does not affect nb-b
        coordinator.reset_scoped("sess-1", Some("nb-a"), None).await.unwrap();
        let s1_after = coordinator.get_status_scoped("sess-1", Some("nb-a"), None).await.unwrap();
        let s2_after = coordinator.get_status_scoped("sess-1", Some("nb-b"), None).await.unwrap();
        assert_eq!(s1_after.epoch, 2);
        assert_eq!(s2_after.epoch, 1);
    }

    #[tokio::test]
    async fn test_duplicate_execution_request_id_returns_cached_result() {
        let coordinator = test_coordinator();
        let params = ExecuteCellParams {
            session_id: "idempotent_sess".into(),
            cell_id: "cell_1".into(),
            code: "print('computed_value')".into(),
            execution_request_id: Some("req-12345".into()),
            ..Default::default()
        };

        let res1 = coordinator.execute_cell(params.clone(), None).await.unwrap();
        assert_eq!(res1.execution_count, 1);
        assert!(res1.stdout.contains("computed_value"));

        // Second call with same execution_request_id returns cached result without incrementing counter
        let res2 = coordinator.execute_cell(params, None).await.unwrap();
        assert_eq!(res2.execution_count, 1);
        assert_eq!(res2.execution_request_id.as_deref(), Some("req-12345"));
        assert!(res2.stdout.contains("computed_value"));
    }
}
