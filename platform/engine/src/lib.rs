//! Custos Sovereign Execution Engine (custos-engine)
//!
//! Grounded in Alan Kay OOP & Anthropic Agentic Systems (Topologies T0-T6):
//! - Alan Kay OOP: Encapsulated objects passing immutable URR messages across dynamic SPI trait ports.
//! - Anthropic Systems: Reason-Act-Observe cognitive loop, multi-agent topologies, durable supervision.
//! - SADE Framework: Single-writer SQLite WAL, two-phase authority mediation, cryptographic evidence completion gate.
//!
//! ZERO concrete I/O: All storage, persistence, network, and execution drivers are dynamically injected
//! through constructor trait ports (SPI).

pub mod application;
pub mod authority;
pub mod components;
pub mod context;
pub mod contracts;
pub mod conversation;
pub mod evidence;
pub mod execution;
pub mod orchestration;
pub mod resources;
pub mod supervision;
pub mod task;

// Canonical module aliases
pub use components as packs;
pub use conversation as session;
pub use execution as agent;
pub use orchestration as workflow;
pub use orchestration::oi;
pub use resources as environment;

// Re-exports from submodules matching core and runtime facades
pub use authority::*;
pub use bridge::{AttachMode, BridgeError, BridgePort, BridgeService, SteerReceipt, SteerResult};
pub use components::{
    compile_governed_tools, create_default_pack_registry, create_default_skill_registry,
    create_pack_registry_from_inventory, create_skill_registry_from_inventory,
    get_manifest_for_pack, register_pack_verifiers_into_pipeline, resolve_pack_by_id,
    DomainPackRegistry, ModuleRegistry,
};
pub use compute::{
    CheckpointRelayCoordinator, LocalTrainingRunner, ModelEvaluationRunner,
    OptionalConsumerPlacementEngine, RemoteExecutionCoordinator, WorkloadAdmissionCoordinator,
};
pub use contracts::skill;
pub use contracts::tool_gate::{ExecutionResult, ToolGate};
pub use conversation::bridge;
pub use conversation::SessionManager;
pub use custos_contracts::{DomainPack, DomainPackManifest, WorkerRole};
pub use evidence::*;
pub use notebook::PythonKernelCoordinator;
pub use orchestration::TaskRuntime;
pub use resources::*;
pub use task::*;
pub use terminal::TerminalCoordinator;
pub use workspace::{
    CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceLease, WorkspaceLeaseManager,
};
