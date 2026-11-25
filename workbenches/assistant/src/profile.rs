//! Assistant Pack Profile (RFC 003 §4)

use custos_contracts::oi::ExecutionTopology;
use custos_engine::contracts::pack::PackProfile;

/// Returns the default profile for the Assistant domain pack.
pub fn assistant_profile() -> PackProfile {
    PackProfile::new(
        "assistant",
        "Assistant Pack",
        ExecutionTopology::T0Direct,
        vec![
            ExecutionTopology::T0Direct,
            ExecutionTopology::T1SingleWorker,
            ExecutionTopology::T2StagedPipeline,
        ],
        "user_acceptance_oracle",
        5_000,
        false,
    )
}
