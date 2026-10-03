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

    /// Cooperative cancel; MUST leave in-flight effects as `Uncertain`, never silently dropped.
    async fn request_cancel(&self, run_id: &str, reason: &str) -> Result<CancelReceipt, DomainError>;

    /// Resumes a halted or crashed run. Validates source, approval, and effect states first.
    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError>;

    /// Persist a resumable packet (CheckpointPolicy, Custos.md §3.4).
    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError>;
}
