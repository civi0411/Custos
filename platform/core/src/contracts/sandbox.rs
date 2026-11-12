//! Port 3 — SandboxPort
//!
//! Executes ONE permitted command inside an OS sandbox (Seatbelt / Bubblewrap).
//! Implementations MUST call [`verify_permit_binding`] before spawning anything.
//!
//! Note: For in-memory workspace path traversal containment (`PathSandbox`),
//! see `custos_core::authority::PathSandbox`.

use async_trait::async_trait;
use custos_domain::{ActionIntent, DomainError, ExecutionReceipt, Permit};
use serde::{Deserialize, Serialize};

/// Typed view of `ActionIntent.parameters` for exec-style actions:
/// `{"program": "cargo", "args": ["test"], "cwd": "."}`
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

/// INV-04 guard run by every SandboxPort driver. Flow:
/// `KernelPort::consume_permit` burns the permit (single-use) -> the burned permit is passed
/// here -> driver verifies it is bound to THIS task/action/arguments and not expired.
/// (Single-use itself is the Kernel's job; checking `used_at` here would always fail.)
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

#[async_trait]
pub trait SandboxPort: Send + Sync {
    /// Stable driver name, e.g. "seatbelt", "bubblewrap", "mock".
    fn driver_id(&self) -> &str;

    /// Probes runtime security and execution capabilities supported by this driver.
    fn probe_capabilities(&self) -> Vec<String> {
        vec![self.driver_id().to_string()]
    }

    async fn execute(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError>;
}

/// Captured result from executing a bounded shell command inside a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Host port for executing workspace verification commands (e.g., candidate race test oracles).
#[async_trait]
pub trait CommandRunnerPort: Send + Sync {
    async fn run_shell(
        &self,
        command: &str,
        cwd: &std::path::Path,
        timeout_ms: u64,
    ) -> Result<CommandExecutionResult, DomainError>;
}
