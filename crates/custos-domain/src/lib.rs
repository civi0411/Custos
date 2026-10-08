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
pub mod decision;
pub mod error;
pub mod evidence;
pub mod fact;
pub mod ids;
pub mod memory;
pub mod oi;
pub mod packet;
pub mod repo;
pub mod run;
pub mod annotation;
pub mod execution_record;
pub mod provider_config;
pub mod recipe;
pub mod session;
pub mod span;
pub mod task;
pub mod terminal;
pub mod types;
pub mod workflow;
pub mod workspace;

// Explicit re-exports of public domain surface (No glob *)
pub use annotation::{AnnotationRecord, AnnotationStatus, AnnotationTarget};
pub use execution_record::{ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus};
pub use provider_config::{ClientApiKeyRecord, ProviderConfig};
pub use recipe::{EnvironmentSpec, Recipe, RecipeInput};
pub use action::{
    Action, ActionIntent, ActionIntentV1, ActionLifecycleState, Assurance, EffectAttempt,
    EffectStatus, RiskLevel,
};
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use artifact::{ArtifactHandoff, ArtifactKind, ArtifactRef, HandoffConsent};
pub use authority::{
    ExecutionPermit, ExecutionReceipt, Grant, Permit, PermitId, PermitV1, Receipt, ReceiptStatus,
    RiskClass,
};
pub use budget::{Budget, Headroom, ReservationToken};
pub use capability::{CapabilityDescriptor, CapabilityGroup, CapabilityManifest, CapabilityStatus};
pub use claim::{
    ArtifactLineageNode, Claim, ClaimEvidenceLink, ClaimGroundingLevel, EnvSnapshot,
    EvidenceRelation, PassageAnchor, PassageAnchorProposal, ResearchClaim, ResearchClaimProposal,
    ResearchExperimentRun, SourceProposal, SourceRecord,
};
pub use context::{
    ContextDeliveryState, ContextItem, ContextPack, ContextReceipt, OmittedContextReason,
    OmittedContextRef,
};
pub use continuation::{
    ContinuationDisplayScope, ContinuationManifest, ContinuationPacket, ContinuationState,
    RequestedModelScope, TransferPrivacyDecision,
};
pub use decision::{
    Candidate, DecisionRecord, DecisionSnapshot, ExecutionTopology, NodePlacement, RejectionReason,
    ReplanBrief, ReplanRecord, ReplanTrigger, StrategyProposal, WorkPacket, WorkerResult,
    WorkerStatus,
};
pub use error::DomainError;
pub use evidence::{
    CriterionVerificationRecord, EvidenceRecord, EvidenceRecordV1, EvidenceRequirement,
    EvidenceStatus, OutcomeStatus, TaskOutcome, VerificationClaim,
};
pub use fact::Fact;
pub use ids::{canonical_json, digest, new_id, TaskId};
pub use memory::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
pub use packet::ContinuationPacket as PacketContinuation;
pub use repo::{
    ConfidenceLevel, CoverageMetrics, MissingReason, QueryResult, RepoSnapshotRef, SourceSpan,
    SymbolMatch,
};
pub use run::{
    CancelReceipt, ClaimStatus, DispatchClaim, LaunchAttempt, LaunchStatus, NodeAttempt, Run,
    RunHandle, RunStatus, StartRunCommand, UsageExecutorKind, UsageMeasurement, UsageRecord,
    WorkerRun,
};
pub use session::{
    ConversationTurn, Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus,
    SessionTaskBinding, TurnActorKind, WorkbenchLens,
};
pub use span::{Span, SpanState};
pub use task::{
    ContractEvidence, CriterionSpec, EvidenceKind, Task, TaskContract, TaskContractRevision,
    TaskContractV1, TaskRevision, TaskStatus,
};
pub use terminal::{
    TerminalOutputChunk, TerminalSession, TerminalSessionId, TerminalSessionStatus,
};
pub use workflow::{
    RevisionNode, WorkflowIR, WorkflowNodeIR, WorkflowPlan, WorkflowRevision, WorkflowStep,
};
pub use workspace::{
    DirtyManifest, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage,
    WorkspaceStatus,
};

