//! Custos Core Domain
//!
//! Pure, deterministic domain models with ZERO side effects and ZERO external framework dependencies.
//! All models are serializable and validate state machine transitions strictly.

pub mod action;
pub mod annotation;
pub mod approval;
pub mod artifact;
pub mod authority;
pub mod budget;
pub mod capability;
pub mod claim;
pub mod context;
pub mod continuation;
pub mod decision;
pub mod error;
pub mod evidence;
pub mod execution_record;
pub mod fact;
pub mod ids;
pub mod memory;
pub mod oi;
pub mod packet;
pub mod provider_config;
pub mod recipe;
pub mod repo;
pub mod run;
pub mod session;
pub mod span;
pub mod task;
pub mod types;
pub mod workflow;
pub mod workspace;

// Explicit re-exports of public domain surface (No glob *)
pub use action::{
    Action, ActionIntent, ActionIntentV1, ActionLifecycleState, Assurance, EffectAttempt,
    EffectStatus, RiskLevel,
};
pub use annotation::{AnnotationRecord, AnnotationStatus, AnnotationTarget};
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use artifact::{ArtifactKind, ArtifactRef};
pub use authority::{
    ExecutionPermit, ExecutionReceipt, Grant, Permit, PermitId, PermitV1, Receipt, ReceiptStatus,
    RiskClass,
};
pub use budget::{Budget, Headroom, ReservationToken};
pub use capability::CapabilityManifest;
pub use claim::{
    ArtifactLineageNode, Claim, ClaimEvidenceLink, ClaimGroundingLevel, EnvSnapshot,
    EvidenceRelation, PassageAnchor, ResearchClaim, ResearchExperimentRun, SourceRecord,
};
pub use context::{ContextItem, ContextPack};
pub use continuation::ContinuationPacket;
pub use decision::{
    Candidate, DecisionRecord, DecisionSnapshot, ExecutionTopology, NodePlacement, RejectionReason,
    ReplanBrief, ReplanRecord, ReplanTrigger, StrategyProposal, WorkPacket, WorkerResult,
    WorkerStatus,
};
pub use error::DomainError;
pub use evidence::{
    EvidenceRecord, EvidenceRecordV1, EvidenceRequirement, EvidenceStatus, VerificationClaim,
};
pub use execution_record::{ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus};
pub use fact::Fact;
pub use ids::{canonical_json, digest, new_id, TaskId};
pub use memory::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
pub use packet::ContinuationPacket as PacketContinuation;
pub use provider_config::{ClientApiKeyRecord, ProviderConfig};
pub use recipe::{EnvironmentSpec, Recipe, RecipeInput};
pub use repo::{
    ConfidenceLevel, CoverageMetrics, MissingReason, QueryResult, RepoSnapshotRef, SourceSpan,
    SymbolMatch,
};
pub use run::{
    CancelReceipt, ClaimStatus, DispatchClaim, NodeAttempt, Run, RunHandle, RunStatus,
    StartRunCommand, WorkerRun,
};
pub use session::{Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus};
pub use span::{Span, SpanState};
pub use task::{
    ContractEvidence, CriterionSpec, EvidenceKind, Task, TaskContract, TaskContractRevision,
    TaskContractV1, TaskRevision, TaskStatus,
};
pub use workflow::{
    RevisionNode, WorkflowIR, WorkflowNodeIR, WorkflowPlan, WorkflowRevision, WorkflowStep,
};
pub use workspace::{
    ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage, WorkspaceStatus,
};
