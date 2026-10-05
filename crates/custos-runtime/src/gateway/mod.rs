//! Custos AgentGateway & 9Router Control Plane
//!
//! Provides intelligent tier-based routing (System 0, 1, 2, HumanGate),
//! local/external/A2A dispatching, token and cost budget enforcement,
//! rate limiting, and execution observability.

pub mod budget;
pub mod dispatch;
#[allow(clippy::module_inception)]
pub mod gateway;
pub mod observability;
pub mod policy;

pub use budget::*;
pub use dispatch::*;
pub use gateway::*;
pub use observability::*;
pub use policy::*;
