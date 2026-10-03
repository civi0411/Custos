//! Port 2 — StoragePort (+ CasPort, OutboxPort)
//!
//! Traits live in core; `custos-persistence` implements them. `StoragePort` is a pure
//! alias-trait over the two persistence traits that already exist, so current code keeps
//! compiling while consumers can depend on one name.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_domain::{ArtifactRef, DomainError, ExecutionReceipt};
use serde::{Deserialize, Serialize};

use crate::kernel::{SessionStore, TaskStore};

/// Tasks, spans, continuations, sessions and journals (Custos.md §3, T1–T5).
pub trait StoragePort: TaskStore + SessionStore {}
impl<T: TaskStore + SessionStore + ?Sized> StoragePort for T {}

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
}
