//! Custos Core Domain (Canonical Re-export Facade over custos-contracts)
//!
//! Pure, deterministic domain models with ZERO side effects and ZERO external framework dependencies.
//! Re-exports authoritatively from `custos-contracts`.

pub use custos_contracts::*;

// Module aliases for backward compatibility with existing code
pub use custos_contracts::authority as authority;
pub use custos_contracts::context as context;
pub use custos_contracts::execution as environment;
pub use custos_contracts::evidence as evidence;
pub use custos_contracts::execution as fleet;
pub use custos_contracts::task::oi as oi;
pub use custos_contracts::contributions as packages;
pub use custos_contracts::task::pipeline as pipeline;
pub use custos_contracts::inference as provider;
pub use custos_contracts::conversation as session;
pub use custos_contracts::identity as shared;
pub use custos_contracts::task as task;
pub use custos_contracts::resources as workspace;

// Additional aliases
pub use custos_contracts::continuation::ContinuationPacket as PacketContinuation;
pub use custos_contracts::execution::lane::{ExecutionLane, LanePolicy};
pub use custos_contracts::oi as decision;
pub use custos_contracts::task::run as supervision;
