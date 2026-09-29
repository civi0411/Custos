//! Custos Trusted Core
//!
//! Kernel state machine transitions, capability gating, authority evaluation,
//! and cryptographic evidence verification.

pub extern crate custos_domain as custos_core_domain;

pub mod authority;
pub mod capability;
pub mod evidence;
pub mod kernel;
pub mod sandbox_policy;

// Re-exports and compatibility aliases
pub use authority::*;
pub use capability::*;
pub use evidence::*;
pub use kernel::*;
pub use sandbox_policy as sandbox;
