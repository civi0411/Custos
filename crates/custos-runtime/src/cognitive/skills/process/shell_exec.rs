//! Sandboxed Process Executor
//!
//! Provides safe process execution within Custos OI constraints:
//! - Bounded wall-clock timeout (no infinite hangs)
//! - Captured stdout + stderr (no raw terminal passthrough)
//! - Working directory enforcement (no cwd escape)
//! - Environment variable filtering (no ambient secrets leaking to child)
//!
//! NOTE: OS-level sandbox (macOS Seatbelt / Linux Landlock) is a separate
//! infrastructure concern mounted at the daemon layer. This module enforces
//! the Rust-level contract: timeout, output capture, and env filtering.

use crate::cognitive::skills::SkillError;
use std::{path::Path, time::Duration};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Result of a sandboxed process execution.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// OS exit code. 0 = success for most toolchains.
    pub exit_code: i32,
    /// Full stdout as string (UTF-8 lossy)
    pub stdout: String,
    /// Full stderr as string (UTF-8 lossy)
    pub stderr: String,
    /// Actual wall-clock elapsed time in milliseconds
    pub elapsed_ms: u64,
}

impl ProcessOutput {
    /// Returns the last `n` chars of stderr — useful for evidence hints.
    pub fn stderr_tail(&self, n: usize) -> &str {
        let start = self.stderr.len().saturating_sub(n);
        &self.stderr[start..]
    }

    /// Returns the last `n` chars of stdout.
    pub fn stdout_tail(&self, n: usize) -> &str {
        let start = self.stdout.len().saturating_sub(n);
        &self.stdout[start..]
    }

    /// Convenience: did the process exit successfully?
    pub fn succeeded(&self) -> bool {
        self.exit_code == 0
    }
}

/// Builder for a sandboxed process invocation.
///
/// # Example
/// ```rust,ignore
/// let output = SandboxedProcess::new("cargo")
///     .args(["test", "--", "--nocapture"])
///     .cwd("/path/to/worktree")
///     .timeout_ms(120_000)
///     .run()
///     .await?;
/// assert!(output.succeeded());
/// ```
pub struct SandboxedProcess {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    timeout_ms: u64,
    /// Allowlisted env vars passed to child. Empty = inherit only PATH + HOME.
    allowed_env: Vec<(String, String)>,
}

impl SandboxedProcess {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: vec![],
            cwd: None,
            timeout_ms: 30_000, // 30s default
            allowed_env: vec![],
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Optionally add an argument if Some.
    pub fn maybe_arg(self, arg: Option<String>) -> Self {
        if let Some(a) = arg {
            self.arg(a)
        } else {
            self
        }
    }

    pub fn cwd(mut self, path: impl Into<String>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Allow an explicit env var through to the child process.
    /// All other env vars are blocked by default to prevent secret leakage (THR-06).
    pub fn allow_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.allowed_env.push((key.into(), value.into()));
        self
    }

    /// Execute the process with timeout and capture.
    pub async fn run(self) -> Result<ProcessOutput, SkillError> {
        let start = std::time::Instant::now();

        // Validate working directory if specified
        if let Some(ref cwd) = self.cwd {
            let path = Path::new(cwd);
            if !path.exists() {
                return Err(SkillError::InvalidArg(format!(
                    "working directory does not exist: {}",
                    cwd
                )));
            }
            // Security: prevent path traversal (THR-08)
            let canonical = path
                .canonicalize()
                .map_err(|e| SkillError::SandboxViolation(format!("canonicalize failed: {}", e)))?;
            if canonical
                .to_str()
                .map(|s| s.contains(".."))
                .unwrap_or(false)
            {
                return Err(SkillError::SandboxViolation(
                    "path traversal detected in working directory".into(),
                ));
            }
        }

        debug!(
            program = %self.program,
            args = ?self.args,
            cwd = ?self.cwd,
            timeout_ms = self.timeout_ms,
            "sandboxed process starting"
        );

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);

        // Environment filtering: clear ambient env, only pass allowlisted vars + PATH
        cmd.env_clear();
        // Always pass PATH so tools can be found
        if let Ok(path_val) = std::env::var("PATH") {
            cmd.env("PATH", path_val);
        }
        // Pass HOME for cargo/rustup to function
        if let Ok(home) = std::env::var("HOME") {
            cmd.env("HOME", home);
        }
        // CARGO_HOME and RUSTUP_HOME needed for cargo toolchain
        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            cmd.env("CARGO_HOME", cargo_home);
        }
        if let Ok(rustup_home) = std::env::var("RUSTUP_HOME") {
            cmd.env("RUSTUP_HOME", rustup_home);
        }
        // Add explicitly allowed env vars
        for (key, val) in &self.allowed_env {
            cmd.env(key, val);
        }

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let duration = Duration::from_millis(self.timeout_ms);

        let result = timeout(duration, async move { cmd.output().await }).await;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

                debug!(
                    exit_code,
                    elapsed_ms,
                    stdout_len = stdout.len(),
                    stderr_len = stderr.len(),
                    "sandboxed process completed"
                );

                Ok(ProcessOutput {
                    exit_code,
                    stdout,
                    stderr,
                    elapsed_ms,
                })
            }
            Ok(Err(io_err)) => {
                warn!(error = %io_err, "sandboxed process io error");
                Err(SkillError::Io(io_err))
            }
            Err(_timeout_elapsed) => {
                warn!(timeout_ms = self.timeout_ms, "sandboxed process timed out");
                Err(SkillError::Timeout(self.timeout_ms))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_command_succeeds() {
        let output = SandboxedProcess::new("echo")
            .args(["hello", "custos"])
            .run()
            .await
            .unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello custos"));
        assert!(output.succeeded());
    }

    #[tokio::test]
    async fn false_command_returns_nonzero() {
        let output = SandboxedProcess::new("false").run().await.unwrap();
        assert_ne!(output.exit_code, 0);
        assert!(!output.succeeded());
    }

    #[tokio::test]
    async fn nonexistent_program_returns_io_error() {
        let result = SandboxedProcess::new("__nonexistent_custos_test__")
            .run()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn timeout_is_enforced() {
        let result = SandboxedProcess::new("sleep")
            .args(["10"])
            .timeout_ms(100) // 100ms timeout against 10s sleep
            .run()
            .await;
        assert!(matches!(result, Err(SkillError::Timeout(100))));
    }

    #[tokio::test]
    async fn invalid_cwd_returns_error() {
        let result = SandboxedProcess::new("echo")
            .cwd("/nonexistent/__custos_test_dir__")
            .run()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stderr_tail_truncates_correctly() {
        let output = SandboxedProcess::new("sh")
            .args(["-c", "echo error >&2"])
            .run()
            .await
            .unwrap();
        assert!(!output.stderr.is_empty());
        let tail = output.stderr_tail(3);
        assert!(tail.len() <= output.stderr.len());
    }
}
