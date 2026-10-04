//! S1 Bounded Micro-Executor (Zero-Risk Preview Tasks)
//!
//! Provides fast, bounded execution for preview diffs, read receipts, and syntax checks.
//!
//! Invariants:
//! - INV-04 / INV-06: Strictly CANNOT grant authority, issue capability tokens, or self-issue permits.
//! - Execution time is strictly bounded to < 5 seconds.

use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, warn};

/// Maximum allowable execution time for S1 micro-tasks (5 seconds).
pub const MICRO_EXECUTOR_TIMEOUT: Duration = Duration::from_secs(5);

/// Safe, non-privileged micro-task operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MicroTask {
    /// Inspect/read file contents up to a maximum byte length.
    ReadFile { path: String, max_bytes: usize },
    /// Generate a unified preview diff between original and proposed content.
    PreviewDiff {
        path: String,
        original: String,
        proposed: String,
    },
    /// Verify JSON or YAML syntax validity.
    ValidateSyntax { content: String, format: String },
    /// Privileged action: MUST BE REJECTED by MicroExecutor (INV-04 / INV-06).
    IssuePermit { capability: String },
}

/// Execution receipt from a completed micro-task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroExecutionReceipt {
    pub task_type: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output: Option<String>,
    pub preview_diff: Option<String>,
    pub error: Option<String>,
}

/// Bounded zero-risk micro-executor.
#[derive(Debug, Clone, Default)]
pub struct MicroExecutor {
    timeout: Duration,
}

impl MicroExecutor {
    pub fn new() -> Self {
        Self {
            timeout: MICRO_EXECUTOR_TIMEOUT,
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Executes a bounded micro-task with strict timeout and capability constraints.
    pub async fn execute(
        &self,
        task: MicroTask,
        workspace_root: &Path,
    ) -> Result<MicroExecutionReceipt, DomainError> {
        let start = std::time::Instant::now();

        tokio::time::timeout(self.timeout, self.run_task(task, workspace_root, start))
            .await
            .map_err(|_| {
                DomainError::InvariantViolation(format!(
                    "S1 MicroExecutor task timed out after {:?}",
                    self.timeout
                ))
            })?
    }

    async fn run_task(
        &self,
        task: MicroTask,
        workspace_root: &Path,
        start: std::time::Instant,
    ) -> Result<MicroExecutionReceipt, DomainError> {
        match task {
            // INV-04 / INV-06: MicroExecutor CANNOT grant authority or self-issue permits!
            MicroTask::IssuePermit { capability } => {
                warn!(
                    capability = %capability,
                    "Violation: MicroExecutor attempted to issue permit (rejected by INV-04/INV-06)"
                );
                Err(DomainError::Unauthorized(format!(
                    "INV-04/INV-06 Violation: S1 MicroExecutor is not authorized to issue permits or grant authority for '{}'",
                    capability
                )))
            }

            MicroTask::ReadFile { path, max_bytes } => {
                let full_path = workspace_root.join(&path);
                if !full_path.exists() {
                    return Ok(MicroExecutionReceipt {
                        task_type: "ReadFile".into(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        output: None,
                        preview_diff: None,
                        error: Some(format!("File not found: {}", path)),
                    });
                }

                let content = std::fs::read(&full_path)
                    .map_err(|e| DomainError::Validation(format!("Failed to read file {}: {}", path, e)))?;
                let truncated = &content[..content.len().min(max_bytes)];
                let lossy_str = String::from_utf8_lossy(truncated).to_string();

                Ok(MicroExecutionReceipt {
                    task_type: "ReadFile".into(),
                    success: true,
                    duration_ms: start.elapsed().as_millis() as u64,
                    output: Some(lossy_str),
                    preview_diff: None,
                    error: None,
                })
            }

            MicroTask::PreviewDiff {
                path,
                original,
                proposed,
            } => {
                debug!(path = %path, "Generating preview diff in S1 micro-executor");
                let mut diff = format!("--- a/{}\n+++ b/{}\n", path, path);
                for line in original.lines() {
                    if !proposed.contains(line) {
                        diff.push_str(&format!("-{}\n", line));
                    }
                }
                for line in proposed.lines() {
                    if !original.contains(line) {
                        diff.push_str(&format!("+{}\n", line));
                    }
                }

                Ok(MicroExecutionReceipt {
                    task_type: "PreviewDiff".into(),
                    success: true,
                    duration_ms: start.elapsed().as_millis() as u64,
                    output: None,
                    preview_diff: Some(diff),
                    error: None,
                })
            }

            MicroTask::ValidateSyntax { content, format } => {
                let valid = match format.to_lowercase().as_str() {
                    "json" => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                    "yaml" => serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok(),
                    _ => false,
                };

                Ok(MicroExecutionReceipt {
                    task_type: "ValidateSyntax".into(),
                    success: valid,
                    duration_ms: start.elapsed().as_millis() as u64,
                    output: Some(if valid { "valid".into() } else { "invalid".into() }),
                    preview_diff: None,
                    error: if valid {
                        None
                    } else {
                        Some(format!("Invalid {} syntax", format))
                    },
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_micro_executor_blocks_authority_grant() {
        let executor = MicroExecutor::new();
        let dir = tempdir().unwrap();

        let res = executor
            .execute(
                MicroTask::IssuePermit {
                    capability: "fs.write".into(),
                },
                dir.path(),
            )
            .await;

        assert!(res.is_err());
        match res.unwrap_err() {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("INV-04/INV-06 Violation"));
            }
            other => panic!("Expected Unauthorized, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_micro_executor_preview_diff() {
        let executor = MicroExecutor::new();
        let dir = tempdir().unwrap();

        let receipt = executor
            .execute(
                MicroTask::PreviewDiff {
                    path: "src/main.rs".into(),
                    original: "fn main() { println!(\"old\"); }".into(),
                    proposed: "fn main() { println!(\"new\"); }".into(),
                },
                dir.path(),
            )
            .await
            .unwrap();

        assert!(receipt.success);
        let diff = receipt.preview_diff.unwrap();
        assert!(diff.contains("-fn main() { println!(\"old\"); }"));
        assert!(diff.contains("+fn main() { println!(\"new\"); }"));
    }

    #[tokio::test]
    async fn test_micro_executor_syntax_validation() {
        let executor = MicroExecutor::new();
        let dir = tempdir().unwrap();

        let receipt = executor
            .execute(
                MicroTask::ValidateSyntax {
                    content: "{\"valid\": true}".into(),
                    format: "json".into(),
                },
                dir.path(),
            )
            .await
            .unwrap();

        assert!(receipt.success);
        assert_eq!(receipt.output.as_deref(), Some("valid"));
    }
}
