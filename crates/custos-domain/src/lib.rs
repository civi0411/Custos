//! Custos Core Domain
//!
//! Pure, deterministic domain models with ZERO side effects and ZERO external framework dependencies.
//! All models are serializable and validate state machine transitions strictly.

pub mod action;
pub mod approval;
pub mod artifact;
pub mod authority;
pub mod budget;
pub mod capability;
pub mod claim;
pub mod context;
pub mod continuation;
pub mod error;
pub mod evidence;
pub mod fact;
pub mod ids;
pub mod memory;
pub mod packet;
pub mod run;
pub mod session;
pub mod span;
pub mod task;
pub mod types;
pub mod workflow;

// Explicit re-exports of public domain surface (No glob *)
pub use action::{Action, ActionIntent, ActionIntentV1, ActionLifecycleState, RiskLevel};
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use artifact::{ArtifactKind, ArtifactRef};
pub use authority::{
    ExecutionPermit, ExecutionReceipt, Grant, Permit, PermitId, PermitV1, Receipt, ReceiptStatus,
    RiskClass,
};
pub use budget::{Budget, Headroom, ReservationToken};
pub use capability::CapabilityManifest;
pub use claim::Claim;
pub use context::{ContextItem, ContextPack};
pub use continuation::ContinuationPacket;
pub use error::DomainError;
pub use evidence::{
    EvidenceRecord, EvidenceRecordV1, EvidenceRequirement, EvidenceStatus, VerificationClaim,
};
pub use fact::Fact;
pub use ids::{canonical_json, digest, new_id, TaskId};
pub use memory::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
pub use packet::ContinuationPacket as PacketContinuation;
pub use run::{Run, RunStatus, WorkerRun};
pub use session::{Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus};
pub use span::{Span, SpanState};
pub use task::{ContractEvidence, EvidenceKind, Task, TaskContract, TaskContractV1, TaskRevision, TaskStatus};
pub use workflow::{WorkflowIR, WorkflowNodeIR, WorkflowPlan, WorkflowStep};
