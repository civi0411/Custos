//! Domain Pack Profiles (RFC 003 §4)
//!
//! Encapsulates default and supported execution topologies, verifiers,
//! and token budgets for each domain pack.

use custos_domain::oi::ExecutionTopology;
use serde::{Deserialize, Serialize};

/// Canonical configuration and constraints for a domain pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackProfile {
    pub pack_id: String,
    pub name: String,
    pub default_topology: ExecutionTopology,
    pub supported_topologies: Vec<ExecutionTopology>,
    pub preferred_verifier: String,
    pub default_budget_tokens: u64,
    /// If false, automated blind retries on uncertainty are prohibited.
    pub allow_blind_retry: bool,
}

impl PackProfile {
    pub fn new(
        pack_id: impl Into<String>,
        name: impl Into<String>,
        default_topology: ExecutionTopology,
        supported_topologies: Vec<ExecutionTopology>,
        preferred_verifier: impl Into<String>,
        default_budget_tokens: u64,
        allow_blind_retry: bool,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            name: name.into(),
            default_topology,
            supported_topologies,
            preferred_verifier: preferred_verifier.into(),
            default_budget_tokens,
            allow_blind_retry,
        }
    }

    /// Checks if a topology is supported by this pack.
    pub fn supports_topology(&self, topology: ExecutionTopology) -> bool {
        self.supported_topologies.contains(&topology)
    }
}
