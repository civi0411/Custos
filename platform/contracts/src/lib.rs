//! Custos Canonical Zero-I/O Domain Models, Contracts, and SPI Trait Ports

pub mod authority;
pub mod context;
pub mod contributions;
pub mod conversation;
pub mod evidence;
pub mod execution;
pub mod identity;
pub mod inference;
pub mod resources;
pub mod task;

// Bounded context aliases for workspace backward compatibility
pub use conversation as session;
pub use execution as environment;
pub use execution as fleet;
pub use identity as shared;
pub use inference as provider;
pub use resources as workspace;
pub use contributions as packages;
pub use task::continuation::ContinuationPacket as PacketContinuation;
pub use task::pipeline;
pub use task::oi;
pub use task::oi as decision;

// Internal module re-exports matching canonical domain layout
pub use authority::action;
pub use authority::approval;
pub use authority::budget;
pub use authority::capability;
pub use authority::outbox;
pub use authority::permit;
pub use context::annotation;
pub use context::memory;
pub use conversation::automation;
pub use evidence::execution_record;
pub use evidence::experiment_attempt;
pub use evidence::fact;
pub use evidence::review;
pub use execution::assurance;
pub use execution::browser;
pub use execution::harness;
pub use execution::lane;
pub use execution::notebook;
pub use execution::profile;
pub use execution::remote_fleet;
pub use execution::snapshot;
pub use execution::spec;
pub use execution::state;
pub use execution::terminal;
pub use identity::error;
pub use identity::ids;
pub use identity::types;
pub use inference::provider::config as provider_config;
pub use inference::provider::observation;
pub use resources::artifact;
pub use resources::compute;
pub use resources::notes;
pub use resources::repo;
pub use resources::workspace_files;
pub use task::continuation;
pub use task::recipe;
pub use task::run;
pub use task::span;
pub use task::workflow;

// --- Identity & Shared Types ---
pub use identity::{
    canonical_json, digest, new_id, truncate_str_to_char_boundary, truncate_to_char_boundary,
    DomainError, HarnessId, JobId, PaneId, TaskId, UniversalResourceRef, WorkflowNodeId,
};

// --- Task, Runs, Workflows & Supervision ---
pub use task::{
    compute_error_signature, default_rules, evaluate_snapshot, group_step_errors,
    CancelReceipt, Candidate, CandidateExecutionReport, CandidateRaceResult, CandidateRaceSpec,
    ClaimStatus, ContinuationDisplayScope, ContinuationManifest, ContinuationPacket,
    ContinuationState, ContractEvidence, CriterionSpec, DecisionRecord, DecisionSnapshot,
    DispatchClaim, ErrorGroupSummary, EvidenceKind, ExecutionTopology, FindingKind,
    KernelPort, LatencyBreakdown, LaunchAttempt, LaunchStatus, LiveRunEvent, LiveRunReplay,
    LiveRunSnapshot, LogSeverity, LongRunningRule, ModelCandidateConfig, NoActivityRule,
    NodeAttempt, NodePlacement, OiPlannerPort, PossibleLoopRule, Recipe, RecipeEnvironmentSpec,
    RecipeInput, ReconciliationSummary, RejectionReason, RepeatedErrorRule, RepeatedToolRule,
    ReplanBrief, ReplanRecord, ReplanTrigger, RequestedModelScope, RevisionNode, Run,
    RunHandle, RunInspectionReport, RunMetricsSummary, RunOverviewSummary, RunPort, RunStepSpan,
    RunStatus, RunStore, RunSummaryRow, Span, SpanState, StartRunCommand, StepError,
    StepSpanKind, StepSpanStatus, StrategyProposal, SupervisionFinding, SupervisionLogEntry,
    SupervisionProfile, SupervisionRule, SupervisionRunRecord, SupervisionRunStatus,
    SupervisionSeverity, SupervisionSnapshot, Task, TaskAdvanced, TaskBlocked, TaskCancelled,
    TaskCompleted, TaskContract, TaskContractRevision, TaskContractV1, TaskCreated, TaskEvent,
    TaskRevision, TaskSnapshotImported, TaskStatus, TaskStore, ThresholdPolicy,
    TransferPrivacyDecision, UsageExecutorKind, UsageMeasurement, UsageRecord, WorkerResult,
    WorkerRun, WorkerStatus, WorkflowIR, WorkflowNodeIR, WorkflowPlan, WorkflowPort,
    WorkflowRevision, WorkflowRevisionPort, WorkflowStep, WorkflowStore,
};
pub use task::run as supervision;

// --- Conversation & Sessions ---
pub use conversation::{
    ConversationTurn, CreateHeadlessJobParams, HeadlessAutomationJob, HeadlessJobStatus,
    HeadlessTaskSpec, HeadlessTrigger, RunHeadlessJobResult, Session, SessionId,
    SessionJournalEntry, SessionMode, SessionStatus, SessionStore, SessionTaskBinding,
    TurnActorKind, WorkbenchId, WorkbenchLens,
};

// --- Authority, Permits & Credentials ---
pub use authority::{
    Action, ActionIntent, ActionIntentV1, ActionLifecycleState, ApprovalDecision, ApprovalRequest,
    ApprovalStatus, Assurance, Budget, CapabilityDescriptor, CapabilityGroup, CapabilityManifest,
    CapabilityStatus, CommandAdmission, CommandLedgerEntry, CommandLedgerPort, CredentialLocation,
    CredentialMetadata, CredentialOperation, CredentialVault, CredentialVaultError, CredentialsPort,
    CredentialsVaultPort, EffectAttempt, EffectLedgerPort, EffectStatus, ExecutionPermit,
    ExecutionReceipt, Grant, Headroom, OutboxActionItem, OutboxDeliveryState, OutboxEntry,
    OutboxPort, OutboxStatus, Permit, PermitId, PermitV1, Receipt, ReceiptStatus, ReservationToken,
    ResourceActionCapability, RiskClass, RiskLevel, StandingGrant,
};

// --- Evidence & Judgment ---
pub use evidence::{
    classify, protection_status, Check, CheckGroup, CheckStatus, CriterionVerificationRecord,
    EvidenceBundle, EvidenceRecord, EvidenceRecordV1, EvidenceRequirement, EvidenceStatus,
    ExecEvent, ExecState, ExecutionArtifact, ExecutionAttempt, ExecutionRecord,
    ExecutionRecordStatus, ExternalJobRef, Fact, FindingSeverity, Invocation,
    InvocationOutcome, JudgmentPort, JudgmentRequest, JudgmentResult, OutcomeStatus,
    ProgressTriplet, ProtectionPolicy, ProtectionStatus, RecordReviewParams, ResourceEvent,
    ResourceState, ResumeVerdict, ReviewFinding, ReviewMethod, ReviewStatus,
    ReviewTargetType, ReviewerRecord, TargetCaps, TaskOutcome, VerificationClaim,
    VerificationReport,
};

// --- Context & Memory ---
pub use context::{
    AnnotationRecord, AnnotationStatus, AnnotationTarget, ContextDeliveryState, ContextItem,
    ContextPack, ContextReceipt, FactProposal, MemoryEntry, MemoryError, MemoryPort, MemoryScope,
    OmittedContextReason, OmittedContextRef, PersonalFact, ProposalReceipt, ProposalStatus,
    RecallQuery,
};

// --- Resources, Workspace, Files & Compute ---
pub use resources::{
    AcceleratorSpec, ArtifactHandoff, ArtifactKind, ArtifactLineageGraph, ArtifactLineageNode,
    ArtifactRef, ArtifactSummary, AttemptObservation, BlobMetadata, CasPort, CheckpointCommitStatus,
    CheckpointFileEntry, CheckpointStorePort, CloudPodLease, CloudPodRequest, CloudPodStatus,
    CompatibilityGateEvaluation, CompatibilityVerdict, ComputeBackendKind, ComputeDeviceRef,
    ComputePort, ComputeTelemetry, ComputeWorkloadKind, DecisionPort, DeviceKind, DirtyManifest,
    ExecutionTargetPort, ExecutionTargetResolver, ExecutionWorkspace, FrozenTrainingRecipe,
    HandoffConsent, HardwareAdaptationPlan, LineageGraphEdge, LineageGraphNode,
    ModelEvaluationReport, ModelEvaluationSpec, NoteRecord, NoteVersionRecord,
    NotebookDriverPort, NotebookStoragePort, NotebookWorkerResponse, NotebookWorkerSession,
    OptionalConsumerPlacement, PreflightReport, PromotionOutcome, ReplanPort,
    RepositoryGitPort, ResumabilityLevel, SourceAcquisitionPort, SourceSpan, StoragePort,
    StoreCapabilities, SupervisionStorePort, TargetCapabilities, TerminalDriverPort,
    TerminalOutputChunk, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle,
    TerminalSession, TerminalSessionId, TerminalSessionStatus, TrainingCheckpointManifest,
    TrainingExecutionReceipt, TrainingStepTelemetry, WorkloadAdmissionPort, WorkloadAdmissionRequest,
    WorkloadAdmissionVerdict, WorkloadLease, WorkloadLeaseStatus, WorkspaceDiffEntry,
    WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff, WorkspaceFileEntry,
    WorkspaceFileTree, WorkspaceFilesPort, WorkspaceId, WorkspaceKind, WorkspaceLineage,
    WorkspaceProvider, WorkspaceRepository, WorkspaceStatus, WriteWorkspaceFileParams,
};

// --- Execution & Harnesses ---
pub use execution::{
    validate_browser_url, AgentDriver, AgentRuntimePort, BrowserConsoleEntry,
    BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab, BrowserTabStatus,
    CaptureSnapshotParams, CellExecutionStatus, CommandExecutionResult, CommandRunnerPort,
    CostVisibility, CreateTabParams, EnvSnapshot, EnvironmentAssurance, EnvironmentHandle,
    EnvironmentPort, EnvironmentProfile, EnvironmentSpec, EnvironmentState, ExecutionResult, ExecuteCellParams,
    ExecuteCellResult, ExecutionLane, FleetExecParams, FleetExecReceipt, FleetExecResult,
    GateReport, HarnessDescriptor, HarnessExecutionResult, HarnessMaturity, HarnessProfile,
    KernelStatus, LanePolicy, NavigateTabParams, NotebookCell, NotebookCellType, NotebookDocument,
    NotebookKernelBinding, NotebookKernelState, RegisterHostParams, RemoteHostNode,
    RemoteHostStatus, RunObserver, SandboxCommand, SandboxPort, SshAuthMethod, ToolGate,
    ToolMediationLevel, WorktreeOwnership,
};

// --- Inference & Models ---
pub use inference::{
    ClientApiKeyRecord, LoadedModelInstance, LocalHardwareProfile, LocalModelDescriptor,
    LocalModelFolder, LocalModelLoadProfile, LocalModelLoadRequest, LocalModelPreflightRequest,
    LocalModelPreflightResponse, ModelCatalogOption, ModelCatalogResult, ModelCompatibility,
    ModelPort, ModelPricing, ModelProvider, ModelResponse, ModelTurnEvent, ModelTurnRequest,
    OAuthTokenRecord, ProbedModel, ProviderConfig, ProviderEvent, ProviderHealthStatus,
    ProviderObservationSnapshot, ProviderRequest, TokenUsage, ToolCall,
    ToolDefinition, ToolPort, ToolResult, TurnDelta,
};

// --- Contributions, Packs & Skills ---
pub use contributions::{
    DomainPack, DomainPackManifest, DownloadProgress, InstalledPath, PackageDistributionEnvelope,
    PackageError, PackageTransferPort, PackLifecycle, PackProfile, PackVerifier, Skill,
    SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, SkillSink, StagedArtifact,
    TaintLevel, VerificationContext, VerificationResult, VerifierResult, WorkbenchJob,
    WorkbenchModule, WorkerRole, WorkerRunId, WorkspaceScope, WriteTarget,
};

// --- Kernel Namespace ---
pub mod kernel {
    pub use crate::conversation::ports::SessionStore;
    pub use crate::task::events::{TaskEvent, TaskSnapshotImported};
    pub use crate::task::ports::{KernelPort, TaskStore};
}

// --- Canonical Contract Port Compatibility Layer ---
pub mod contracts {
    pub use crate::*;
    pub mod agent {
        pub use crate::execution::ports::*;
    }
    pub mod authority {
        pub use crate::authority::*;
    }
    pub mod checkpoint_store {
        pub use crate::resources::compute::*;
        pub use crate::resources::ports::*;
    }
    pub mod compute {
        pub use crate::resources::compute::*;
        pub use crate::resources::ports::*;
    }
    pub mod credentials {
        pub use crate::authority::ports::*;
    }
    pub mod environment {
        pub use crate::execution::ports::*;
    }
    pub mod harness {
        pub use crate::execution::harness::*;
        pub use crate::execution::ports::*;
    }
    pub mod judgment {
        pub use crate::evidence::ports::*;
    }
    pub mod kernel {
        pub use crate::kernel::*;
    }
    pub mod memory {
        pub use crate::context::ports::*;
    }
    pub mod notebook {
        pub use crate::execution::notebook::*;
    }
    pub mod oi {
        pub use crate::task::oi::*;
        pub use crate::task::ports::OiPlannerPort;
    }
    pub mod packages {
        pub use crate::contributions::packages::*;
        pub use crate::contributions::ports::*;
        pub use crate::resources::ports::*;
    }
    pub mod pack {
        pub use crate::contributions::pack::*;
    }
    pub mod sandbox {
        pub use crate::authority::ports::*;
        pub use crate::execution::ports::*;
        pub use crate::execution::terminal::*;
    }
    pub mod skill {
        pub use crate::contributions::skill::*;
    }
    pub mod source_acquisition {
        pub use crate::resources::ports::*;
    }
    pub mod storage {
        pub use crate::authority::ports::*;
        pub use crate::resources::ports::*;
        pub use crate::task::ports::*;
    }
    pub mod supervision {
        pub use crate::resources::ports::SupervisionStorePort;
        pub use crate::task::run::telemetry::*;
        pub use crate::task::run::watchdog::*;
    }
    pub mod terminal {
        pub use crate::execution::ports::*;
        pub use crate::execution::terminal::*;
    }
    pub mod tool_gate {
        pub use crate::execution::ports::*;
    }
    pub mod workflow {
        pub use crate::resources::ports::SupervisionStorePort;
        pub use crate::task::ports::*;
        pub use crate::task::run::telemetry::*;
        pub use crate::task::run::watchdog::*;
        pub use crate::task::workflow::*;
    }
    pub mod workspace {
        pub use crate::resources::ports::*;
        pub use crate::resources::workspace::*;
    }
    pub mod workspace_files {
        pub use crate::resources::ports::*;
        pub use crate::resources::workspace_files::*;
    }
}


