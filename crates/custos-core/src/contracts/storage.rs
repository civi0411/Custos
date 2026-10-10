//! Port 2 — StoragePort (+ CasPort, OutboxPort)
//!
//! Traits live in core; `custos-persistence` implements them. `StoragePort` is a pure
//! alias-trait over the two persistence traits that already exist, so current code keeps
//! compiling while consumers can depend on one name.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_domain::{
    ArtifactRef, DecisionRecord, DispatchClaim, DomainError, ExecutionReceipt, LaunchAttempt,
    NodePlacement, ReplanRecord, Run, WorkerRun, WorkflowRevision,
};
use serde::{Deserialize, Serialize};

use crate::kernel::{SessionStore, TaskStore};

/// Tasks, spans, continuations, sessions, runs, and journals (Custos.md §3, T1–T5).
pub trait StoragePort:
    TaskStore + SessionStore + RunPort + DecisionPort + WorkflowRevisionPort + ReplanPort
{
}
impl<
        T: TaskStore
            + SessionStore
            + RunPort
            + DecisionPort
            + WorkflowRevisionPort
            + ReplanPort
            + ?Sized,
    > StoragePort for T
{
}

/// Persistence Port for sovereign Runs and bounded WorkerRuns.
#[async_trait]
pub trait RunPort: Send + Sync {
    async fn save_run(&self, run: &Run) -> Result<(), DomainError>;
    async fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError>;
    async fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError>;
    async fn save_worker_run(&self, wrun: &WorkerRun) -> Result<(), DomainError>;
    async fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError>;
    async fn list_worker_runs_for_task(&self, task_id: &str)
        -> Result<Vec<WorkerRun>, DomainError>;
    async fn list_worker_runs_for_run(&self, run_id: &str) -> Result<Vec<WorkerRun>, DomainError>;
    async fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError>;
    async fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError>;
    async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError>;
    async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError>;
    async fn save_launch_attempt(&self, attempt: &LaunchAttempt) -> Result<(), DomainError>;
    async fn get_launch_attempt(&self, attempt_id: &str) -> Result<Option<LaunchAttempt>, DomainError>;
    async fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        Ok(0)
    }
}

/// Persistence Port for Orchestration Intelligence Decision Records (RFC 003).
#[async_trait]
pub trait DecisionPort: Send + Sync {
    async fn record_decision(&self, record: &DecisionRecord) -> Result<(), DomainError>;
    async fn get_decision(&self, id: &str) -> Result<Option<DecisionRecord>, DomainError>;
    async fn list_decisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<DecisionRecord>, DomainError>;
}

/// Persistence Port for Workflow Revisions and Node Placements (RFC 004).
#[async_trait]
pub trait WorkflowRevisionPort: Send + Sync {
    async fn save_revision(
        &self,
        revision: &WorkflowRevision,
        placements: &[NodePlacement],
    ) -> Result<(), DomainError>;
    async fn get_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<WorkflowRevision>, DomainError>;
    async fn list_revisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkflowRevision>, DomainError>;
    async fn get_placements_for_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<NodePlacement>, DomainError>;
}

/// Persistence Port for Replan Records (RFC 004 / RFC 005).
#[async_trait]
pub trait ReplanPort: Send + Sync {
    async fn record_replan(&self, record: &ReplanRecord) -> Result<(), DomainError>;
    async fn get_replan(&self, id: &str) -> Result<Option<ReplanRecord>, DomainError>;
    async fn list_replans_for_task(&self, task_id: &str) -> Result<Vec<ReplanRecord>, DomainError>;
}

/// Content-addressed artifact store (`.custos/cas/`). Hash = SHA-256 of bytes.
#[async_trait]
pub trait CasPort: Send + Sync {
    async fn put(&self, data: &[u8], mime: Option<String>) -> Result<ArtifactRef, DomainError>;
    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, DomainError>;
    async fn exists(&self, hash: &str) -> Result<bool, DomainError>;
}

/// Crash-window states from Custos.md §4.3. `Uncertain` must NEVER be blind-retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    Pending,
    Dispatching,
    Receipted,
    Uncertain,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub permit_id: String,
    pub argument_digest: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub status: OutboxStatus,
    pub created_at: DateTime<Utc>,
    pub receipt: Option<ExecutionReceipt>,
}

/// Transactional outbox: the durable record written in T3 BEFORE any external effect.
#[async_trait]
pub trait OutboxPort: Send + Sync {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError>;
    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError>;
    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError>;
    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError>;
    /// Used by the reconciliation protocol on daemon restart.
    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError>;
    /// List outbox entries filtered by task_id and status (Fix G6).
    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError>;
}

/// Durable effect ledger for recording and looking up effect attempts (Gate 3).
#[async_trait]
pub trait EffectLedgerPort: Send + Sync {
    async fn record_effect(&self, effect: &custos_domain::EffectAttempt)
        -> Result<(), DomainError>;
    async fn update_effect_status(
        &self,
        id: &str,
        status: custos_domain::EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError>;
    async fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<custos_domain::EffectAttempt>, DomainError>;
    async fn reconcile_on_startup(&self) -> Result<usize, DomainError>;
}
