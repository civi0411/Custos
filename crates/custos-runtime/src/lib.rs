//! Custos Cognitive Orchestration & Runtime
//!
//! Manages the agent loop, cognitive S1/S2 routing, workflow DAG execution,
//! session lifecycle, and context window assembly.

pub mod agent;
pub mod blocks;
pub mod cognitive;
pub mod compute;
pub mod context;
pub use context::compaction as context_management;
pub mod clock;
pub mod event_bus;
pub mod integration;
pub use crate::context::memory as memory_service;
pub mod notebook;
pub mod oi;
pub mod packages;
pub mod session;
pub mod terminal;
pub mod workflow;
pub mod workspace;

pub use clock::{ClockError, ClockTick, PrecisionEventClock};
pub use event_bus::{EventBusError, PipelineEvent, PipelineEventBus};
pub use integration::{
    BindingDirection, IntegrationBinding, IntegrationKind, IntegrationRegistry, LifecycleState,
    MediationLevel,
};
pub use packages::WorkerLease;

// Targeted root re-exports
pub use compute::{
    CheckpointRelayCoordinator, LocalTrainingRunner, ModelEvaluationRunner,
    OptionalConsumerPlacementEngine, RemoteExecutionCoordinator, WorkloadAdmissionCoordinator,
};
pub use notebook::PythonKernelCoordinator;
pub use session::SessionManager;
pub use terminal::TerminalCoordinator;
pub use workflow::{TaskRuntime, WorkspaceLeaseManager};
pub use workspace::{CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceFilesCoordinator};
