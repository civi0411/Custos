use serde::{Deserialize, Serialize};

use crate::DomainError;

fn invalid(msg: impl Into<String>) -> DomainError {
    DomainError::Validation(msg.into())
}

// ------------------------------------------------------------- invocation

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationOutcome {
    Ok,
    Err,
    /// Process/daemon died before the outcome was recorded: attempt becomes Uncertain.
    Unknown,
}

/// One CLI/API call. Distinct from the remote job it may have created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub id: String,
    pub attempt_id: String,
    pub op: String,
    pub idempotency_key: String,
    /// Fingerprint of redacted argv; never raw argv (may carry secrets).
    pub argv_fingerprint: String,
    pub outcome: InvocationOutcome,
}

// -------------------------------------------------------------- progress

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressTriplet {
    pub observed: Option<u64>,
    pub local: Option<u64>,
    pub protected: Option<u64>,
}

impl ProgressTriplet {
    /// protected <= local <= observed; anything else is a producer bug or a lie.
    pub fn validate(&self) -> Result<(), DomainError> {
        let o = self.observed.unwrap_or(0);
        let l = self.local.unwrap_or(0);
        let p = self.protected.unwrap_or(0);
        if p > l || l > o {
            return Err(invalid(
                "Progress must satisfy protected <= local <= observed",
            ));
        }
        Ok(())
    }

    /// The only step with evidence of recoverability.
    pub fn recovery_point(&self) -> Option<u64> {
        self.protected
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionPolicy {
    pub max_unprotected_steps: u64,
    pub max_unprotected_secs: u64,
    pub max_inflight_uploads: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionStatus {
    Protected,
    AtRisk,
    /// Uploads are backing up beyond the cap; raising save frequency would not help.
    Degraded,
}

pub fn protection_status(
    progress: &ProgressTriplet,
    policy: &ProtectionPolicy,
    secs_since_protected: u64,
    inflight_uploads: u32,
) -> ProtectionStatus {
    if inflight_uploads > policy.max_inflight_uploads {
        return ProtectionStatus::Degraded;
    }
    let gap = progress
        .observed
        .unwrap_or(0)
        .saturating_sub(progress.protected.unwrap_or(0));
    if gap > policy.max_unprotected_steps
        || (gap > 0 && secs_since_protected > policy.max_unprotected_secs)
    {
        ProtectionStatus::AtRisk
    } else {
        ProtectionStatus::Protected
    }
}

