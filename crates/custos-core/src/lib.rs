//! Custos Trusted Core
//!
//! Kernel state machine transitions, capability gating, authority evaluation,
//! and cryptographic evidence verification.

pub mod authority;
pub mod capability;
pub mod context;
pub mod contracts;
pub mod decision;
pub mod evidence;
pub mod kernel;
pub mod oi;
pub mod research_ingress;
pub mod sandbox_policy;

/// Standard Decoupled Architectural Blocks (Kim Tự Tháp - Level 1)
pub mod blocks;

// Re-exports and compatibility aliases
pub use authority::*;
pub use capability::*;
pub use context::*;
pub use contracts::skill;
pub use decision::*;
pub use evidence::*;
pub use kernel::*;
pub use oi::*;
pub use sandbox_policy as sandbox;
