//! Local Python Notebook Driver
//!
//! Spawns Python worker processes, manages line-delimited JSON IPC framing with
//! strict frame byte caps to prevent OOM, and guarantees process-tree termination.

use async_trait::async_trait;
use custos_core::contracts::notebook::{
    NotebookDriverPort, NotebookWorkerResponse, NotebookWorkerSession,
};
use custos_domain::DomainError;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

pub const PYTHON_RUNNER_SCRIPT: &str = r#"
import sys, json, traceback, io, os

MAX_OUTPUT_BYTES = 4 * 1024 * 1024

class BoundedWriter:
    def __init__(self, limit=MAX_OUTPUT_BYTES, tag="Output"):
        self.limit = limit
        self.tag = tag
        self.buf = io.StringIO()
        self.total_bytes = 0
        self.truncated = False

    def write(self, s):
        if self.truncated:
            return
        b = s.encode('utf-8', errors='ignore')
        if self.total_bytes + len(b) > self.limit:
            remaining = self.limit - self.total_bytes
            if remaining > 0:
                self.buf.write(b[:remaining].decode('utf-8', errors='ignore'))
            self.buf.write(f"\n[{self.tag} truncated: exceeded 4MB limit]")
            self.total_bytes = self.limit + 1
            self.truncated = True
        else:
            self.buf.write(s)
            self.total_bytes += len(b)

    def flush(self):
        pass

    def getvalue(self):
        return self.buf.getvalue()

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
        s_out = BoundedWriter(MAX_OUTPUT_BYTES, "Output")
        s_err = BoundedWriter(MAX_OUTPUT_BYTES, "Stderr")
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
            
        out = s_out.getvalue()
        err = s_err.getvalue()
        resp = {
            'status': status,
            'stdout': out,
            'stderr': err
        }
        encoded = json.dumps(resp)
        if len(encoded.encode('utf-8')) > 14 * 1024 * 1024:
            resp = {
                'status': status,
                'stdout': out[:1024 * 1024] + "\n[Output truncated: escaped JSON frame exceeded 14MB limit]",
                'stderr': err[:1024 * 1024] + ("\n[Stderr truncated]" if err else "")
            }
            encoded = json.dumps(resp)
        sys.stdout.write(encoded + '\n')
        sys.stdout.flush()
    except Exception as e:
        err_msg = str(e)
        if len(err_msg.encode('utf-8')) > MAX_OUTPUT_BYTES:
            err_msg = err_msg.encode('utf-8')[:MAX_OUTPUT_BYTES].decode('utf-8', errors='ignore') + "\n[Stderr truncated: exceeded 4MB limit]"
        resp = {'status': 'error', 'stdout': '', 'stderr': err_msg}
        sys.stdout.write(json.dumps(resp) + '\n')
        sys.stdout.flush()
"#;

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024; // 16 MiB cap per response frame (safety headroom for escaped Unicode)

/// Reads a single newline-terminated line bounded by `max_bytes` to prevent buffer bloat/OOM.
async fn read_bounded_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<String>, DomainError> {
    let mut line = Vec::new();
    loop {
        let buf = reader.fill_buf().await.map_err(|e| {
            DomainError::Validation(format!("Error reading from Python kernel: {e}"))
        })?;
        if buf.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            break;
        }
        if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            if line.len() + pos > max_bytes {
                return Err(DomainError::Validation(format!(
                    "Python kernel response frame exceeded byte cap of {max_bytes} bytes"
                )));
            }
            line.extend_from_slice(&buf[..pos]);
            reader.consume(pos + 1);
            break;
        } else {
            if line.len() + buf.len() > max_bytes {
                return Err(DomainError::Validation(format!(
                    "Python kernel response frame exceeded byte cap of {max_bytes} bytes"
                )));
            }
            line.extend_from_slice(buf);
            let len = buf.len();
            reader.consume(len);
        }
    }
    if line.ends_with(b"\r") {
        line.pop();
    }
    String::from_utf8(line).map(Some).map_err(|e| {
        DomainError::Validation(format!("Python kernel response was not valid UTF-8: {e}"))
    })
}

/// Terminates an entire process tree given its root process ID.
fn terminate_process_tree(pid: u32) {
    #[cfg(unix)]
    unsafe {
        // Send SIGKILL to negative PGID to wipe the whole process group
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
    }
}

struct WorkerRequest {
    code: String,
    cwd: Option<String>,
    resp_tx: oneshot::Sender<Result<NotebookWorkerResponse, DomainError>>,
}

/// Active worker session handle communicating with the spawned child Python process.
pub struct LocalNotebookWorkerSession {
    tx: mpsc::Sender<WorkerRequest>,
    pid: u32,
    interrupted: Arc<AtomicBool>,
    abort_handle: tokio::task::AbortHandle,
}

#[async_trait]
impl NotebookWorkerSession for LocalNotebookWorkerSession {
    async fn execute(
        &self,
        code: String,
        cwd: Option<String>,
    ) -> Result<NotebookWorkerResponse, DomainError> {
        if self.interrupted.load(Ordering::SeqCst) {
            return Err(DomainError::Validation(
                "Python worker was interrupted".into(),
            ));
        }

        let (resp_tx, resp_rx) = oneshot::channel();
        let req = WorkerRequest { code, cwd, resp_tx };

        if self.tx.send(req).await.is_err() {
            return Err(DomainError::Validation(
                "Python worker is unavailable; reset it before retrying".into(),
            ));
        }

        match resp_rx.await {
            Ok(res) => res,
            Err(_) => Err(DomainError::Validation(
                "Python worker stopped before returning output; execution was not replayed".into(),
            )),
        }
    }

    fn interrupt(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
        self.abort_handle.abort();
        terminate_process_tree(self.pid);
    }
}

impl Drop for LocalNotebookWorkerSession {
    fn drop(&mut self) {
        self.abort_handle.abort();
        terminate_process_tree(self.pid);
    }
}

/// Local physical Python notebook driver implementing `NotebookDriverPort`.
#[derive(Clone, Default)]
pub struct LocalPythonDriver;

#[async_trait]
impl NotebookDriverPort for LocalPythonDriver {
    fn discover_python_path(&self) -> PathBuf {
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

    fn get_python_version(&self, python_path: &PathBuf) -> String {
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

    async fn probe_device_environment(
        &self,
        python_path: Option<&PathBuf>,
    ) -> Result<String, DomainError> {
        let py = python_path
            .cloned()
            .unwrap_or_else(|| self.discover_python_path());
        let check_script = "import sys; \
try:\n    import torch\n    print(f'torch:{torch.__version__},cuda:{torch.cuda.is_available()},mps:{hasattr(torch.backends, \"mps\") and torch.backends.mps.is_available()}')\n\
except Exception as e:\n    print(f'error:{e}')";

        let child = Command::new(py)
            .arg("-c")
            .arg(check_script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => return Ok(format!("python_probe_failed: {e}")),
        };

        match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait_with_output())
            .await
        {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if output.status.success() && !stdout.is_empty() {
                    Ok(stdout)
                } else if !stderr.is_empty() {
                    Ok(format!("probe_stderr: {stderr}"))
                } else {
                    Ok("torch_unavailable".to_string())
                }
            }
            Ok(Err(e)) => Ok(format!("probe_execution_error: {e}")),
            Err(_) => Ok("probe_timed_out_5s".to_string()),
        }
    }

    async fn spawn_worker(
        &self,
        python_path: PathBuf,
        work_dir: PathBuf,
    ) -> Result<Box<dyn NotebookWorkerSession>, DomainError> {
        let (tx, mut rx) = mpsc::channel::<WorkerRequest>(32);

        let mut cmd = Command::new(&python_path);
        cmd.arg("-u")
            .arg("-c")
            .arg(PYTHON_RUNNER_SCRIPT)
            .current_dir(&work_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| DomainError::Validation(format!("Failed to launch Python worker: {e}")))?;

        let pid = child.id().unwrap_or(0);
        let mut stdin = child.stdin.take().ok_or_else(|| {
            DomainError::Validation("Failed to acquire stdin for Python worker".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            DomainError::Validation("Failed to acquire stdout for Python worker".into())
        })?;

        let mut reader = tokio::io::BufReader::new(stdout);

        let actor = tokio::spawn(async move {
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

                match read_bounded_line(&mut reader, MAX_FRAME_BYTES).await {
                    Ok(Some(resp_line)) => {
                        match serde_json::from_str::<NotebookWorkerResponse>(&resp_line) {
                            Ok(resp) => {
                                let _ = req.resp_tx.send(Ok(resp));
                            }
                            Err(e) => {
                                let _ = req.resp_tx.send(Err(DomainError::Validation(format!(
                                    "Failed to parse Python kernel response: {e}; raw: {resp_line}"
                                ))));
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = req.resp_tx.send(Err(DomainError::Validation(
                            "Python kernel process terminated unexpectedly".into(),
                        )));
                        break;
                    }
                    Err(e) => {
                        let _ = req.resp_tx.send(Err(e));
                        break;
                    }
                }
            }
            let _ = child.kill().await;
            terminate_process_tree(pid);
        });

        Ok(Box::new(LocalNotebookWorkerSession {
            tx,
            pid,
            interrupted: Arc::new(AtomicBool::new(false)),
            abort_handle: actor.abort_handle(),
        }))
    }

    async fn read_plot_png_base64(&self, work_dir: &std::path::Path) -> Option<String> {
        let possible_plot = work_dir.join("_plot.png");
        let meta = tokio::fs::metadata(&possible_plot).await.ok()?;
        if meta.len() > 8 * 1024 * 1024 {
            tracing::warn!(path = %possible_plot.display(), "Plot image exceeds 8MB limit; omitted from inline response");
            return None;
        }
        let bytes = tokio::fs::read(&possible_plot).await.ok()?;
        Some(format!("data:image/png;base64,{}", encode_base64(&bytes)))
    }
}

fn encode_base64(data: &[u8]) -> String {
    use std::fmt::Write;
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        let _ = out.write_char(table[((n >> 18) & 63) as usize] as char);
        let _ = out.write_char(table[((n >> 12) & 63) as usize] as char);
        let _ = out.write_char(if chunk.len() > 1 {
            table[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        let _ = out.write_char(if chunk.len() > 2 {
            table[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
