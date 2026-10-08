//! Python Kernel Coordinator
//!
//! Manages authorized Python execution kernels with epoch tracking,
//! execution counters, interrupts, resets, and output persistence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::RwLock;

use custos_domain::{
    CellExecutionStatus, DomainError, ExecuteCellParams, ExecuteCellResult, KernelStatus,
    NotebookCell, NotebookCellType, NotebookKernelState,
};
use custos_persistence::ResearchRepository;

struct ActiveKernel {
    epoch: u32,
    execution_counter: u32,
    status: KernelStatus,
    python_path: PathBuf,
    active_child_pid: Option<u32>,
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
                    active_child_pid: None,
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
                    active_child_pid: None,
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
        let (epoch, execution_count) = {
            let mut kernels = self.kernels.write().await;
            let kernel = kernels
                .entry(params.session_id.clone())
                .or_insert_with(|| ActiveKernel {
                    epoch: 1,
                    execution_counter: 0,
                    status: KernelStatus::Idle,
                    python_path: python_path.clone(),
                    active_child_pid: None,
                });

            kernel.execution_counter += 1;
            kernel.status = KernelStatus::Busy;
            (kernel.epoch, kernel.execution_counter)
        };

        let temp_dir = tempfile::tempdir()
            .map_err(|e| DomainError::Validation(format!("Failed to create scratch dir: {e}")))?;
        let script_path = temp_dir.path().join("cell_exec.py");
        tokio::fs::write(&script_path, &params.code)
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to write cell code: {e}")))?;

        let work_dir = if let Some(ref cwd) = params.cwd {
            PathBuf::from(cwd)
        } else {
            temp_dir.path().to_path_buf()
        };

        let start = Instant::now();
        let mut cmd = Command::new(&python_path);
        cmd.arg("-u")
            .arg(&script_path)
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| DomainError::Validation(format!("Failed to launch Python: {e}")))?;

        let pid = child.id();
        {
            let mut kernels = self.kernels.write().await;
            if let Some(k) = kernels.get_mut(&params.session_id) {
                k.active_child_pid = pid;
            }
        }

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let wait_result = child.wait().await;
        let wall_ms = start.elapsed().as_millis() as u64;

        let mut stdout = String::new();
        if let Some(mut r) = stdout_handle {
            let _ = r.read_to_string(&mut stdout).await;
        }

        let mut stderr = String::new();
        if let Some(mut r) = stderr_handle {
            let _ = r.read_to_string(&mut stderr).await;
        }

        // Check for any plot image output generated in work_dir
        let mut output_image: Option<String> = None;
        let possible_plot = work_dir.join("_plot.png");
        if possible_plot.exists() {
            if let Ok(bytes) = tokio::fs::read(&possible_plot).await {
                output_image = Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
            }
        }

        let status = match wait_result {
            Ok(exit) if exit.success() => CellExecutionStatus::Success,
            _ => CellExecutionStatus::Error,
        };

        {
            let mut kernels = self.kernels.write().await;
            if let Some(k) = kernels.get_mut(&params.session_id) {
                k.status = KernelStatus::Idle;
                k.active_child_pid = None;
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
                stdout: Some(stdout.clone()),
                stderr: if stderr.is_empty() { None } else { Some(stderr.clone()) },
                output_image: output_image.clone(),
                wall_ms: Some(wall_ms),
                epoch,
                updated_at: chrono::Utc::now().timestamp(),
            };
            let _ = r.save_notebook_cell(&cell);
        }

        Ok(ExecuteCellResult {
            cell_id: params.cell_id,
            execution_count,
            status,
            stdout,
            stderr,
            output_image,
            wall_ms,
            epoch,
        })
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<bool, DomainError> {
        let mut kernels = self.kernels.write().await;
        if let Some(k) = kernels.get_mut(session_id) {
            if let Some(pid) = k.active_child_pid.take() {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as i32, libc::SIGINT);
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
        let entry = kernels.entry(session_id.to_string()).or_insert_with(|| ActiveKernel {
            epoch: 1,
            execution_counter: 0,
            status: KernelStatus::Idle,
            python_path: py.clone(),
            active_child_pid: None,
        });

        // Kill active process if any
        if let Some(pid) = entry.active_child_pid.take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
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
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(res2.status, CellExecutionStatus::Success);
        assert_eq!(res2.epoch, 2);
        assert_eq!(res2.execution_count, 1);
        assert!(res2.stdout.contains("x is 42"));
    }
}
