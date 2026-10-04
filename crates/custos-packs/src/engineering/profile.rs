//! Engineering Pack Profile (RFC 003 §4)

use crate::profile::PackProfile;
use custos_domain::oi::ExecutionTopology;

/// Returns the default profile for the Engineering domain pack.
pub fn engineering_profile() -> PackProfile {
    PackProfile::new(
        "engineering",
        "Engineering Pack",
        ExecutionTopology::T4Worktree,
        vec![
            ExecutionTopology::T1SingleWorker,
            ExecutionTopology::T2StagedPipeline,
            ExecutionTopology::T4Worktree,
            ExecutionTopology::T6RepairLoop,
        ],
        "cargo_test_oracle",
        25_000,
        false, // Engineering modifications require strict verification and no blind retry
    )
}
