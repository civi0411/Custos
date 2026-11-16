//! Execution & Harness SPI Trait Ports

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use crate::identity::error::DomainError;
use crate::resources::workspace::WorkspaceId;
use crate::authority::{Action, ActionIntent, ExecutionReceipt, Permit};
use crate::context::ContextPack;
use crate::execution::harness::{
    HarnessExecutionResult, HarnessProfile,
};
use crate::execution::assurance::EnvironmentAssurance;
use crate::execution::spec::EnvironmentSpec;
use crate::execution::state::EnvironmentState;
use crate::task::run::WorkerRun;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. Agent Runtime / Driver Port (Port 7)
// ---------------------------------------------------------------------------

pub use AgentRuntimePort as AgentDriver;

#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    fn harness_id(&self) -> &str;
    fn profile(&self) -> HarnessProfile;

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError>;

    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        _observer: Arc<dyn RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.run_native(instruction, cwd).await
    }

    async fn cancel_run(&self, _run_id: &str) -> Result<(), DomainError> {
        if !self.profile().supports_cancel {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support cancellation",
                self.harness_id()
            )));
        }
        Ok(())
    }

    async fn steer_run(&self, _run_id: &str, _guidance: &str) -> Result<(), DomainError> {
        if !self.profile().supports_steer {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support steering",
                self.harness_id()
            )));
        }
        Ok(())
    }

    async fn run_native(
        &self,
        _instruction: &str,
        _cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        Err(DomainError::Validation(
            "Native subprocess execution not supported by this harness".into(),
        ))
    }
}

pub trait RunObserver: Send + Sync {
    fn emit(&self, kind: &str, payload: serde_json::Value);
    fn is_cancelled(&self) -> bool;
    fn run_id(&self) -> Option<&str> {
        None
    }
}

// ---------------------------------------------------------------------------
// 2. Sandbox Port (Port 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCommand {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
}

fn default_cwd() -> String {
    ".".into()
}

impl SandboxCommand {
    pub fn from_intent(intent: &ActionIntent) -> Result<Self, DomainError> {
        serde_json::from_value(intent.parameters.clone())
            .map_err(|e| DomainError::Validation(format!("invalid sandbox command: {e}")))
    }
}

pub fn verify_permit_binding(permit: &Permit, intent: &ActionIntent) -> Result<(), DomainError> {
    if permit.task_id != intent.task_id.clone().unwrap_or_default() {
        return Err(DomainError::Unauthorized(format!(
            "permit {} belongs to a different task",
            permit.id
        )));
    }
    if permit.action_id != intent.id {
        return Err(DomainError::Unauthorized(format!(
            "permit {} was issued for action {}, not {}",
            permit.id, permit.action_id, intent.id
        )));
    }
    let actual = intent.argument_digest();
    if !permit.argument_digest.is_empty() && permit.argument_digest != actual {
        return Err(DomainError::Unauthorized(format!(
            "argument digest mismatch for permit {}: bound {}, got {} (Confused Deputy Guard)",
            permit.id, permit.argument_digest, actual
        )));
    }
    if permit.idempotency_key.is_some() && permit.idempotency_key != intent.idempotency_key {
        return Err(DomainError::Unauthorized(format!(
            "idempotency key mismatch for permit {}: bound {:?}, got {:?}",
            permit.id, permit.idempotency_key, intent.idempotency_key
        )));
    }
    if chrono::Utc::now() > permit.expires_at {
        return Err(DomainError::Unauthorized(format!(
            "permit {} has expired",
            permit.id
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait SandboxPort: Send + Sync {
    async fn execute(
        &self,
        command: &SandboxCommand,
        permit: &Permit,
    ) -> Result<CommandExecutionResult, DomainError>;
}

#[async_trait]
pub trait CommandRunnerPort: Send + Sync {
    async fn run_command(
        &self,
        program: &str,
        args: &[String],
        cwd: &Path,
    ) -> Result<CommandExecutionResult, DomainError>;
}

// ---------------------------------------------------------------------------
// 3. Tool Gate Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub permit_id: Option<String>,
    pub evidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ExecutionReceipt>,
}

#[async_trait]
pub trait ToolGate: Send + Sync {
    async fn dispatch(&self, action: &Action) -> Result<ExecutionResult, DomainError>;

    async fn dispatch_for_task(
        &self,
        task_id: &str,
        action: &Action,
        actor: &str,
    ) -> Result<ExecutionResult, DomainError>;
}

// ---------------------------------------------------------------------------
// 4. Sovereign Agent Execution Environment (AEE) Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReport {
    pub env_id: String,
    pub passed: bool,
    pub state: EnvironmentState,
    pub assurance: EnvironmentAssurance,
    pub violations: Vec<String>,
}

impl GateReport {
    pub fn ok(
        env_id: impl Into<String>,
        state: EnvironmentState,
        assurance: EnvironmentAssurance,
    ) -> Self {
        Self {
            env_id: env_id.into(),
            passed: true,
            state,
            assurance,
            violations: Vec::new(),
        }
    }

    pub fn failed(
        env_id: impl Into<String>,
        state: EnvironmentState,
        assurance: EnvironmentAssurance,
        violations: Vec<String>,
    ) -> Self {
        Self {
            env_id: env_id.into(),
            passed: false,
            state,
            assurance,
            violations,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentHandle {
    pub env_id: String,
    pub workspace_id: WorkspaceId,
    pub spec: EnvironmentSpec,
    pub state: EnvironmentState,
    pub assurance: EnvironmentAssurance,
}

#[async_trait]
pub trait EnvironmentPort: Send + Sync {
    async fn provision(
        &self,
        workspace_id: &WorkspaceId,
        spec: &EnvironmentSpec,
    ) -> Result<EnvironmentHandle, DomainError>;

    async fn inspect(&self, env_id: &str) -> Result<EnvironmentHandle, DomainError>;
    async fn verify(&self, env_id: &str) -> Result<GateReport, DomainError>;
    async fn teardown(&self, env_id: &str) -> Result<(), DomainError>;
}

