//! Foundation Shared Block (Domain Layer)
//!
//! Provides cross-cutting primitives and shared domain value objects:
//! canonical newtype IDs, DomainError, budget management, execution spans, and packet streaming.

pub use crate::shared::budget::{Budget, Headroom, ReservationToken};
pub use crate::shared::error::DomainError;
pub use crate::shared::ids::{
    canonical_json, digest, new_id, HarnessId, JobId, PaneId, TaskId, WorkflowNodeId,
};
pub use crate::shared::packet::ContinuationPacket;
pub use crate::shared::span::{Span, SpanState};
pub use crate::shared::types::{truncate_str_to_char_boundary, truncate_to_char_boundary};
pub use crate::task::task::{
    ContractEvidence, CriterionSpec, EvidenceKind, Task, TaskContract, TaskRevision, TaskStatus,
};
