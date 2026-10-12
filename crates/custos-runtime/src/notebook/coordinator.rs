//! Python Kernel Coordinator
//!
//! Manages authorized Python execution kernels with epoch tracking,
//! execution counters, interrupts, resets, and output persistence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, RwLock};

use custos_domain::{
    CellExecutionStatus, DomainError, ExecuteCellParams, ExecuteCellResult, KernelStatus,
    NotebookCell, NotebookCellType, NotebookKernelState,
};
use custos_persistence::ResearchRepository;

struct WorkerRequest {
    code: String,
    cwd: Option<String>,
    resp_tx: oneshot::Sender<Result<WorkerResponse, DomainError>>,
}

#[derive(serde::Deserialize)]
struct WorkerResponse {
    status: String,
    stdout: String,
    stderr: String,
}

struct WorkerHandle {
    sender: mpsc::Sender<WorkerRequest>,
    pid: Option<u32>,
}

const PYTHON_RUNNER_SCRIPT: &str = r#"
import sys, json, traceback, io, os

__globals__ = {
    '__name__': '__main__',
    '__builtins__': __builtins__,
}

while True:
    line = sys.stdin.readline()
    if not line:
        break
    try:
        req = json.loads(line)
        code = req.get('code', '')
        cwd = req.get('cwd')
        if cwd and os.path.isdir(cwd):
            try:
                os.chdir(cwd)
            except Exception:
                pass
        
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        s_out = io.StringIO()
        s_err = io.StringIO()
        sys.stdout = s_out
        sys.stderr = s_err
        
        status = 'success'
        try:
            compiled = compile(code, '<notebook_cell>', 'exec')
            exec(compiled, __globals__)
        except Exception:
            status = 'error'
            traceback.print_exc(file=s_err)
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
            
        resp = {
            'status': status,
            'stdout': s_out.getvalue(),
            'stderr': s_err.getvalue()
        }
        sys.stdout.write(json.dumps(resp) + '\n')
        sys.stdout.flush()
    except Exception as e:
        resp = {'status': 'error', 'stdout': '', 'stderr': str(e)}
        sys.stdout.write(json.dumps(resp) + '\n')
        sys.stdout.flush()
"#;

async fn spawn_worker_actor(
    python_path: PathBuf,
    work_dir: PathBuf,
) -> Result<WorkerHandle, DomainError> {
    let (tx, mut rx) = mpsc::channel::<WorkerRequest>(32);

    let mut cmd = Command::new(&python_path);
    cmd.arg("-u")
        .arg("-c")
        .arg(PYTHON_RUNNER_SCRIPT)
        .current_dir(&work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| DomainError::Validation(format!("Failed to launch Python worker: {e}")))?;

    let pid = child.id();
    let mut stdin = child.stdin.take().ok_or_else(|| {
        DomainError::Validation("Failed to acquire stdin for Python worker".into())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        DomainError::Validation("Failed to acquire stdout for Python worker".into())
    })?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            let payload = serde_json::json!({
                "code": req.code,
                "cwd": req.cwd
            });
            let mut line = serde_json::to_string(&payload).unwrap_or_default();
            line.push('\n');

            if let Err(e) = stdin.write_all(line.as_bytes()).await {
                let _ = req.resp_tx.send(Err(DomainError::Validation(format!(
                    "Failed to write to Python kernel stdin: {e}"
                ))));
                break;
            }
            if let Err(e) = stdin.flush().await {
                let _ = req.resp_tx.send(Err(DomainError::Validation(format!(
                    "Failed to flush Python kernel stdin: {e}"
                ))));
                break;
            }

            match reader.next_line().await {
                Ok(Some(resp_line)) => match serde_json::from_str::<WorkerResponse>(&resp_line) {
                    Ok(resp) => {
                        let _ = req.resp_tx.send(Ok(resp));
                    }
                    Err(e) => {
                        let _ = req.resp_tx.send(Err(DomainError::Validation(format!(
                            "Failed to parse Python kernel response: {e}; raw: {resp_line}"
                        ))));
                    }
                },
                Ok(None) => {
                    let _ = req.resp_tx.send(Err(DomainError::Validation(
                        "Python kernel process terminated unexpectedly".into(),
                    )));
                    break;
                }
                Err(e) => {
                    let _ = req.resp_tx.send(Err(DomainError::Validation(format!(
                        "Error reading from Python kernel: {e}"
                    ))));
                    break;
                }
            }
        }
        let _ = child.kill().await;
    });

    Ok(WorkerHandle { sender: tx, pid })
}

struct ActiveKernel {
    epoch: u32,
    execution_counter: u32,
    status: KernelStatus,
    python_path: PathBuf,
    worker: Option<WorkerHandle>,
}

#[derive(Clone)]
pub struct PythonKernelCoordinator {
    kernels: Arc<RwLock<HashMap<String, ActiveKernel>>>,
}

impl Default for PythonKernelCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonKernelCoordinator {
    pub fn new() -> Self {
        Self {
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

    pub async fn get_status(
        &self,
        session_id: &str,
        repo: Option<&ResearchRepository>,
    ) -> Result<NotebookKernelState, DomainError> {
        let mut kernels = self.kernels.write().await;
        if let Some(active) = kernels.get(session_id) {
            let ver = Self::get_python_version(&active.python_path);
            let state = NotebookKernelState {
                session_id: session_id.to_string(),
                epoch: active.epoch,
                status: active.status,
                python_version: ver,
                execution_counter: active.execution_counter,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
            };
            if let Some(r) = repo {
                let _ = r.save_notebook_kernel_state(&state);
            }
            Ok(state)
        } else if let Some(r) = repo {
            let state = r.get_notebook_kernel_state(session_id)?;
            let py = Self::discover_python_path();
            kernels.insert(
                session_id.to_string(),
                ActiveKernel {
                    epoch: state.epoch,
                    execution_counter: state.execution_counter,
                    status: state.status,
                    python_path: py,
                    worker: None,
                },
            );
            Ok(state)
        } else {
            let py = Self::discover_python_path();
            let ver = Self::get_python_version(&py);
            let state = NotebookKernelState {
                session_id: session_id.to_string(),
                epoch: 1,
                status: KernelStatus::Idle,
                python_version: ver,
                execution_counter: 0,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
            };
            kernels.insert(
                session_id.to_string(),
                ActiveKernel {
                    epoch: 1,
                    execution_counter: 0,
                    status: KernelStatus::Idle,
                    python_path: py,
                    worker: None,
                },
            );
            Ok(state)
        }
    }

    pub async fn execute_cell(
        &self,
        params: ExecuteCellParams,
        repo: Option<&ResearchRepository>,
    ) -> Result<ExecuteCellResult, DomainError> {
        let python_path = Self::discover_python_path();
        let work_dir = if let Some(ref cwd) = params.cwd {
            PathBuf::from(cwd)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let (epoch, execution_count, worker_sender) = {
            let mut kernels = self.kernels.write().await;
            let kernel = kernels
                .entry(params.session_id.clone())
                .or_insert_with(|| ActiveKernel {
                    epoch: 1,
                    execution_counter: 0,
                    status: KernelStatus::Idle,
                    python_path: python_path.clone(),
                    worker: None,
                });

            kernel.execution_counter += 1;
            kernel.status = KernelStatus::Busy;

            let sender = match &kernel.worker {
                Some(w) if !w.sender.is_closed() => w.sender.clone(),
                _ => {
                    let handle = spawn_worker_actor(python_path.clone(), work_dir.clone()).await?;
                    let tx = handle.sender.clone();
                    kernel.worker = Some(handle);
                    tx
                }
            };

            (kernel.epoch, kernel.execution_counter, sender)
        };

        let start = Instant::now();
        let (resp_tx, resp_rx) = oneshot::channel();
        let worker_req = WorkerRequest {
            code: params.code.clone(),
            cwd: params.cwd.clone(),
            resp_tx,
        };

        let worker_res = if worker_sender.send(worker_req).await.is_err() {
            // Worker channel closed, restart once and retry
            let handle = spawn_worker_actor(python_path.clone(), work_dir.clone()).await?;
            let sender = handle.sender.clone();
            {
                let mut kernels = self.kernels.write().await;
                if let Some(k) = kernels.get_mut(&params.session_id) {
                    k.worker = Some(handle);
                }
            }
            let (retry_tx, retry_rx) = oneshot::channel();
            let retry_req = WorkerRequest {
                code: params.code.clone(),
                cwd: params.cwd.clone(),
                resp_tx: retry_tx,
            };
            sender.send(retry_req).await.map_err(|_| {
                DomainError::Validation("Failed to communicate with restarted Python worker".into())
            })?;
            retry_rx.await.map_err(|_| {
                DomainError::Validation("Python worker dropped execution response".into())
            })?
        } else {
            resp_rx.await.map_err(|_| {
                DomainError::Validation("Python worker dropped execution response".into())
            })?
        };

        let wall_ms = start.elapsed().as_millis() as u64;

        {
            let mut kernels = self.kernels.write().await;
            if let Some(k) = kernels.get_mut(&params.session_id) {
                k.status = KernelStatus::Idle;
            }
        }

        let resp = worker_res?;

        let status = if resp.status == "success" {
            CellExecutionStatus::Success
        } else {
            CellExecutionStatus::Error
        };

        // Check for plot image in work_dir
        let mut output_image: Option<String> = None;
        let possible_plot = work_dir.join("_plot.png");
        if possible_plot.exists() {
            if let Ok(bytes) = tokio::fs::read(&possible_plot).await {
                output_image = Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
            }
        }

        if let Some(r) = repo {
            let cell = NotebookCell {
                id: params.cell_id.clone(),
                session_id: params.session_id.clone(),
                cell_type: NotebookCellType::Code,
                source: params.code.clone(),
                cell_index: 0,
                execution_count: Some(execution_count),
                status,
                stdout: Some(resp.stdout.clone()),
                stderr: if resp.stderr.is_empty() {
                    None
                } else {
                    Some(resp.stderr.clone())
                },
                output_image: output_image.clone(),
                wall_ms: Some(wall_ms),
                epoch,
                updated_at: chrono::Utc::now().timestamp(),
            };
            let _ = r.save_notebook_cell(&cell);
        }

        let execution_record_id = params
            .task_id
            .as_ref()
            .map(|tid| format!("exec_{}_{}", tid, execution_count));

        Ok(ExecuteCellResult {
            cell_id: params.cell_id,
            execution_count,
            status,
            stdout: resp.stdout,
            stderr: resp.stderr,
            output_image,
            wall_ms,
            epoch,
            execution_record_id,
        })
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<bool, DomainError> {
        let mut kernels = self.kernels.write().await;
        if let Some(k) = kernels.get_mut(session_id) {
            if let Some(worker) = k.worker.take() {
                if let Some(pid) = worker.pid {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                k.status = KernelStatus::Interrupted;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn reset(
        &self,
        session_id: &str,
        repo: Option<&ResearchRepository>,
    ) -> Result<NotebookKernelState, DomainError> {
        let py = Self::discover_python_path();
        let ver = Self::get_python_version(&py);
        let mut kernels = self.kernels.write().await;
        let entry = kernels
            .entry(session_id.to_string())
            .or_insert_with(|| ActiveKernel {
                epoch: 1,
                execution_counter: 0,
                status: KernelStatus::Idle,
                python_path: py.clone(),
                worker: None,
            });

        // Terminate active worker actor process if any
        if let Some(worker) = entry.worker.take() {
            if let Some(pid) = worker.pid {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
        }

        entry.epoch += 1;
        entry.execution_counter = 0;
        entry.status = KernelStatus::Idle;

        let state = NotebookKernelState {
            session_id: session_id.to_string(),
            epoch: entry.epoch,
            status: KernelStatus::Idle,
            python_version: ver,
            execution_counter: 0,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        if let Some(r) = repo {
            let _ = r.save_notebook_kernel_state(&state);
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

    #[tokio::test]
    async fn test_python_kernel_execution_and_epoch_reset() {
        let coordinator = PythonKernelCoordinator::new();

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
}
