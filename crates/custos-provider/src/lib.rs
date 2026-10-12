//! Canonical model-attempt contracts and provider-neutral semantics.
//!
//! `ModelPort` plus `ModelTurnRequest`/`ModelTurnEvent` are the public
//! convergence boundary used by the Custos runtime. The richer
//! `types::base::Provider` interface remains available as a compatibility
//! implementation surface while its structured conversation, tool, reasoning,
//! streaming and usage semantics are bridged into `ModelPort`.
//!
//! This crate must not own credentials, vendor HTTP clients, native agent
//! processes or effect authorization. Concrete implementations live in
//! `custos-adapters`; production composition lives in `custos-daemon`.

pub mod catalog;
pub mod conformance;
pub mod events;
pub mod port;
pub mod request;
pub mod tool;
pub mod traits;
pub mod turn;
pub mod types;

pub use catalog::*;
pub use conformance::*;
pub use events::*;
pub use port::*;
pub use request::*;
pub use tool::*;
pub use turn::*;
pub use types::*;
