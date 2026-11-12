//! Canonical Port Registry (Interface Freeze — Sprint 0)
//!
//! Single place that names the 6 frozen ports. Traits are DEFINED here (or re-exported
//! from where they already live) and IMPLEMENTED by outer crates, so the dependency
//! arrow always points inward: `adapters/persistence/runtime -> core -> domain`.
//!
//! | # | Port         | Defined in                         | Implemented by                  | Owner  |
//! |---|--------------|------------------------------------|---------------------------------|--------|
//! | 1 | KernelPort   | `contracts::kernel`                | `TrustedKernel` (this crate)    | Vĩ     |
//! | 2 | StoragePort  | `contracts::storage` (+Cas,Outbox) | `custos-persistence`            | Trường |
//! | 3 | SandboxPort  | `contracts::sandbox`               | `custos-adapters/sandbox`       | Trường |
//! | 4 | ModelPort    | `custos-provider::port`            | `custos-adapters/providers`     | Vĩ     |
//! | 5 | WorkflowPort | `contracts::workflow`              | `custos-runtime/workflow`       | Vinh   |
//! | 6 | MemoryPort   | `contracts::memory`                | `custos-persistence` + runtime  | Vĩ     |
//!
//! NOTE: module is `contracts`, not `ports`, because `kernel::ports` is glob re-exported
//! at the crate root and a top-level `ports` module would shadow `crate::ports::TaskStore`.

pub mod checkpoint_store;
pub mod compute;
pub mod credentials;
pub mod environment;
pub mod harness;
pub mod judgment;
pub mod kernel;
pub mod memory;
pub mod notebook;
pub mod oi;
pub mod packages;
pub mod pack;
pub mod sandbox;
pub mod skill;
pub mod source_acquisition;
pub mod storage;
pub mod terminal;
pub mod tool_gate;
pub mod workflow;
pub mod workspace;
pub mod workspace_files;

// --- 1. Execution, Kernel & Specialization ---
pub use harness::{
    AgentRuntimePort, CostVisibility, HarnessExecutionResult, HarnessProfile, ToolMediationLevel,
    WorktreeOwnership,
};
pub use kernel::{KernelPort, TrustedKernel};
pub use pack::{
    DomainPack, DomainPackManifest, PackLifecycle, PackProfile, PackVerifier, VerificationContext,
    VerificationResult, VerifierResult, WorkbenchJob, WorkbenchModule, WorkerRole,
};
pub use skill::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, SkillSink, TaintLevel,
    WorkerRunId, WorkspaceScope, WriteTarget,
};

// --- 2. Persistence & Storage Ports ---
pub use checkpoint_store::{
    BlobMetadata, CheckpointStorePort, PromotionOutcome, StoreCapabilities,
};
pub use memory::MemoryPort;
pub use storage::{
    CasPort, CommandAdmission, CommandLedgerEntry, CommandLedgerPort, DecisionPort,
    EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus, ReplanPort, RunPort, RunStore,
    SessionStore, StoragePort, TaskStore, WorkflowRevisionPort, WorkflowStore,
};

// --- 3. Process Isolation, Shell & Workspaces ---
pub use sandbox::{CommandExecutionResult, CommandRunnerPort, SandboxCommand, SandboxPort};
pub use terminal::{
    TerminalDriverPort, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle,
};
pub use tool_gate::{ExecutionResult, ToolGate};
pub use workspace::{RepositoryGitPort, WorkspaceProvider, WorkspaceRepository};
pub use workspace_files::WorkspaceFilesPort;

// --- 4. Compute & Hardware Admission ---
pub use compute::{
    AttemptObservation, CloudPodRequest, ComputePort, ComputeTelemetry, ExecutionTargetPort,
    ExecutionTargetResolver, PreflightReport, TargetCapabilities, WorkloadAdmissionPort,
};
pub use notebook::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, KernelStatus, NotebookCell,
    NotebookCellType, NotebookDocument, NotebookDriverPort, NotebookKernelBinding,
    NotebookKernelState, NotebookStoragePort, NotebookWorkerResponse, NotebookWorkerSession,
};

// --- 5. Authority, Deliberation & Intelligence ---
pub use judgment::{JudgmentPort, JudgmentRequest, JudgmentResult};
pub use oi::OiPlannerPort;
pub use source_acquisition::SourceAcquisitionPort;
pub use workflow::WorkflowPort;

// --- 6. Packages ---
pub use packages::{
    DownloadProgress, InstalledPath, PackageError, PackageTransferPort, StagedArtifact,
};

// --- 7. Sovereign Agent Execution Environment (AEE) ---
pub use environment::{EnvironmentHandle, EnvironmentPort, GateReport};
pub use storage::SupervisionStorePort;
pub use workflow::{
    NoopSupervisionSink, NormalizedTelemetryBatch, RunWaterfallView, SupervisionFindingFilter,
    SupervisionIngestPort, SupervisionLogFilter, SupervisionOverview, SupervisionQueryPort,
    SupervisionRunFilter, SupervisionTraceSink,
};
pub mod supervision {
    pub use super::storage::SupervisionStorePort;
    pub use super::workflow::{
        NoopSupervisionSink, NormalizedTelemetryBatch, RunWaterfallView, SupervisionFindingFilter,
        SupervisionIngestPort, SupervisionLogFilter, SupervisionOverview, SupervisionQueryPort,
        SupervisionRunFilter, SupervisionTraceSink,
    };
}
