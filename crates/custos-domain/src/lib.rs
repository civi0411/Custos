//! Custos Core Domain
//!
//! Pure, deterministic domain models with ZERO side effects and ZERO external framework dependencies.
//! Organized by Bounded Contexts:
//! - `task`: Task, Run, Attempt, Continuation, Workflow, Recipe
//! - `session`: Session, Assistant, Automation
//! - `authority`: Action, Permit, Approval, Capability
//! - `evidence`: Evidence, Claim, Fact, Synthesis, Review, ExecutionRecord, ExperimentAttempt
//! - `context`: ContextPack, Memory, Annotation
//! - `workspace`: Workspace, WorkspaceFiles, Artifact, Terminal, Repo
//! - `research`: Notebook, Note
//! - `fleet`: Compute, RemoteFleet, Browser, ProviderConfig
//! - `shared`: IDs, Error, Types, Budget, Span, Packet
//! - `oi`: Orchestration Intelligence (Topologies, Decision)

pub mod authority;
pub mod context;
pub mod evidence;
pub mod fleet;
pub mod oi;
pub mod pipeline;
pub mod quant;
pub mod research;
pub mod session;
pub mod shared;
pub mod task;
pub mod workspace;

// Backward-compatible module paths for existing callers
pub use authority::action;
pub use authority::approval;
pub use authority::capability;
pub use context::annotation;
pub use context::memory;
pub use evidence::claim;
pub use evidence::execution_record;
pub use evidence::experiment_attempt;
pub use evidence::fact;
pub use evidence::review;
pub use evidence::synthesis;
pub use fleet::browser;
pub use fleet::compute;
pub use fleet::provider_config;
pub use fleet::remote_fleet;
pub use quant::backtest;
pub use quant::factor;
pub use quant::order;
pub use quant::risk;
pub use research::experiment;
pub use research::note;
pub use research::notebook;
pub use session::assistant;
pub use session::automation;
pub use shared::budget;
pub use shared::error;
pub use shared::ids;
pub use shared::packages;
pub use shared::packet;
pub use shared::span;
pub use shared::types;
pub use task::continuation;
pub use task::decision;
pub use task::recipe;
pub use task::run;
pub use task::workflow;
pub use workspace::artifact;
pub use workspace::repo;
pub use workspace::terminal;
pub use workspace::workspace_files;

// Explicit re-exports of public domain surface (No glob *)
pub use action::{
    Action, ActionIntent, ActionIntentV1, ActionLifecycleState, Assurance, EffectAttempt,
    EffectStatus, RiskLevel,
};
pub use annotation::{AnnotationRecord, AnnotationStatus, AnnotationTarget};
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use artifact::{
    ArtifactHandoff, ArtifactKind, ArtifactLineageGraph, ArtifactRef, ArtifactSummary,
    HandoffConsent, LineageGraphEdge, LineageGraphNode,
};
pub use assistant::{
    AssistantDraft, OutboxActionItem, OutboxDeliveryState, ResourceActionCapability, StandingGrant,
};
pub use authority::{
    ExecutionPermit, ExecutionReceipt, Grant, Permit, PermitId, PermitV1, Receipt, ReceiptStatus,
    RiskClass,
};
pub use automation::{
    CreateHeadlessJobParams, HeadlessAutomationJob, HeadlessJobStatus, HeadlessTaskSpec,
    HeadlessTrigger, RunHeadlessJobResult,
};
pub use browser::{
    validate_browser_url, BrowserConsoleEntry, BrowserNetworkRequest, BrowserPageSnapshot,
    BrowserSession, BrowserTab, BrowserTabStatus, CaptureSnapshotParams, CreateTabParams,
    NavigateTabParams,
};
pub use budget::{Budget, Headroom, ReservationToken};
pub use capability::{CapabilityDescriptor, CapabilityGroup, CapabilityManifest, CapabilityStatus};
pub use claim::{
    ArtifactLineageNode, AssessmentStatus, ClaimAssessmentRecord, ClaimEvidenceLink,
    ClaimGroundingLevel, EntailmentVerdict, EnvSnapshot, EvidenceRelation, PassageAnchor,
    PassageAnchorProposal, ResearchClaim, ResearchClaimProposal, ResearchExperimentRun,
    SourceProposal, SourceRecord,
};
pub use compute::{
    AcceleratorSpec, CheckpointCommitStatus, CheckpointFileEntry, CloudPodLease, CloudPodStatus,
    CompatibilityGateEvaluation, CompatibilityVerdict, ComputeBackendKind, ComputeDeviceRef,
    ComputeWorkloadKind, DeviceKind, HardwareAdaptationPlan, OptionalConsumerPlacement,
    ResumabilityLevel, TrainingCheckpointManifest, WorkloadAdmissionRequest,
    WorkloadAdmissionVerdict, WorkloadLease, WorkloadLeaseStatus,
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
pub use execution_record::{ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus};
pub use experiment_attempt::{
    classify, protection_status, Check, CheckGroup, CheckStatus, ExecEvent, ExecState,
    ExecutionAttempt, ExternalJobRef, Invocation, InvocationOutcome, ProgressTriplet,
    ProtectionPolicy, ProtectionStatus, ResourceEvent, ResourceState, ResumeVerdict, TargetCaps,
    VerificationReport,
};
pub use fact::Fact;
pub use ids::{
    canonical_json, digest, new_id, HarnessId, JobId, PaneId, TaskId, WorkflowNodeId,
};
pub use memory::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
pub use note::{NoteRecord, NoteVersionRecord};
pub use notebook::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, KernelStatus, NotebookCell,
    NotebookCellType, NotebookDocument, NotebookKernelBinding, NotebookKernelState,
};
pub use packet::ContinuationPacket as PacketContinuation;
pub use provider_config::{
    ClientApiKeyRecord, LoadedModelInstance, LocalHardwareProfile, LocalModelDescriptor,
    LocalModelFolder, LocalModelLoadProfile, LocalModelLoadRequest, LocalModelPreflightRequest,
    LocalModelPreflightResponse, ModelCatalogOption, ModelCatalogResult, ModelCompatibility,
    ModelPricing, OAuthTokenRecord, ProbedModel, ProviderConfig,
};
pub use experiment::{
    FrozenTrainingRecipe, ModelEvaluationReport, ModelEvaluationSpec, TrainingExecutionReceipt,
    TrainingStepTelemetry,
};
pub use pipeline::{
    AstSyntaxInvariant, ClosedBarInvariant, DataInvariant, DataModality, EvidenceGroundingInvariant,
    GovernedOutboxInvariant, PipelineRecord, PipelineStage,
};
pub use quant::{
    AlphaFactor, BacktestTearsheet, MonthlyReturn, OhlcvBar, OrderDirection, OrderTicket,
    OrderTicketStatus, OrderType, RiskBreach, RiskPermitConfig,
};
pub use recipe::{EnvironmentSpec, Recipe, RecipeInput};
pub use remote_fleet::{
    FleetExecParams, FleetExecReceipt, FleetExecResult, RegisterHostParams, RemoteHostNode,
    RemoteHostStatus, SshAuthMethod,
};
pub use repo::SourceSpan;
pub use review::{
    FindingSeverity, RecordReviewParams, ReviewFinding, ReviewMethod, ReviewStatus,
    ReviewTargetType, ReviewerRecord,
};
pub use run::{
    CancelReceipt, ClaimStatus, DispatchClaim, LaunchAttempt, LaunchStatus, NodeAttempt,
    ReconciliationSummary, Run, RunHandle, RunStatus, StartRunCommand, UsageExecutorKind,
    UsageMeasurement, UsageRecord, WorkerRun,
};
pub use run::{LiveRunEvent, LiveRunReplay, LiveRunSnapshot};
pub use session::{
    ConversationTurn, Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus,
    SessionTaskBinding, TurnActorKind, WorkbenchLens,
};
pub use span::{Span, SpanState};
pub use synthesis::{
    ClaimHandoffSummary, HandoffToCodingParams, HandoffToCodingResult, RecipeHandoffSummary,
    ResearchSynthesisProposal, SaveSynthesisProposalParams, SynthesisProposalStatus,
};
pub use task::{
    ContractEvidence, CriterionSpec, EvidenceKind, Task, TaskContract, TaskContractRevision,
    TaskContractV1, TaskRevision, TaskStatus,
};
pub use terminal::{
    TerminalOutputChunk, TerminalSession, TerminalSessionId, TerminalSessionStatus,
};
pub use workflow::{
    CandidateExecutionReport, CandidateRaceResult, CandidateRaceSpec, ModelCandidateConfig,
    RevisionNode, WorkflowIR, WorkflowNodeIR, WorkflowPlan, WorkflowRevision, WorkflowStep,
};
pub use workspace::{
    DirtyManifest, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage,
    WorkspaceStatus,
};
pub use workspace_files::{
    WorkspaceDiffEntry, WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff,
    WorkspaceFileEntry, WorkspaceFileTree, WriteWorkspaceFileParams,
};
pub use types::{truncate_str_to_char_boundary, truncate_to_char_boundary};
