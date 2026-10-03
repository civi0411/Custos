//! Port 6 — MemoryPort (Custos.md §9.3)
//!
//! Read freely, PROPOSE only. Destructive operations (delete/purge/override retention)
//! are intentionally absent: only the Kernel may do those, on direct user command.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_domain::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    RecallQuery, WorkerRunId,
};

#[async_trait]
pub trait MemoryPort: Send + Sync {
    async fn recall_context(
        &self,
        query: &RecallQuery,
        scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError>;

    async fn recall_temporal_fact(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError>;

    async fn propose_fact(
        &self,
        proposal: FactProposal,
        worker_run_id: WorkerRunId,
    ) -> Result<ProposalReceipt, MemoryError>;
}
