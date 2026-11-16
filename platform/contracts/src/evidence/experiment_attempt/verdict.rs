use serde::{Deserialize, Serialize};

use crate::compute::{CheckpointCommitStatus, TrainingCheckpointManifest};

// ---------------------------------------------------- verification report

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckGroup {
    Identity,
    Integrity,
    State,
    DataContinuity,
    Environment,
    PostLoad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    /// Could not be proven. Never treated as Pass.
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub group: CheckGroup,
    pub status: CheckStatus,
    pub evidence: String,
}

/// Produced by an engine profile; domain only reads it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VerificationReport {
    pub profile: String,
    pub checks: Vec<Check>,
}

impl VerificationReport {
    pub fn group(&self, g: CheckGroup) -> impl Iterator<Item = &Check> {
        self.checks.iter().filter(move |c| c.group == g)
    }

    /// Group is satisfied only if it exists and every check passed.
    pub fn all_pass(&self, g: CheckGroup) -> bool {
        let mut it = self.group(g).peekable();
        it.peek().is_some() && it.all(|c| c.status == CheckStatus::Pass)
    }

    pub fn any_fail(&self, g: CheckGroup) -> bool {
        self.group(g).any(|c| c.status == CheckStatus::Fail)
    }

    pub fn unverified(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Unverified)
            .collect()
    }
}

// ----------------------------------------------------------------- verdict

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCaps {
    pub supports_state_restore: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum ResumeVerdict {
    /// Only for the verified engine profile; never a general guarantee.
    StateResumeSupported {
        profile: String,
    },
    /// Recipe/environment deltas; optimizer dynamics may differ.
    AdaptedContinuation {
        deltas: Vec<String>,
    },
    WeightsOnlyWarmStart,
    Unsupported {
        reasons: Vec<String>,
    },
}

pub(crate) fn has_weights(m: &TrainingCheckpointManifest) -> bool {
    m.files.iter().any(|f| {
        f.path.ends_with("model.safetensors")
            || f.path.ends_with("adapter_model.safetensors")
            || f.path.ends_with("pytorch_model.bin")
    })
}

/// Pure decision table. Integrity or Identity failure always means Unsupported.
pub fn classify(
    manifest: &TrainingCheckpointManifest,
    report: &VerificationReport,
    caps: TargetCaps,
) -> ResumeVerdict {
    use CheckGroup as G;
    let unsupported = |r: &str| ResumeVerdict::Unsupported {
        reasons: vec![r.into()],
    };

    if let Err(e) = manifest.validate() {
        return unsupported(&e.to_string());
    }
    if manifest.commit_status != CheckpointCommitStatus::CommittedProtected {
        return unsupported("Checkpoint is not committed to protected storage");
    }
    if report.any_fail(G::Identity) || report.any_fail(G::Integrity) {
        return unsupported("Identity or integrity check failed");
    }
    if !report.all_pass(G::Integrity) {
        return unsupported("Integrity of checkpoint bytes is not verified");
    }
    if report.any_fail(G::PostLoad) {
        return unsupported("Post-load validation failed");
    }

    let state_ok = manifest.resumability_level.allows_state_continuation()
        && caps.supports_state_restore
        && report.all_pass(G::Identity)
        && report.all_pass(G::State)
        && report.all_pass(G::DataContinuity);

    if state_ok && report.all_pass(G::Environment) {
        return ResumeVerdict::StateResumeSupported {
            profile: report.profile.clone(),
        };
    }
    if state_ok && report.any_fail(G::Environment) {
        let deltas = report
            .group(G::Environment)
            .filter(|c| c.status == CheckStatus::Fail)
            .map(|c| c.id.clone())
            .collect();
        return ResumeVerdict::AdaptedContinuation { deltas };
    }
    if has_weights(manifest) {
        return ResumeVerdict::WeightsOnlyWarmStart;
    }
    unsupported("No usable weights or verified state in checkpoint")
}

