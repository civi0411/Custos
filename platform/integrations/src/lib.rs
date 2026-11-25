//! CUSTOS UNIFIED INTEGRATIONS SUBSYSTEM (`custos-integrations`)
//!
//! Block 3 of the Custos 4-Block Target Architecture:
//! Pure Concrete I/O, AI Model Providers, System Drivers, Protocols, and External Connectors.

pub mod compute;
pub mod connectors;
pub mod credentials;
pub mod filesystem;
pub mod harnesses;
pub mod host;
pub mod models;
pub mod notebook;
pub mod protocols;
pub mod sandbox;
pub mod terminal;

// ============================================================================
// I. Compatibility Re-exports for AI Subsystem
// ============================================================================

pub use models::catalog;
pub use models::cloud;
pub use models::contracts;
pub use models::contracts::types::documents;
pub use models::contracts::*;
pub use models::local;
pub use models::routing;

pub use models as ai;
pub use models::cloud as providers;
pub use models::local as local_inference;
pub use models::local::local_inference::prompt_template;

pub mod model {
    pub use crate::models::contracts::types::model::*;
    pub use crate::models::local::local_inference::model::*;
}

// ============================================================================
// II. Compatibility Re-exports for Adapters Subsystem
// ============================================================================

pub use harnesses as harness;
pub use harnesses as cli_agents;
pub use harnesses::adapter::CliAgentAdapter;

pub use connectors::a2a;
pub use connectors::compute as compute_connectors;
pub use connectors::mcp;
pub use connectors::oauth;
#[cfg(feature = "roaming")]
pub use connectors::roaming;

pub use host::binaries;
pub use host::caffeinate;
pub use host::caffeinate::{CaffeinateManager, CAFFEINATE_MANAGER};
pub use host::documents as host_documents;
pub use host::integrations;
pub use host::operations;
pub use host::packages;
pub use host::telemetry;
pub use host::vault;
pub use host::worker;
pub use host::workspace;

/// Re-export workspace github utilities for repository integration
pub use host::workspace::github as repository_git;

pub mod capabilities {
    pub use super::host::documents;
}
