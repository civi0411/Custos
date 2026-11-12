//! Custos Trusted Core
//!
//! Kernel state machine transitions, capability gating, authority evaluation,
//! and cryptographic evidence verification.

pub mod authority;
pub mod contracts;
pub mod evidence;
pub mod kernel;

// Canonical Core Re-exports
pub use authority::*;
pub use contracts::skill;
pub use contracts::tool_gate::{ExecutionResult, ToolGate};
pub use evidence::*;
pub use kernel::*;
