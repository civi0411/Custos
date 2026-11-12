//! Agent Runtime / Native Harness Port (Port 7)
//!
//! Standardized contract governing native coding agents (Claude Code, Codex, Goose)
//! and governed agent runtimes under RFC 002.
//!
//! Enforces Gate 4 (Native Bypass Assurance): External/native harnesses that run their
//! own tools cannot fraudulently claim `custos-mediated` assurance.

use async_trait::async_trait;
use std::path::Path;

use custos_domain::{ActionIntent, ContextPack, DomainError, WorkerRun};
pub use custos_domain::{
    CostVisibility, HarnessDescriptor, HarnessExecutionResult, HarnessMaturity, HarnessProfile,
    ToolMediationLevel, WorktreeOwnership,
};

/// Agent Runtime Port (Deep Dissection of Agent Loop & Native Harnesses)
///
/// Dissects the legacy upstream execution loop into a strictly governed contract.
/// Concrete agent engines (Goose loops, Claude Code / Codex wrappers) implement this trait.
#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    /// Unique identifier of this harness (e.g. "claude-code", "codex", "governed-agent").
    fn harness_id(&self) -> &str;

    /// Honest capability and assurance profile (RFC 002).
    fn profile(&self) -> HarnessProfile;

    /// Executes a single agent step or full worker turn under the governed WorkerRun contract.
    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError>;

    async fn run_native_observed(
        &self,
        instruction: &str,
        cwd: &Path,
        _observer: std::sync::Arc<dyn RunObserver>,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.run_native(instruction, cwd).await
    }

    /// Signals cancellation to an ongoing agent run.
    async fn cancel_run(&self, _run_id: &str) -> Result<(), DomainError> {
        if !self.profile().supports_cancel {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support cancellation",
                self.harness_id()
            )));
        }
        Ok(())
    }

    /// Injects steering guidance/feedback to an ongoing agent run.
    async fn steer_run(&self, _run_id: &str, _guidance: &str) -> Result<(), DomainError> {
        if !self.profile().supports_steer {
            return Err(DomainError::Validation(format!(
                "Harness '{}' does not support steering",
                self.harness_id()
            )));
        }
        Ok(())
    }

    /// Executes a native subprocess instruction with assurance tracking.
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

/// Transport-neutral, non-authoritative presentation observer.
pub trait RunObserver: Send + Sync {
    fn emit(&self, kind: &str, payload: serde_json::Value);
    fn is_cancelled(&self) -> bool;
    fn run_id(&self) -> Option<&str> {
        None
    }
}
