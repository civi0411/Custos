use custos_core::contracts::worker_wire::{WorkerBudget, WorkerObservation, WorkerOperation};
use custos_domain::DomainError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

/// Runtime lease governing execution of an external or sandboxed worker subprocess.
pub struct WorkerLease {
    pub operation_id: String,
    pub component_id: String,
    pub version: String,
    pub execution_kind: custos_core::contracts::worker_wire::WorkerExecutionKind,
    pub worker_path: PathBuf,
    pub budget: WorkerBudget,
    child: Child,
    started_at: Instant,
}

impl WorkerLease {
    /// Spawns a worker binary, feeds the `WorkerOperation` via stdin, and captures process handle.
    pub async fn spawn(op: &WorkerOperation, worker_path: &Path) -> Result<Self, DomainError> {
        if !worker_path.exists() {
            return Err(DomainError::Validation(format!(
                "Worker executable not found at: {}",
                worker_path.display()
            )));
        }

        let mut cmd = Command::new(worker_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            DomainError::Validation(format!(
                "Failed to spawn worker binary {}: {}",
                worker_path.display(),
                e
            ))
        })?;

        // Write operation payload to stdin then close stream (EOF)
        if let Some(mut stdin) = child.stdin.take() {
            let input_bytes = serde_json::to_vec(op).map_err(|e| {
                DomainError::Validation(format!("Failed to serialize WorkerOperation: {}", e))
            })?;
            stdin.write_all(&input_bytes).await.map_err(|e| {
                DomainError::Validation(format!("Failed to write to worker stdin: {}", e))
            })?;
            stdin.flush().await.map_err(|e| {
                DomainError::Validation(format!("Failed to flush worker stdin: {}", e))
            })?;
            drop(stdin);
        }

        Ok(Self {
            operation_id: op.operation_id.clone(),
            component_id: op.component_id.clone(),
            version: op.version.clone(),
            execution_kind: op.execution_kind,
            worker_path: worker_path.to_path_buf(),
            budget: op.resource_budget.clone(),
            child,
            started_at: Instant::now(),
        })
    }

    /// Spawns using a custom command and arguments (useful for tests or wrapped engines)
    pub async fn spawn_command(
        op: &WorkerOperation,
        program: &str,
        args: &[&str],
    ) -> Result<Self, DomainError> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            DomainError::Validation(format!("Failed to spawn command {}: {}", program, e))
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            let input_bytes = serde_json::to_vec(op).map_err(|e| {
                DomainError::Validation(format!("Failed to serialize WorkerOperation: {}", e))
            })?;
            stdin.write_all(&input_bytes).await.map_err(|e| {
                DomainError::Validation(format!("Failed to write to worker stdin: {}", e))
            })?;
            stdin.flush().await.map_err(|e| {
                DomainError::Validation(format!("Failed to flush worker stdin: {}", e))
            })?;
            drop(stdin);
        }

        Ok(Self {
            operation_id: op.operation_id.clone(),
            component_id: op.component_id.clone(),
            version: op.version.clone(),
            execution_kind: op.execution_kind,
            worker_path: PathBuf::from(program),
            budget: op.resource_budget.clone(),
            child,
            started_at: Instant::now(),
        })
    }

    /// Awaits worker completion within the allocated wall-clock budget and enforces output limits.
    pub async fn await_observation(mut self) -> Result<WorkerObservation, DomainError> {
        let wall_timeout = Duration::from_secs(self.budget.max_wall_secs);

        let execution = async {
            let stdout = self.child.stdout.take();
            let stderr = self.child.stderr.take();

            let max_bytes = self.budget.max_output_bytes as usize;

            let (stdout_res, stderr_res) = tokio::join!(
                async {
                    let mut buf = Vec::new();
                    if let Some(mut out) = stdout {
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = out.read(&mut chunk).await?;
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if buf.len() > max_bytes {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("Output exceeded max buffer limit of {} bytes", max_bytes),
                                ));
                            }
                        }
                    }
                    Ok::<Vec<u8>, std::io::Error>(buf)
                },
                async {
                    let mut buf = Vec::new();
                    if let Some(mut err) = stderr {
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = err.read(&mut chunk).await?;
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if buf.len() > max_bytes {
                                break;
                            }
                        }
                    }
                    Ok::<Vec<u8>, std::io::Error>(buf)
                }
            );

            let stdout_buf = match stdout_res {
                Ok(b) => b,
                Err(e) => {
                    let _ = self.child.kill().await;
                    return Ok(WorkerObservation::failure(
                        &self.operation_id,
                        e.to_string(),
                        self.started_at.elapsed().as_millis() as u64,
                    ));
                }
            };
            let stderr_buf = stderr_res.unwrap_or_default();

            let status = self.child.wait().await?;
            let wall_ms = self.started_at.elapsed().as_millis() as u64;

            if !status.success() {
                let err_msg = String::from_utf8_lossy(&stderr_buf);
                return Ok(WorkerObservation::failure(
                    &self.operation_id,
                    format!("Worker process exited with code {:?}: {}", status.code(), err_msg.trim()),
                    wall_ms,
                ));
            }

            // Parse observation from stdout, or wrap raw JSON
            if let Ok(obs) = serde_json::from_slice::<WorkerObservation>(&stdout_buf) {
                return Ok(obs);
            }

            let parsed_output: serde_json::Value = serde_json::from_slice(&stdout_buf)
                .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&stdout_buf).to_string()));

            Ok(WorkerObservation::success(
                &self.operation_id,
                parsed_output,
                wall_ms,
                stdout_buf.len() as u64,
            ))
        };

        match timeout(wall_timeout, execution).await {
            Ok(result) => result.map_err(|e: std::io::Error| DomainError::Validation(e.to_string())),
            Err(_) => {
                let _ = self.child.kill().await;
                Ok(WorkerObservation::failure(
                    &self.operation_id,
                    format!("Operation timed out after {}s", self.budget.max_wall_secs),
                    self.started_at.elapsed().as_millis() as u64,
                ))
            }
        }
    }

    /// Aborts the running subprocess immediately.
    pub async fn cancel(mut self) -> Result<(), DomainError> {
        self.child
            .kill()
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to kill worker child: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_lease_echo_success() {
        let op = WorkerOperation {
            operation_id: "op_test_1".to_string(),
            component_id: "test".to_string(),
            version: "0.1.0".to_string(),
            operation_kind: "echo".to_string(),
            execution_kind: custos_core::contracts::worker_wire::WorkerExecutionKind::OneShot,
            payload: serde_json::json!({ "msg": "hello" }),
            resource_budget: WorkerBudget {
                max_wall_secs: 5,
                max_output_bytes: 4096,
                max_memory_bytes: None,
            },
        };

        // Run python to parse stdin and return JSON
        let lease = WorkerLease::spawn_command(
            &op,
            "python3",
            &["-c", "import sys, json; data = json.load(sys.stdin); print(json.dumps({'received': data['payload']}))"],
        )
        .await
        .expect("spawn");

        let obs = lease.await_observation().await.expect("await");
        assert!(obs.success);
        assert_eq!(obs.output.unwrap()["received"]["msg"], "hello");
    }

    #[tokio::test]
    async fn test_worker_lease_output_limit_enforcement() {
        let op = WorkerOperation {
            operation_id: "op_test_2".to_string(),
            component_id: "test".to_string(),
            version: "0.1.0".to_string(),
            operation_kind: "overflow".to_string(),
            execution_kind: custos_core::contracts::worker_wire::WorkerExecutionKind::OneShot,
            payload: serde_json::Value::Null,
            resource_budget: WorkerBudget {
                max_wall_secs: 5,
                max_output_bytes: 100, // Small limit
                max_memory_bytes: None,
            },
        };

        // Generates 1000 characters which exceeds 100 bytes limit
        let lease = WorkerLease::spawn_command(
            &op,
            "python3",
            &["-c", "print('A' * 1000)"],
        )
        .await
        .expect("spawn");

        let obs = lease.await_observation().await.expect("await");
        assert!(!obs.success);
        assert!(obs.error.unwrap().contains("Output exceeded max buffer limit"));
    }

    #[tokio::test]
    async fn test_worker_lease_non_zero_exit_capture() {
        let op = WorkerOperation {
            operation_id: "op_test_3".to_string(),
            component_id: "test".to_string(),
            version: "0.1.0".to_string(),
            operation_kind: "crash".to_string(),
            execution_kind: custos_core::contracts::worker_wire::WorkerExecutionKind::OneShot,
            payload: serde_json::Value::Null,
            resource_budget: WorkerBudget::default(),
        };

        let lease = WorkerLease::spawn_command(
            &op,
            "python3",
            &["-c", "import sys; sys.stderr.write('fatal domain error\\n'); sys.exit(42)"],
        )
        .await
        .expect("spawn");

        let obs = lease.await_observation().await.expect("await");
        assert!(!obs.success);
        let err = obs.error.unwrap();
        assert!(err.contains("42"));
        assert!(err.contains("fatal domain error"));
    }
}
