//! Custos Trusted Core
//!
//! Kernel state machine transitions, capability gating, authority evaluation,
//! and cryptographic evidence verification.

pub mod authority;
pub mod capability;
pub mod context;
pub mod contracts;
pub mod evidence;
pub mod kernel;
pub mod sandbox_policy;

// Re-exports and compatibility aliases
pub use authority::*;
pub use capability::*;
pub use context::*;
pub use evidence::*;
pub use kernel::*;
pub use sandbox_policy as sandbox;
