//! Port 5 — WorkflowPort
//!
//! Drives a Task through bounded worker runs: start, cancel, checkpoint/resume.
//! Defined here so the Kernel/Bridge can depend on it; `custos-runtime/workflow` implements it.
//!
//! STATUS: DRAFT — signatures are a proposal for Vinh to review via RFC before freeze.

use async_trait::async_trait;
use custos_domain::{ContinuationPacket, DomainError, WorkerRun};

#[async_trait]
pub trait WorkflowPort: Send + Sync {
    /// Begin (or resume from the latest checkpoint) a bounded worker run for the task.
    async fn start_run(&self, task_id: &str) -> Result<WorkerRun, DomainError>;

    /// Cooperative cancel; MUST leave in-flight effects as `Uncertain`, never silently dropped.
    async fn cancel(&self, task_id: &str, reason: &str) -> Result<(), DomainError>;

    /// Persist a resumable packet (CheckpointPolicy, Custos.md §3.4).
    async fn checkpoint(&self, task_id: &str) -> Result<ContinuationPacket, DomainError>;
}
