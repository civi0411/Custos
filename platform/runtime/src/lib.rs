//! Custos Cognitive Orchestration & Runtime (6 Canonical Pillars)
//!
//! - `agent`: Governed agent loop, System-1 heuristics/micro-executor, skill registry
//! - `context`: Token-aware compilation, memory, compaction, and redaction
//! - `environment`: Coordinated execution environments (workspace, terminal, notebook, compute)
//! - `oi`: Orchestration Intelligence planner, topologies, admissibility, benchmark
//! - `session`: Interactive session lifecycle, journal, session-task bridge
//! - `workflow`: Event loop, DAG dispatcher, task execution, worker supervisor

pub mod agent;
pub mod context;
pub mod environment;
pub mod oi;
pub mod session;
pub mod workflow;

pub use crate::context::memory as memory_service;
pub use environment::*;
pub use session::bridge;

// Targeted root re-exports
pub use bridge::{AttachMode, BridgeError, BridgePort, BridgeService, SteerReceipt, SteerResult};
pub use compute::{
    CheckpointRelayCoordinator, LocalTrainingRunner, ModelEvaluationRunner,
    OptionalConsumerPlacementEngine, RemoteExecutionCoordinator, WorkloadAdmissionCoordinator,
};
pub use notebook::PythonKernelCoordinator;
pub use session::SessionManager;
pub use terminal::TerminalCoordinator;
pub use workflow::TaskRuntime;
pub use workspace::{
    CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceLease, WorkspaceLeaseManager,
};
