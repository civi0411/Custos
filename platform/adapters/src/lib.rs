//! # Custos Adapters Crate (`custos-adapters`)
//!
//! Pillar 2: Host OS Infrastructure Drivers, Agent Harnesses & Sovereign Connectors in Layer 2.
//!
//! Organized into 3 dedicated subsystems:
//! 1. `host`: Physical OS & hardware drivers (sandbox, terminal, workspace, notebook, packages, documents, vault)
//! 2. `harnesses`: Bounded CLI agent subprocess drivers (Claude Code, Codex, Goose, OpenCode)
//! 3. `connectors`: External network gateways & plugin federation (mcp, a2a, compute, oauth, roaming)

// ============================================================================
// I. Dedicated Subsystems
// ============================================================================

pub mod harnesses;
pub mod host;
pub mod connectors;

// ============================================================================
// II. Host OS Infrastructure Drivers & Agent Harnesses (Direct Re-exports)
// ============================================================================

pub use harnesses as harness;
pub use harnesses as cli_agents;
pub use harnesses::adapter::CliAgentAdapter;
pub use host::binaries;
pub use host::caffeinate;
pub use host::caffeinate::{CaffeinateManager, CAFFEINATE_MANAGER};
pub use host::documents;
pub use host::integrations;
pub use host::notebook;
pub use host::operations;
pub use host::packages;
pub use host::sandbox;
pub use host::telemetry;
pub use host::terminal;
pub use host::vault;
pub use host::workspace;

/// Re-export workspace github utilities for repository integration
pub use host::workspace::github as repository_git;

// ============================================================================
// III. Sovereign Connectors & Plugin Gateways (Direct Re-exports)
// ============================================================================

pub use connectors::a2a;
pub use connectors::compute;
pub use connectors::mcp;
pub use connectors::oauth;
#[cfg(feature = "roaming")]
pub use connectors::roaming;

/// Legacy alias for connectors
pub use connectors as protocols;

/// Legacy alias for capabilities
pub mod capabilities {
    pub use super::host::documents;
}

// ============================================================================
// IV. Compatibility Re-exports for AI Subsystem
// ============================================================================

pub use custos_provider as ai;
pub use custos_provider::cloud as providers;
pub use custos_provider::local as local_inference;
