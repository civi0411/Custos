//! Goose CLI Native Coding Harness Adapter (Gate 4 Hardened)
//!
//! Bridges Block Goose autonomous coding agent loops into Custos's `AgentRuntimePort`.
//! Accurately publishes a `HarnessProfile` with `ToolMediationLevel::ProviderGoverned`.

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
pub struct GooseHarnessAdapter {
    binary_path: PathBuf,
    workspace_root: PathBuf,
    default_timeout: Duration,
    mock_mode: bool,
    mock_responses: Arc<Mutex<Vec<String>>>,
    active_runs: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<()>>>>,
}

impl GooseHarnessAdapter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: PathBuf::from("goose"),
            workspace_root: workspace_root.into(),
            default_timeout: Duration::from_secs(60),
            mock_mode: false,
            mock_responses: Arc::new(Mutex::new(Vec::new())),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary_path = binary.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
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

    /// Normalizes Goose tool usages into structured ActionIntents.
    pub fn normalize_output_to_intents(&self, task_id: &str, raw_output: &str) -> Vec<ActionIntent> {
        let mut intents = Vec::new();

        for line in raw_output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    let tool_name = val.get("tool").or_else(|| val.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                    let params = val.get("parameters").or_else(|| val.get("arguments")).cloned().unwrap_or(serde_json::Value::Null);

                    if !tool_name.is_empty() {
                        let (action_name, target, risk) = match tool_name {
                            "developer__text_editor" | "text_editor" => {
                                let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("unknown");
                                ("write_file", path.to_string(), RiskLevel::High)
                            }
                            "developer__shell" | "shell" => {
                                let cmd = params.get("command").and_then(|v| v.as_str()).unwrap_or("");
                                ("shell_exec", cmd.to_string(), RiskLevel::High)
                            }
                            _ => (tool_name, "workspace".to_string(), RiskLevel::Medium),
                        };

                        let intent = ActionIntent::new(
                            new_id("intent"),
                            action_name.to_string(),
                            target,
                            params,
                            risk,
                        )
                        .with_task_id(task_id)
                        .with_assurance(Assurance::ProviderGoverned);

                        intents.push(intent);
                    }
                }
            }
        }

        intents
    }
}

#[async_trait]
impl AgentRuntimePort for GooseHarnessAdapter {
    fn harness_id(&self) -> &str {
        "goose"
    }

    fn profile(&self) -> HarnessProfile {
        HarnessProfile {
            harness_id: "goose".into(),
            tool_mediation: ToolMediationLevel::ProviderGoverned,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: false,
            cost_visibility: CostVisibility::Estimated,
        }
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        _context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let profile = self.profile();

        let raw_output = if self.mock_mode {
            let mut mocks = self.mock_responses.lock().await;
            if mocks.is_empty() {
                String::new()
            } else {
                mocks.remove(0)
            }
        } else {
            if !self.binary_path.exists() && which::which(&self.binary_path).is_err() {
                return Err(DomainError::Validation(format!(
                    "Goose CLI binary '{}' not found on PATH or disk",
                    self.binary_path.display()
                )));
            }
            String::new()
        };

        let intents = self.normalize_output_to_intents(&worker_run.task_id, &raw_output);

        for intent in &intents {
            profile.validate_intent_assurance(intent)?;
        }

        Ok(intents)
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        let mut runs = self.active_runs.lock().await;
        if let Some(tx) = runs.remove(run_id) {
            let _ = tx.send(());
        }
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
                format!("{{\"tool\": \"developer__text_editor\", \"parameters\": {{\"path\": \"src/app.rs\"}}}}\nGoose completed: {instruction}")
            } else {
                mocks.remove(0)
            };

            let intents = self.normalize_output_to_intents("native-goose-run", &output);
            return Ok(HarnessExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: output,
                stderr: String::new(),
                observed_effects: intents,
                mediation_level: ToolMediationLevel::ProviderGoverned,
            });
        }

        if !self.binary_path.exists() && which::which(&self.binary_path).is_err() {
            return Err(DomainError::Validation(format!(
                "Goose CLI binary '{}' not found on PATH or disk",
                self.binary_path.display()
            )));
        }

        let output = tokio::process::Command::new(&self.binary_path)
            .current_dir(cwd)
            .arg("run")
            .arg("--text")
            .arg(instruction)
            .output()
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to execute goose CLI: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let intents = self.normalize_output_to_intents("native-goose-run", &stdout);

        Ok(HarnessExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout,
            stderr,
            observed_effects: intents,
            mediation_level: ToolMediationLevel::ProviderGoverned,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_goose_profile_and_gate_4_enforcement() {
        let adapter = GooseHarnessAdapter::new("/tmp").with_mock_mode();
        let profile = adapter.profile();

        assert_eq!(profile.harness_id, "goose");
        assert_eq!(profile.tool_mediation, ToolMediationLevel::ProviderGoverned);

        let valid_intent = ActionIntent::new(
            new_id("intent"),
            "shell_exec".to_string(),
            "cargo test".to_string(),
            serde_json::json!({}),
            RiskLevel::High,
        )
        .with_task_id("task_1")
        .with_assurance(Assurance::ProviderGoverned);

        assert!(profile.validate_intent_assurance(&valid_intent).is_ok());
    }
}
