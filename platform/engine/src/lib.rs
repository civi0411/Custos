//! Custos Sovereign Execution Engine (custos-engine)
//!
//! Grounded in Alan Kay OOP & Anthropic Agentic Systems (Topologies T0-T6):
//! - Alan Kay OOP: Encapsulated objects passing immutable URR messages across dynamic SPI trait ports.
//! - Anthropic Systems: Reason-Act-Observe cognitive loop, multi-agent topologies, durable supervision.
//! - SADE Framework: Single-writer SQLite WAL, two-phase authority mediation, cryptographic evidence completion gate.
//!
//! ZERO concrete I/O: All storage, persistence, network, and execution drivers are dynamically injected
//! through constructor trait ports (SPI).

// In-crate backward-compatibility aliases:
extern crate custos_contracts as custos_domain;
pub use crate as custos_runtime;

pub mod custos_provider {
    pub use custos_contracts::inference::*;
    pub mod request {
        pub use custos_contracts::inference::{ModelResponse, ProviderRequest};
    }
    pub mod events {
        pub use custos_contracts::inference::*;
    }
    pub mod conversation {
        pub mod message {
            pub use crate::conversation::message::*;
        }
    }
}

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

// Canonical aliases matching target architecture
pub use authority as core_authority;
pub use evidence as core_evidence;
pub use task as kernel;
pub use execution as agent;
pub use orchestration as workflow;
pub use orchestration::oi;
pub use resources as environment;
pub use conversation as session;
pub use components as packs;

// Re-exports from submodules matching core and runtime facades
pub use authority::*;
pub use contracts::skill;
pub use contracts::tool_gate::{ExecutionResult, ToolGate};
pub use evidence::*;
pub use task::*;
pub use resources::*;
pub use conversation::bridge;
pub use bridge::{AttachMode, BridgeError, BridgePort, BridgeService, SteerReceipt, SteerResult};
pub use compute::{
    CheckpointRelayCoordinator, LocalTrainingRunner, ModelEvaluationRunner,
    OptionalConsumerPlacementEngine, RemoteExecutionCoordinator, WorkloadAdmissionCoordinator,
};
pub use notebook::PythonKernelCoordinator;
pub use conversation::SessionManager;
pub use terminal::TerminalCoordinator;
pub use orchestration::TaskRuntime;
pub use workspace::{
    CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceLease, WorkspaceLeaseManager,
};
