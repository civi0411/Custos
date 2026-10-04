//! Sovereign Developer Capability Adapter (Goose Developer Tools Transformed)
//!
//! Adapts Goose's developer tool execution mechanics (bash, file read, write, list)
//! into Custos's sovereign `CapabilityPort`.
//!
//! Key Security Guarantees:
//! 1. Every dispatch requires a valid, unexpired `ExecutionPermit` whose action_id matches.
//! 2. Path boundaries are strictly constrained to the workspace root, preventing directory traversal.
//! 3. Command execution enforces a deterministic timeout and captures output.
//! 4. Every effect produces an append-only `ExecutionReceipt` with a Blake3 digest of the output.

use async_trait::async_trait;
use custos_domain::{
    digest, new_id, ActionIntent, DomainError, ExecutionReceipt, ReceiptStatus,
};
use custos_core::contracts::sandbox::{SandboxPort, verify_permit_binding};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct SovereignDeveloperAdapter {
    workspace_root: PathBuf,
    default_timeout: Duration,
}

impl SovereignDeveloperAdapter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            default_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Validates that a requested file path does not escape the workspace root.
    fn canonicalize_safe_path(&self, rel_or_abs: &str) -> Result<PathBuf, DomainError> {
        let path = Path::new(rel_or_abs);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Normalize lexical dots
        let mut components = Vec::new();
        for comp in resolved.components() {
            match comp {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(c);
                }
                std::path::Component::RootDir => {
                    components.clear();
                    components.push(std::ffi::OsStr::new("/"));
                }
                _ => {}
            }
        }

        let mut normalized = PathBuf::new();
        for c in components {
            normalized.push(c);
        }

        // Check if within workspace root if workspace root is set and absolute
        if self.workspace_root.is_absolute() && normalized.is_absolute() {
            if !normalized.starts_with(&self.workspace_root) {
                return Err(DomainError::Unauthorized(format!(
                    "Path traversal violation: target '{}' escapes workspace root '{}'",
                    rel_or_abs,
                    self.workspace_root.display()
                )));
            }
        }

        Ok(normalized)
    }

    async fn handle_read(&self, params: &serde_json::Value) -> Result<serde_json::Value, DomainError> {
        let path_str = params
            .get("path")
            .or_else(|| params.get("file"))
            .or_else(|| params.get("target_file"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Validation("Missing 'path' parameter for read".into()))?;

        let safe_path = self.canonicalize_safe_path(path_str)?;
        let content = tokio::fs::read_to_string(&safe_path).await.map_err(|e| {
            DomainError::Validation(format!(
                "Failed to read file '{}': {}",
                safe_path.display(),
                e
            ))
        })?;

        Ok(serde_json::json!({
            "path": safe_path.to_string_lossy(),
            "content": content,
            "bytes": content.len(),
        }))
    }

    async fn handle_write(&self, params: &serde_json::Value) -> Result<serde_json::Value, DomainError> {
        let path_str = params
            .get("path")
            .or_else(|| params.get("file"))
            .or_else(|| params.get("target_file"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Validation("Missing 'path' parameter for write".into()))?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Validation("Missing 'content' parameter for write".into()))?;

        let safe_path = self.canonicalize_safe_path(path_str)?;

        if let Some(parent) = safe_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                DomainError::InvariantViolation(format!("Failed to create parent dirs for '{}': {}", safe_path.display(), e))
            })?;
        }

        tokio::fs::write(&safe_path, content).await.map_err(|e| {
            DomainError::InvariantViolation(format!("Failed to write file '{}': {}", safe_path.display(), e))
        })?;

        Ok(serde_json::json!({
            "path": safe_path.to_string_lossy(),
            "bytes_written": content.len(),
            "status": "written",
        }))
    }

    async fn handle_list(&self, params: &serde_json::Value) -> Result<serde_json::Value, DomainError> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let safe_path = self.canonicalize_safe_path(path_str)?;
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&safe_path).await.map_err(|e| {
            DomainError::Validation(format!("Failed to list directory '{}': {}", safe_path.display(), e))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| DomainError::InvariantViolation(e.to_string()))? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            entries.push(serde_json::json!({
                "name": file_name,
                "is_dir": is_dir,
            }));
        }

        Ok(serde_json::json!({
            "path": safe_path.to_string_lossy(),
            "entries": entries,
            "count": entries.len(),
        }))
    }

    async fn handle_bash(&self, params: &serde_json::Value) -> Result<serde_json::Value, DomainError> {
        let command = params
            .get("command")
            .or_else(|| params.get("cmd"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Validation("Missing 'command' parameter for bash".into()))?;

        let timeout_secs = params
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(&self.workspace_root);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let start = Instant::now();
        let child = cmd.spawn().map_err(|e| {
            DomainError::InvariantViolation(format!("Failed to spawn shell process: {}", e))
        })?;

        let output = match tokio::time::timeout(timeout_secs, child.wait_with_output()).await {
            Ok(res) => res.map_err(|e| DomainError::InvariantViolation(format!("Process error: {}", e)))?,
            Err(_) => {
                return Err(DomainError::Validation(format!(
                    "Command '{}' timed out after {:?}",
                    command, timeout_secs
                )));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(serde_json::json!({
            "command": command,
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "duration_ms": duration_ms,
        }))
    }
}

#[async_trait]
impl SandboxPort for SovereignDeveloperAdapter {
    fn driver_id(&self) -> &str {
        "developer"
    }

    async fn execute(
        &self,
        action: &ActionIntent,
        permit: &custos_domain::Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        // 1. Invariant I2: Strict permit correlation & TTL expiration via Kernel's verify_permit_binding
        verify_permit_binding(permit, action)?;

        let start = Instant::now();

        // 3. Dispatch specific developer tool based on action name
        let execution_result = match action.name.as_str() {
            "developer__read" | "read_file" | "read" => self.handle_read(&action.parameters).await,
            "developer__write" | "write_file" | "write" => self.handle_write(&action.parameters).await,
            "developer__list" | "list_dir" | "ls" => self.handle_list(&action.parameters).await,
            "developer__bash" | "bash" | "shell" | "exec" => self.handle_bash(&action.parameters).await,
            other => Err(DomainError::Validation(format!(
                "Unknown developer capability action: '{}'",
                other
            ))),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // 4. Seal into immutable ExecutionReceipt with Blake3 digest
        match execution_result {
            Ok(output_data) => {
                let json_bytes = serde_json::to_vec(&output_data).unwrap_or_default();
                let output_hash = format!("blake3:{}", digest(&json_bytes));
                let is_failure = output_data.get("exit_code").and_then(|c| c.as_i64()).map(|c| c != 0).unwrap_or(false);

                Ok(ExecutionReceipt {
                    receipt_id: new_id("rcpt"),
                    permit_id: permit.id.clone(),
                    action_id: action.id.clone(),
                    status: if is_failure {
                        ReceiptStatus::Failure
                    } else {
                        ReceiptStatus::Success
                    },
                    output_digest: output_hash,
                    output_data: Some(output_data),
                    error_message: None,
                    duration_ms: Some(duration_ms),
                    executed_at: chrono::Utc::now(),
                    assurance: custos_domain::Assurance::CustosMediated,
                })
            }
            Err(err) => {
                let err_msg = err.to_string();
                let output_hash = format!("blake3:{}", digest(err_msg.as_bytes()));

                Ok(ExecutionReceipt {
                    receipt_id: new_id("rcpt"),
                    permit_id: permit.id.clone(),
                    action_id: action.id.clone(),
                    status: ReceiptStatus::Failure,
                    output_digest: output_hash,
                    output_data: None,
                    error_message: Some(err_msg),
                    duration_ms: Some(duration_ms),
                    executed_at: chrono::Utc::now(),
                    assurance: custos_domain::Assurance::CustosMediated,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{Action, Permit, RiskClass, RiskLevel};

    #[tokio::test]
    async fn test_sovereign_developer_adapter_read_write_flow() {
        let temp_dir = std::env::temp_dir().join(format!("custos_dev_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let adapter = SovereignDeveloperAdapter::new(&temp_dir);

        // 1. Create Write Action & Permit
        let write_action = Action::new(
            "act_write_1".into(),
            "developer__write".into(),
            "hello.txt".into(),
            serde_json::json!({
                "path": "hello.txt",
                "content": "Sovereign Custos Execution",
            }),
            RiskLevel::Medium,
        ).with_task_id("task_1");

        let write_permit = Permit::new(
            "task_1".into(),
            "act_write_1".into(),
            "developer".into(),
            None,
            RiskClass::Medium,
            60,
        );

        let write_receipt = adapter.execute(&write_action, &write_permit).await.unwrap();
        assert_eq!(write_receipt.status, ReceiptStatus::Success);
        assert!(write_receipt.output_digest.starts_with("blake3:"));

        // 2. Create Read Action & Permit
        let read_action = Action::new(
            "act_read_1".into(),
            "developer__read".into(),
            "hello.txt".into(),
            serde_json::json!({
                "path": "hello.txt",
            }),
            RiskLevel::Low,
        ).with_task_id("task_1");

        let read_permit = Permit::new(
            "task_1".into(),
            "act_read_1".into(),
            "developer".into(),
            None,
            RiskClass::Low,
            60,
        );

        let read_receipt = adapter.execute(&read_action, &read_permit).await.unwrap();
        assert_eq!(read_receipt.status, ReceiptStatus::Success);
        let data = read_receipt.output_data.unwrap();
        assert_eq!(data["content"], "Sovereign Custos Execution");

        // Clean up
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_sovereign_developer_adapter_rejects_path_traversal() {
        let temp_dir = std::env::temp_dir().join(format!("custos_dev_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let adapter = SovereignDeveloperAdapter::new(&temp_dir);

        let action = Action::new(
            "act_escape".into(),
            "developer__read".into(),
            "../../../etc/passwd".into(),
            serde_json::json!({
                "path": "../../../etc/passwd",
            }),
            RiskLevel::High,
        ).with_task_id("task_1");

        let permit = Permit::new(
            "task_1".into(),
            "act_escape".into(),
            "developer".into(),
            None,
            RiskClass::High,
            60,
        );

        let receipt = adapter.execute(&action, &permit).await.unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Failure);
        assert!(receipt.error_message.unwrap().contains("Path traversal violation"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}

