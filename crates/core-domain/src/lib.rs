//! Custos Core Domain
//!
//! Pure, deterministic domain models with ZERO side effects and ZERO external framework dependencies.
//! All models are serializable and validate state machine transitions strictly.

pub mod action;
pub mod approval;
pub mod artifact;
pub mod authority;
pub mod budget;
pub mod capability;
pub mod claim;
pub mod context;
pub mod continuation;
pub mod error;
pub mod evidence;
pub mod fact;
pub mod ids;
pub mod packet;
pub mod run;
pub mod span;
pub mod task;
pub mod types;
pub mod workflow;

// Re-export public domain surface
pub use action::*;
pub use approval::*;
pub use artifact::*;
pub use authority::*;
pub use budget::*;
pub use capability::*;
pub use claim::*;
pub use context::*;
pub use continuation::*;
pub use error::*;
pub use evidence::*;
pub use fact::*;
pub use run::*;
pub use span::*;
pub use task::*;
pub use types::*;
pub use workflow::*;
