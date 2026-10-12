//! Research Pack Profile (RFC 003 §4)

use crate::profile::PackProfile;
use custos_domain::oi::ExecutionTopology;

/// Returns the default profile for the Research domain pack.
pub fn research_profile() -> PackProfile {
    PackProfile::new(
        "research",
        "Research Pack",
        ExecutionTopology::T3ReadFanOut,
        vec![
            ExecutionTopology::T0Direct,
            ExecutionTopology::T1SingleWorker,
            ExecutionTopology::T2StagedPipeline,
            ExecutionTopology::T3ReadFanOut,
            ExecutionTopology::T4Worktree,
        ],
        "citation_coverage_oracle",
        15_000,
        true, // Read-only idempotent research tasks can safely retry
    )
}
