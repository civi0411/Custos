//! Wire DTOs shared across Ring 2 artifacts/notes and Research synthesis RPCs.

pub mod notes;
pub mod synthesis;

pub use notes::*;
pub use synthesis::*;

pub use custos_domain::ArtifactLineageNode;
use serde::{Deserialize, Serialize};

/// 4-Level Grounding Status enforcing zero false-passes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ClaimGroundingLevel {
    /// L0: Raw assertion without citation or verification.
    #[default]
    L0Ungrounded,
    /// L1: Formally anchored to a cryptographic passage in a verified source.
    L1Cited,
    /// L2: Empirically verified via reproducible computation or invariant check.
    L2Verified,
    /// L3: Invariant proof sealed into CAS Ledger (`cas://bafy...`).
    L3Sealed,
}

