//! Port 5 — WorkflowPort
//!
//! Drives a Task through bounded workflow runs: start, cancel, checkpoint/resume.
//! Defined here so the Kernel/Bridge/Daemon can depend on it; `custos-runtime/workflow` implements it.

use async_trait::async_trait;
use custos_domain::{CancelReceipt, ContinuationPacket, DomainError, RunHandle, StartRunCommand};

#[async_trait]
pub trait WorkflowPort: Send + Sync {
    /// Starts a run matching a pre-compiled workflow revision or task.
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError>;

    /// Opt-in background dispatch. Unsupported runtimes fail explicitly.
    async fn start_run_background(&self, _cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        Err(DomainError::Validation("Background execution is not supported".into()))
    }

    async fn live_run(&self, _session_id: &str, _run_id: &str, _cursor: u64) -> Result<custos_domain::LiveRunReplay, DomainError> {
        Err(DomainError::Validation("Live run projection is not supported".into()))
    }

    /// Cooperative cancel; MUST leave in-flight effects as `Uncertain`, never silently dropped.
    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError>;

    /// Resumes a halted or crashed run. Validates source, approval, and effect states first.
    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError>;

    /// Persist a resumable packet (CheckpointPolicy, Custos.md §3.4).
    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError>;
}
