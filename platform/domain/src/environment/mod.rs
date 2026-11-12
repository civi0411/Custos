//! Sovereign Agent Execution Environment (AEE) Domain Models
//!
//! Provides the canonical aggregate vocabulary for agent workspace anchoring,
//! lifecycle states, reproducibility snapshots, and honest assurance metrics.

pub mod assurance;
pub mod browser;
pub mod harness;
pub mod notebook;
pub mod profile;
pub mod snapshot;
pub mod spec;
pub mod state;
pub mod terminal;

pub use assurance::*;
pub use browser::*;
pub use harness::*;
pub use notebook::*;
pub use profile::*;
pub use snapshot::*;
pub use spec::*;
pub use state::*;
pub use terminal::*;

