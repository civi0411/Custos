//! OI Candidate representation & Rejection Reason taxonomy (RFC 003 §2B / RFC 004)

use super::topology::ExecutionTopology;
use serde::{Deserialize, Serialize};

/// An execution candidate evaluated during OI deliberation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub topology: ExecutionTopology,
    pub harness_id: String,
    pub est_cost_usd_min: f32,
    pub est_cost_usd_max: f32,
    pub est_tokens: u64,
    pub est_latency_ms: u64,
    pub risks: Vec<String>,
}

/// Explicit reason why an execution candidate was filtered out or rejected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// Token or monetary budget would be exceeded
    BudgetExceeded,
    /// Required capability is missing from the harness
    CapabilityMissing(String),
    /// Proposed harness conflicts with pinned model configuration
    ModelPinMismatch,
    /// Violates network egress restrictions
    EgressViolation(String),
    /// Local-only constraint violated by external cloud provider
    LocalOnlyConstraint,
    /// Sensitivity / data classification level exceeded
    SensitivityExceeded,
    /// Static policy rule violation
    PolicyViolation(String),
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded => write!(f, "Budget exceeded"),
            Self::CapabilityMissing(cap) => write!(f, "Missing capability: {}", cap),
            Self::ModelPinMismatch => write!(f, "Model pin mismatch"),
            Self::EgressViolation(rule) => write!(f, "Egress rule violation: {}", rule),
            Self::LocalOnlyConstraint => write!(f, "Local-only constraint violated"),
            Self::SensitivityExceeded => write!(f, "Data sensitivity level exceeded"),
            Self::PolicyViolation(msg) => write!(f, "Policy violation: {}", msg),
        }
    }
}
