//! Generic CLI & Governed Harness Adapter (Gate 4 Hardened)
//!
//! Provides a standard execution and capability wrapper for external coding agent CLIs
//! (such as Antigravity `agy`, Gemini `gemini`, Aider `aider`, Cursor `cursor-agent`,
//! Cline `cline`, and Custos Governed Agent Runtime) adhering to Port 7 contracts.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use custos_core::contracts::harness::{
    AgentRuntimePort, CostVisibility, HarnessExecutionResult, HarnessProfile, ToolMediationLevel,
    WorktreeOwnership,
};
use custos_domain::{
    new_id, ActionIntent, Assurance, ContextPack, DomainError, RiskLevel, WorkerRun,
};

#[derive(Clone)]
pub struct GenericCliHarnessAdapter {
    id: String,
    binary_path: PathBuf,
    workspace_root: PathBuf,
    _default_timeout: Duration,
    mediation_level: ToolMediationLevel,
    mock_mode: bool,
    mock_responses: Arc<Mutex<Vec<String>>>,
    active_runs: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<()>>>>,
}

impl GenericCliHarnessAdapter {
    pub fn new(
        id: impl Into<String>,
        binary_name: impl Into<PathBuf>,
        workspace_root: impl Into<PathBuf>,
        mediation_level: ToolMediationLevel,
    ) -> Self {
        Self {
            id: id.into(),
            binary_path: binary_name.into(),
            workspace_root: workspace_root.into(),
            _default_timeout: Duration::from_secs(120),
            mediation_level,
            mock_mode: false,
            mock_responses: Arc::new(Mutex::new(Vec::new())),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_mock_mode(mut self) -> Self {
        self.mock_mode = true;
        self
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub async fn push_mock_response(&self, response: impl Into<String>) {
        self.mock_responses.lock().await.push(response.into());
    }

    fn resolve_binary(&self) -> Option<PathBuf> {
        let bin_str = self.binary_path.to_str().unwrap_or("");
        if bin_str == "internal" {
            return Some(self.binary_path.clone());
        }
        for sub in bin_str.split(|c| c == ',' || c == '|').map(str::trim) {
            let bin_name = sub.split_whitespace().next().unwrap_or(sub);
            let bin = Path::new(bin_name);
            if bin.exists() {
                return Some(bin.to_path_buf());
            }
            if let Ok(path) = which::which(bin) {
                return Some(path);
            }
            // Check common locations on macOS / Linux
            let candidates = [
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/usr/bin",
                "/bin",
            ];
            for dir in candidates {
                let candidate = Path::new(dir).join(bin);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                let candidate1 = Path::new(&home).join(".cargo/bin").join(bin);
                if candidate1.exists() {
                    return Some(candidate1);
                }
                let candidate2 = Path::new(&home).join(".local/bin").join(bin);
                if candidate2.exists() {
                    return Some(candidate2);
                }
            }
        }
        None
    }
}

#[async_trait]
impl AgentRuntimePort for GenericCliHarnessAdapter {
    fn harness_id(&self) -> &str {
        &self.id
    }

    fn profile(&self) -> HarnessProfile {
        HarnessProfile {
            harness_id: self.id.clone(),
            tool_mediation: self.mediation_level,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: true,
            cost_visibility: CostVisibility::Estimated,
        }
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        _context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let profile = self.profile();
        let intent = ActionIntent::new(
            new_id("intent"),
            "agent_turn".to_string(),
            self.id.clone(),
            serde_json::json!({ "instruction": worker_run.task_id }),
            RiskLevel::Medium,
        )
        .with_task_id(&worker_run.task_id)
        .with_assurance(match self.mediation_level {
            ToolMediationLevel::CustosMediated => Assurance::CustosMediated,
            _ => Assurance::ProviderGoverned,
        });

        profile.validate_intent_assurance(&intent)?;
        Ok(vec![intent])
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        let active = self.active_runs.lock().await;
        if let Some(cancel_tx) = active.get(run_id) {
            let _ = cancel_tx.send(());
        }
        Ok(())
    }

    async fn steer_run(&self, _run_id: &str, _guidance: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        if self.mock_mode {
            let mut mocks = self.mock_responses.lock().await;
            let output = if mocks.is_empty() {
                format!("{}: processed task '{instruction}'", self.id)
            } else {
                mocks.remove(0)
            };
            return Ok(HarnessExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: output,
                stderr: String::new(),
                observed_effects: vec![],
                mediation_level: self.mediation_level,
            });
        }

        let resolved_bin = self.resolve_binary().ok_or_else(|| {
            DomainError::Validation(format!(
                "CLI binary '{}' for agent '{}' is not installed or not found on PATH",
                self.binary_path.display(),
                self.id
            ))
        })?;

        // Internal Custos Governed Agent Runtime execution
        if resolved_bin.as_os_str() == "internal" {
            return Ok(HarnessExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: format!("Custos Governed Runtime dispatched: {instruction}"),
                stderr: String::new(),
                observed_effects: vec![],
                mediation_level: ToolMediationLevel::CustosMediated,
            });
        }

        // Generic CLI execution with agent-tailored prompt arguments
        let mut cmd = tokio::process::Command::new(&resolved_bin);
        cmd.current_dir(cwd);

        match self.id.as_str() {
            "claude" | "claude-code" | "openclaude" => {
                cmd.arg("-p").arg(instruction);
            }
            "goose" => {
                cmd.arg("run").arg("--text").arg(instruction);
            }
            "opencode" | "opencode2" => {
                cmd.arg("run").arg(instruction);
            }
            "gemini" => {
                cmd.arg("-p").arg(instruction);
            }
            "antigravity" => {
                cmd.arg("-p").arg(instruction);
            }
            "aider" => {
                cmd.arg("--message").arg(instruction).arg("--yes-always");
            }
            "cursor" => {
                cmd.arg("--").arg(instruction);
            }
            "kimi" => {
                cmd.arg("-p").arg(instruction);
            }
            "trae" | "prime-agent" | "grok" => {
                cmd.arg("--").arg(instruction);
            }
            _ => {
                cmd.arg(instruction);
            }
        }

        let output = cmd.output().await.map_err(|e| {
            DomainError::Validation(format!("Failed to execute {} CLI: {e}", self.id))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(HarnessExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout,
            stderr,
            observed_effects: vec![],
            mediation_level: self.mediation_level,
        })
    }
}
