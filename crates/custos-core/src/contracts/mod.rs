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
pub mod experiment;
pub mod harness;
pub mod judgment;
pub mod kernel;
pub mod memory;
pub mod notebook;
pub mod oi;
pub mod sandbox;
pub mod skill;
pub mod source_acquisition;
pub mod storage;
pub mod terminal;
pub mod workflow;
pub mod workspace;
pub mod workspace_files;
pub mod packages;
pub mod worker_wire;

// --- 1. Execution, Kernel & Specialization ---
pub use kernel::{KernelPort, TrustedKernel};
pub use harness::{
    AgentRuntimePort, CostVisibility, HarnessExecutionResult, HarnessProfile, ToolMediationLevel,
    WorktreeOwnership,
};
pub use skill::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, SkillRegistry,
    TaintLevel, WorkerRunId, WorkspaceScope, WriteTarget,
};

// --- 2. Persistence & Storage Ports ---
pub use storage::{
    AssistantDraftPort, CasPort, CommandAdmission, CommandLedgerEntry, CommandLedgerPort, DecisionPort,
    EffectLedgerPort, NotebookStoragePort, OutboxEntry, OutboxPort, OutboxStatus, QuantPort, ReplanPort,
    RunPort, RunStore, SessionStore, StoragePort, TaskStore, WorkflowRevisionPort, WorkflowStore,
};
pub use checkpoint_store::{BlobMetadata, CheckpointStorePort, PromotionOutcome, StoreCapabilities};
pub use memory::MemoryPort;

// --- 3. Process Isolation, Shell & Workspaces ---
pub use sandbox::{SandboxCommand, SandboxPort};
pub use terminal::{TerminalDriverPort, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle};
pub use workspace::{RepositoryGitPort, WorkspaceProvider, WorkspaceRepository};
pub use workspace_files::WorkspaceFilesPort;
pub use notebook::{NotebookDriverPort, NotebookWorkerResponse, NotebookWorkerSession};

// --- 4. Compute & Hardware Admission ---
pub use compute::{CloudPodRequest, ComputePort, ComputeTelemetry, WorkloadAdmissionPort};
pub use experiment::{
    AttemptObservation, ExecutionTargetPort, PreflightReport, TargetCapabilities,
};

// --- 5. Authority, Deliberation & Intelligence ---
pub use judgment::{JudgmentPort, JudgmentRequest, JudgmentResult};
pub use oi::OiPlannerPort;
pub use source_acquisition::SourceAcquisitionPort;
pub use workflow::WorkflowPort;

// --- 6. Packages & Worker Wire Protocol ---
pub use packages::{
    DownloadProgress, InstalledPath, PackageError, PackageTransferPort, StagedArtifact,
};
pub use worker_wire::{WorkerBudget, WorkerObservation, WorkerOperation, WorkerUsage};

