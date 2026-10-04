//! Assistant Pack Profile (RFC 003 §4)

use crate::profile::PackProfile;
use custos_domain::oi::ExecutionTopology;

/// Returns the default profile for the Assistant domain pack.
pub fn assistant_profile() -> PackProfile {
    PackProfile::new(
        "assistant",
        "Assistant Pack",
        ExecutionTopology::T0Direct,
        vec![
            ExecutionTopology::T0Direct,
            ExecutionTopology::T1SingleWorker,
        ],
        "user_acceptance_oracle",
        5_000,
        false,
    )
}
