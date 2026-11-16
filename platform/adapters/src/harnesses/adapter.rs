//! Unified CLI & Governed Harness Adapter (Gate 4 Hardened)
//!
//! Provides a standardized execution and capability wrapper for external coding agent CLIs
//! (Claude Code, OpenAI Codex, OpenCode, Block Goose, Antigravity, Aider, Cursor, etc.)
//! adhering to Port 7 contracts and declarative [`HarnessSpec`] profiles.

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
use custos_domain::{ActionIntent, ContextPack, DomainError, WorkerRun};

use super::catalog::find_spec;
#[cfg(any(test, feature = "test-support"))]
use super::mock::MockHarnessState;
use super::spec::{HarnessSpec, HeadlessArgs, StreamProtocol};

/// Standardized CLI Agent Adapter alias conforming to SADE Ring 2 specifications.
pub type CliAgentAdapter = CliHarness;

#[derive(Clone)]
pub struct CliHarness {
    pub(crate) id: String,
    pub(crate) binary_path: PathBuf,
    pub(crate) workspace_root: PathBuf,
    pub(crate) default_timeout: Duration,
    pub(crate) mediation_level: ToolMediationLevel,
    pub(crate) headless: HeadlessArgs,
    pub(crate) protocol: StreamProtocol,
    pub(crate) cost_visibility: CostVisibility,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) mock_state: MockHarnessState,
    pub(crate) active_runs: Arc<Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>>,
    pub steer_messages: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl CliHarness {
    /// Construct a harness runtime adapter from a declarative specification.
    pub fn from_spec(spec: &HarnessSpec, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            id: spec.id.to_string(),
            binary_path: PathBuf::from(
                spec.bins
                    .split([',', '|'])
                    .next()
                    .unwrap_or(spec.bins.as_str())
                    .trim(),
            ),
            workspace_root: workspace_root.into(),
            default_timeout: Duration::from_secs(120),
            mediation_level: spec.mediation,
            headless: spec.headless,
            protocol: spec.protocol,
            cost_visibility: spec.cost,
            #[cfg(any(test, feature = "test-support"))]
            mock_state: MockHarnessState::new(),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            steer_messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Custom constructor for ad-hoc CLI harnesses or tests.
    pub fn new(
        id: impl Into<String>,
        binary_name: impl Into<PathBuf>,
        workspace_root: impl Into<PathBuf>,
        mediation_level: ToolMediationLevel,
    ) -> Self {
        let id_str = id.into();
        if let Some(spec) = find_spec(&id_str) {
            let mut harness = Self::from_spec(spec, workspace_root);
            harness.binary_path = binary_name.into();
            harness.mediation_level = mediation_level;
            return harness;
        }

        Self {
            id: id_str,
            binary_path: binary_name.into(),
            workspace_root: workspace_root.into(),
            default_timeout: Duration::from_secs(120),
            mediation_level,
            headless: HeadlessArgs::Positional,
            protocol: StreamProtocol::PlainText,
            cost_visibility: CostVisibility::Estimated,
            #[cfg(any(test, feature = "test-support"))]
            mock_state: MockHarnessState::new(),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            steer_messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn claude_code(workspace_root: impl Into<PathBuf>) -> Self {
        Self::from_spec(
            find_spec("claude-code").expect("claude-code spec defined"),
            workspace_root,
        )
    }

    pub fn codex(workspace_root: impl Into<PathBuf>) -> Self {
        Self::from_spec(
            find_spec("codex").expect("codex spec defined"),
            workspace_root,
        )
    }

    pub fn goose(workspace_root: impl Into<PathBuf>) -> Self {
        Self::from_spec(
            find_spec("goose").expect("goose spec defined"),
            workspace_root,
        )
    }

    pub fn opencode(workspace_root: impl Into<PathBuf>) -> Self {
        Self::from_spec(
            find_spec("opencode").expect("opencode spec defined"),
            workspace_root,
        )
    }

    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary_path = binary.into();
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn with_mock_mode(mut self) -> Self {
        self.mock_state.enabled = true;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub fn with_mediation_level(mut self, level: ToolMediationLevel) -> Self {
        self.mediation_level = level;
        self
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[cfg(any(test, feature = "test-support"))]
    pub async fn push_mock_response(&self, response: impl Into<String>) {
        self.mock_state.push_response(response).await;
    }

    pub fn normalize_output_to_intents(
        &self,
        task_id: &str,
        raw_output: &str,
    ) -> Vec<ActionIntent> {
        super::stream::extract_observed_intents(raw_output, &self.id, task_id)
    }

    pub fn build_command(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<tokio::process::Command, DomainError> {
        let resolved_bin = self.resolve_binary().ok_or_else(|| {
            DomainError::Validation(format!(
                "CLI binary '{}' for agent '{}' is not installed or not found on PATH",
                self.binary_path.display(),
                self.id
            ))
        })?;

        let mut cmd = tokio::process::Command::new(&resolved_bin);
        cmd.current_dir(cwd);
        self.headless.apply(&mut cmd, instruction);
        Ok(cmd)
    }

    pub fn resolve_binary(&self) -> Option<PathBuf> {
        super::binary::resolve_binary_path(self.binary_path.to_str().unwrap_or(""))
    }
}

#[async_trait]
impl AgentRuntimePort for CliHarness {
    fn harness_id(&self) -> &str {
        &self.id
    }

    fn profile(&self) -> HarnessProfile {
        HarnessProfile {
            harness_id: self.id.clone(),
            tool_mediation: self.mediation_level,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: false,
            cost_visibility: self.cost_visibility,
        }
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let profile = self.profile();
        let effective_run_id = worker_run
            .run_id
            .clone()
            .unwrap_or_else(|| worker_run.id.clone());

        let mut prompt = String::new();
        prompt.push_str(&format!("Goal for task {}: \n", worker_run.task_id));

        // Inject any pending steer guidance
        {
            let mut steer_map = self.steer_messages.lock().await;
            if let Some(steer_notes) = steer_map.remove(&effective_run_id) {
                prompt.push_str("\n--- Steering Guidance ---\n");
                for note in steer_notes {
                    prompt.push_str(&format!("- {}\n", note));
                }
                prompt.push('\n');
            }
        }

        for item in &context_pack.items {
            prompt.push_str(&item.content);
            prompt.push('\n');
        }

        struct TurnObserver {
            run_id: String,
        }
        impl custos_core::contracts::harness::RunObserver for TurnObserver {
            fn emit(&self, _kind: &str, _payload: serde_json::Value) {}
            fn is_cancelled(&self) -> bool {
                false
            }
            fn run_id(&self) -> Option<&str> {
                Some(&self.run_id)
            }
        }

        let observer = Arc::new(TurnObserver {
            run_id: effective_run_id.clone(),
        });

        let res = self
            .run_native_observed(&prompt, &self.workspace_root, observer)
            .await?;

        let mut effects = res.observed_effects;
        for intent in &mut effects {
            intent.task_id = Some(worker_run.task_id.clone());
            profile.validate_intent_assurance(intent)?;
        }
        Ok(effects)
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        let active = self.active_runs.lock().await;
        if let Some(flag) = active.get(run_id) {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        let mut steer = self.steer_messages.lock().await;
        steer
            .entry(run_id.to_string())
            .or_default()
            .push(guidance.to_string());
        Ok(())
    }

    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        observer: std::sync::Arc<dyn custos_core::contracts::harness::RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        #[cfg(any(test, feature = "test-support"))]
        if self.mock_state.enabled {
            let default_output = if self.id == "claude-code" {
                "[tool_call:bash] cargo check\nWriting file: src/lib.rs\nDone".to_string()
            } else {
                format!("{}: processed task '{instruction}'", self.id)
            };
            let output = self.mock_state.next_response(&default_output).await;
            let observed_effects = self.normalize_output_to_intents("mock_task", &output);
            return Ok(HarnessExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: output,
                stderr: String::new(),
                observed_effects,
                mediation_level: self.mediation_level,
            });
        }

        let cmd = self.build_command(instruction, cwd)?;
        let run_id = observer.run_id().unwrap_or("cli-observed").to_string();
        let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let mut active = self.active_runs.lock().await;
            active.insert(run_id.clone(), cancel_flag.clone());
        }

        let wrapped_observer = Arc::new(super::observer::CancellableObserver::new(
            observer,
            cancel_flag,
        ));
        let res = super::stream::run_observed_process_with_protocol(
            cmd,
            wrapped_observer,
            self.protocol,
            self.default_timeout,
        )
        .await;

        {
            let mut active = self.active_runs.lock().await;
            active.remove(&run_id);
        }
        res
    }

    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.run_native_observed(
            instruction,
            cwd,
            std::sync::Arc::new(super::observer::NoopObserver),
        )
        .await
    }
}

pub type GenericCliHarnessAdapter = CliHarness;
pub type CodexHarnessAdapter = CliHarness;
pub type GooseHarnessAdapter = CliHarness;

/// Transparent compatibility adapter preserving `ClaudeCodeHarnessAdapter::new(workspace_root)`.
#[derive(Clone)]
pub struct ClaudeCodeHarnessAdapter(pub CliHarness);

impl ClaudeCodeHarnessAdapter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self(CliHarness::claude_code(workspace_root))
    }
}

impl std::ops::Deref for ClaudeCodeHarnessAdapter {
    type Target = CliHarness;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ClaudeCodeHarnessAdapter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl AgentRuntimePort for ClaudeCodeHarnessAdapter {
    fn harness_id(&self) -> &str {
        self.0.harness_id()
    }
    fn profile(&self) -> HarnessProfile {
        self.0.profile()
    }
    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        self.0.execute_turn(worker_run, context_pack).await
    }
    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        self.0.cancel_run(run_id).await
    }
    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        self.0.steer_run(run_id, guidance).await
    }
    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        observer: std::sync::Arc<dyn custos_core::contracts::harness::RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.0.run_native_observed(instruction, cwd, observer).await
    }
    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.0.run_native(instruction, cwd).await
    }
}

/// Transparent compatibility adapter preserving `OpenCodeHarnessAdapter::new(workspace_root)`.
#[derive(Clone)]
pub struct OpenCodeHarnessAdapter(pub CliHarness);

impl OpenCodeHarnessAdapter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self(CliHarness::opencode(workspace_root))
    }
}

impl std::ops::Deref for OpenCodeHarnessAdapter {
    type Target = CliHarness;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OpenCodeHarnessAdapter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl AgentRuntimePort for OpenCodeHarnessAdapter {
    fn harness_id(&self) -> &str {
        self.0.harness_id()
    }
    fn profile(&self) -> HarnessProfile {
        self.0.profile()
    }
    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        self.0.execute_turn(worker_run, context_pack).await
    }
    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        self.0.cancel_run(run_id).await
    }
    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        self.0.steer_run(run_id, guidance).await
    }
    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        observer: std::sync::Arc<dyn custos_core::contracts::harness::RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.0.run_native_observed(instruction, cwd, observer).await
    }
    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.0.run_native(instruction, cwd).await
    }
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
