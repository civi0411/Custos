//! Supervised Experiment Execution & Recovery — pure domain model (F10 / P1).
//!
//! Zero I/O. Two independent axes per attempt (execution vs. compute resource),
//! a verification report that distinguishes `Unverified` from `Pass`, and a
//! resume verdict derived from evidence rather than declared booleans.

use serde::{Deserialize, Serialize};

use crate::compute::{CheckpointCommitStatus, ComputeBackendKind, TrainingCheckpointManifest};
use crate::ids::new_id;
use crate::DomainError;

fn invalid(msg: impl Into<String>) -> DomainError {
    DomainError::Validation(msg.into())
}

// ---------------------------------------------------------------- identity

/// Stable external identity of a remote job. "latest" is never a valid version:
/// another push could replace it and we would observe the wrong job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalJobRef {
    pub account_ref: String,
    pub job_id: String,
    pub version_id: String,
}

impl ExternalJobRef {
    pub fn new(
        account_ref: impl Into<String>,
        job_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let r = Self { account_ref: account_ref.into(), job_id: job_id.into(), version_id: version_id.into() };
        let bad = |s: &str| s.trim().is_empty();
        if bad(&r.account_ref) || bad(&r.job_id) || bad(&r.version_id)
            || r.version_id.trim().eq_ignore_ascii_case("latest")
        {
            return Err(invalid("External job ref needs account, job and a pinned (non-'latest') version"));
        }
        Ok(r)
    }
}

// ----------------------------------------------------------------- states

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecState {
    Planned,
    Preflighted,
    Submitting,
    Submitted,
    Running,
    /// Outcome of the last effect (submit/observe/stop) is unknown; never resubmit blindly.
    Uncertain,
    Stopping,
    Stopped,
    Completed,
    Failed,
    Collected,
    Abandoned,
}

impl ExecState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Collected | Self::Abandoned)
    }
}

/// Compute resource axis: independent from whether training is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState {
    Unknown,
    Held,
    ReleaseRequested,
    ReleaseConfirmed,
    ReleaseFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecEvent {
    PreflightOk,
    SubmitStarted,
    SubmitAcked(ExternalJobRef),
    SubmitRejected,
    ObservedRunning,
    ConnectionLost,
    ReconciledRunning,
    ReconciledCompleted,
    ReconciledFailed,
    RemoteCompleted,
    RemoteFailed,
    StopRequested,
    StopConfirmed,
    Collected,
    Abandon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEvent {
    Observed,
    ReleaseRequested,
    ReleaseConfirmed,
    ReleaseFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAttempt {
    pub id: String,
    pub experiment_id: String,
    pub recipe_id: String,
    pub backend: ComputeBackendKind,
    pub external_ref: Option<ExternalJobRef>,
    pub exec_state: ExecState,
    pub resource_state: ResourceState,
    pub parent_checkpoint_id: Option<String>,
    /// Fencing generation; only a store that checks it can enforce it.
    pub generation: u64,
    pub output_prefix: String,
}

impl ExecutionAttempt {
    pub fn new(
        experiment_id: impl Into<String>,
        recipe_id: impl Into<String>,
        backend: ComputeBackendKind,
        parent_checkpoint_id: Option<String>,
    ) -> Self {
        let id = new_id("attempt");
        let experiment_id = experiment_id.into();
        Self {
            output_prefix: format!("experiments/{experiment_id}/attempts/{id}/"),
            id,
            experiment_id,
            recipe_id: recipe_id.into(),
            backend,
            external_ref: None,
            exec_state: ExecState::Planned,
            resource_state: ResourceState::Unknown,
            parent_checkpoint_id,
            generation: 0,
        }
    }

    pub fn apply(&mut self, event: ExecEvent) -> Result<(), DomainError> {
        use ExecEvent as E;
        use ExecState as S;
        let next = match (self.exec_state, &event) {
            (S::Planned, E::PreflightOk) => S::Preflighted,
            (S::Preflighted, E::SubmitStarted) => S::Submitting,
            (S::Submitting, E::SubmitAcked(_)) => S::Submitted,
            (S::Submitting, E::SubmitRejected) => S::Failed,
            (S::Submitted, E::ObservedRunning) => S::Running,
            (S::Submitting | S::Submitted | S::Running | S::Stopping, E::ConnectionLost) => S::Uncertain,
            (S::Uncertain, E::ReconciledRunning) => S::Running,
            (S::Uncertain, E::ReconciledCompleted) => S::Completed,
            (S::Uncertain, E::ReconciledFailed) => S::Failed,
            (S::Submitted | S::Running, E::RemoteCompleted) => S::Completed,
            (S::Submitted | S::Running, E::RemoteFailed) => S::Failed,
            (S::Submitted | S::Running | S::Uncertain, E::StopRequested) => S::Stopping,
            (S::Stopping, E::StopConfirmed) => S::Stopped,
            (S::Stopped | S::Completed | S::Failed, E::Collected) => S::Collected,
            (s, E::Abandon) if !s.is_terminal() => S::Abandoned,
            (from, _) => {
                return Err(DomainError::InvalidStateTransition {
                    from: format!("{from:?}"),
                    to: format!("{event:?}"),
                })
            }
        };
        if let E::SubmitAcked(r) = event {
            self.external_ref = Some(r);
            self.apply_resource(ResourceEvent::Observed)?;
        }
        self.exec_state = next;
        Ok(())
    }

    pub fn apply_resource(&mut self, event: ResourceEvent) -> Result<(), DomainError> {
        use ResourceEvent as E;
        use ResourceState as R;
        self.resource_state = match (self.resource_state, event) {
            (R::Unknown, E::Observed) => R::Held,
            (R::Held | R::Unknown | R::ReleaseFailed, E::ReleaseRequested) => R::ReleaseRequested,
            (R::ReleaseRequested, E::ReleaseConfirmed) => R::ReleaseConfirmed,
            (R::ReleaseRequested, E::ReleaseFailed) => R::ReleaseFailed,
            (from, ev) => {
                return Err(DomainError::InvalidStateTransition {
                    from: format!("{from:?}"),
                    to: format!("{ev:?}"),
                })
            }
        };
        Ok(())
    }

    /// True while compute may still be billing: a stopped job can still hold a pod.
    pub fn may_still_incur_cost(&self) -> bool {
        !matches!(self.exec_state, ExecState::Planned | ExecState::Preflighted)
            && self.resource_state != ResourceState::ReleaseConfirmed
    }

    /// Bump generation; the receiving store must reject writes from older generations.
    pub fn fence(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }
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
            return Err(invalid("Progress must satisfy protected <= local <= observed"));
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
    let gap = progress.observed.unwrap_or(0).saturating_sub(progress.protected.unwrap_or(0));
    if gap > policy.max_unprotected_steps || (gap > 0 && secs_since_protected > policy.max_unprotected_secs) {
        ProtectionStatus::AtRisk
    } else {
        ProtectionStatus::Protected
    }
}

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
    fn group(&self, g: CheckGroup) -> impl Iterator<Item = &Check> {
        self.checks.iter().filter(move |c| c.group == g)
    }

    /// Group is satisfied only if it exists and every check passed.
    fn all_pass(&self, g: CheckGroup) -> bool {
        let mut it = self.group(g).peekable();
        it.peek().is_some() && it.all(|c| c.status == CheckStatus::Pass)
    }

    fn any_fail(&self, g: CheckGroup) -> bool {
        self.group(g).any(|c| c.status == CheckStatus::Fail)
    }

    pub fn unverified(&self) -> Vec<&Check> {
        self.checks.iter().filter(|c| c.status == CheckStatus::Unverified).collect()
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
    StateResumeSupported { profile: String },
    /// Recipe/environment deltas; optimizer dynamics may differ.
    AdaptedContinuation { deltas: Vec<String> },
    WeightsOnlyWarmStart,
    Unsupported { reasons: Vec<String> },
}

fn has_weights(m: &TrainingCheckpointManifest) -> bool {
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
    let unsupported = |r: &str| ResumeVerdict::Unsupported { reasons: vec![r.into()] };

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
        return ResumeVerdict::StateResumeSupported { profile: report.profile.clone() };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{CheckpointFileEntry, ResumabilityLevel};

    fn digest() -> String { "a".repeat(64) }

    fn file(path: &str, req: bool) -> CheckpointFileEntry {
        CheckpointFileEntry { path: path.into(), size_bytes: 1, digest: digest(), required_for_state_resume: req }
    }

    fn manifest() -> TrainingCheckpointManifest {
        let mut m = TrainingCheckpointManifest::new("ck1", 50, 0.5, "file:///ck/1", digest())
            .with_commit_status(CheckpointCommitStatus::CommittedProtected)
            .with_full_state("m@r", "e@r", "cursor50");
        for (p, r) in [("model.safetensors", true), ("optimizer.pt", true), ("scheduler.pt", true),
                       ("rng_state.pth", true), ("trainer_state.json", true)] {
            m = m.add_file(file(p, r));
        }
        m
    }

    fn check(id: &str, g: CheckGroup, s: CheckStatus) -> Check {
        Check { id: id.into(), group: g, status: s, evidence: String::new() }
    }

    fn report(statuses: [(CheckGroup, CheckStatus); 5]) -> VerificationReport {
        VerificationReport {
            profile: "hf-trainer/cpu".into(),
            checks: statuses.iter().enumerate().map(|(i, (g, s))| check(&format!("c{i}"), *g, *s)).collect(),
        }
    }

    use CheckGroup as G;
    use CheckStatus::{Fail, Pass, Unverified};
    const CAPS: TargetCaps = TargetCaps { supports_state_restore: true };

    fn all_pass() -> VerificationReport {
        report([(G::Identity, Pass), (G::Integrity, Pass), (G::State, Pass), (G::DataContinuity, Pass), (G::Environment, Pass)])
    }

    fn attempt() -> ExecutionAttempt {
        ExecutionAttempt::new("exp", "rcp", ComputeBackendKind::Local, None)
    }

    fn job() -> ExternalJobRef { ExternalJobRef::new("acct", "job", "v3").unwrap() }

    #[test]
    fn external_ref_rejects_latest_and_blanks() {
        assert!(ExternalJobRef::new("a", "j", "latest").is_err());
        assert!(ExternalJobRef::new("a", "j", "LATEST").is_err());
        assert!(ExternalJobRef::new("", "j", "v1").is_err());
        assert!(ExternalJobRef::new("a", "j", "v1").is_ok());
    }

    #[test]
    fn happy_path_and_resource_axis_are_independent() {
        let mut a = attempt();
        for e in [ExecEvent::PreflightOk, ExecEvent::SubmitStarted, ExecEvent::SubmitAcked(job()),
                  ExecEvent::ObservedRunning, ExecEvent::StopRequested, ExecEvent::StopConfirmed] {
            a.apply(e).unwrap();
        }
        // Stopped, but compute still held and possibly billing.
        assert_eq!(a.exec_state, ExecState::Stopped);
        assert_eq!(a.resource_state, ResourceState::Held);
        assert!(a.may_still_incur_cost());
        a.apply_resource(ResourceEvent::ReleaseRequested).unwrap();
        a.apply_resource(ResourceEvent::ReleaseFailed).unwrap();
        assert!(a.may_still_incur_cost());
        a.apply_resource(ResourceEvent::ReleaseRequested).unwrap();
        a.apply_resource(ResourceEvent::ReleaseConfirmed).unwrap();
        assert!(!a.may_still_incur_cost());
        a.apply(ExecEvent::Collected).unwrap();
        assert!(a.exec_state.is_terminal());
    }

    #[test]
    fn lost_connection_during_submit_is_uncertain_not_failed() {
        let mut a = attempt();
        a.apply(ExecEvent::PreflightOk).unwrap();
        a.apply(ExecEvent::SubmitStarted).unwrap();
        a.apply(ExecEvent::ConnectionLost).unwrap();
        assert_eq!(a.exec_state, ExecState::Uncertain);
        assert!(a.external_ref.is_none());
        // No path from Uncertain back to Submitting: resubmitting blind is impossible.
        assert!(a.apply(ExecEvent::SubmitStarted).is_err());
        a.apply(ExecEvent::ReconciledRunning).unwrap();
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let mut a = attempt();
        assert!(a.apply(ExecEvent::ObservedRunning).is_err());
        assert!(a.apply(ExecEvent::Collected).is_err());
        assert!(a.apply_resource(ResourceEvent::ReleaseConfirmed).is_err());
        a.apply(ExecEvent::Abandon).unwrap();
        assert!(a.apply(ExecEvent::PreflightOk).is_err());
        assert!(a.apply(ExecEvent::Abandon).is_err());
    }

    #[test]
    fn fence_is_monotonic_and_prefix_is_per_attempt() {
        let mut a = attempt();
        assert_eq!((a.fence(), a.fence()), (1, 2));
        assert!(a.output_prefix.contains(&a.id));
        assert_ne!(attempt().output_prefix, attempt().output_prefix);
    }

    #[test]
    fn progress_ordering_and_recovery_point() {
        let p = ProgressTriplet { observed: Some(920), local: Some(900), protected: Some(850) };
        p.validate().unwrap();
        assert_eq!(p.recovery_point(), Some(850));
        assert!(ProgressTriplet { observed: Some(10), local: Some(20), protected: None }.validate().is_err());
        assert!(ProgressTriplet { observed: Some(20), local: Some(10), protected: Some(15) }.validate().is_err());
    }

    #[test]
    fn protection_status_boundaries() {
        let pol = ProtectionPolicy { max_unprotected_steps: 100, max_unprotected_secs: 600, max_inflight_uploads: 2 };
        let at = |o, p| ProgressTriplet { observed: Some(o), local: Some(o), protected: Some(p) };
        assert_eq!(protection_status(&at(950, 850), &pol, 10, 0), ProtectionStatus::Protected);
        assert_eq!(protection_status(&at(951, 850), &pol, 10, 0), ProtectionStatus::AtRisk);
        assert_eq!(protection_status(&at(860, 850), &pol, 601, 0), ProtectionStatus::AtRisk);
        // Nothing unprotected: age alone does not alarm.
        assert_eq!(protection_status(&at(850, 850), &pol, 9999, 0), ProtectionStatus::Protected);
        assert_eq!(protection_status(&at(850, 850), &pol, 0, 3), ProtectionStatus::Degraded);
    }

    #[test]
    fn verdict_state_resume_requires_every_group_to_pass() {
        assert_eq!(classify(&manifest(), &all_pass(), CAPS),
            ResumeVerdict::StateResumeSupported { profile: "hf-trainer/cpu".into() });
    }

    #[test]
    fn verdict_unverified_is_never_pass() {
        // D6: production cannot prove cursor continuity -> must not claim state resume.
        let r = report([(G::Identity, Pass), (G::Integrity, Pass), (G::State, Pass), (G::DataContinuity, Unverified), (G::Environment, Pass)]);
        assert_eq!(classify(&manifest(), &r, CAPS), ResumeVerdict::WeightsOnlyWarmStart);
        assert_eq!(r.unverified().len(), 1);
    }

    #[test]
    fn verdict_valid_empty_optimizer_state_is_accepted() {
        // D5: nothing in classify inspects optimizer norms; engine reports State=Pass.
        let mut r = all_pass();
        r.checks[2].evidence = "SGD without momentum: optimizer state has no tensors".into();
        assert!(matches!(classify(&manifest(), &r, CAPS), ResumeVerdict::StateResumeSupported { .. }));
    }

    #[test]
    fn verdict_integrity_or_identity_failure_is_unsupported() {
        let r = report([(G::Identity, Pass), (G::Integrity, Fail), (G::State, Pass), (G::DataContinuity, Pass), (G::Environment, Pass)]);
        assert!(matches!(classify(&manifest(), &r, CAPS), ResumeVerdict::Unsupported { .. }));
        let r = report([(G::Identity, Fail), (G::Integrity, Pass), (G::State, Pass), (G::DataContinuity, Pass), (G::Environment, Pass)]);
        assert!(matches!(classify(&manifest(), &r, CAPS), ResumeVerdict::Unsupported { .. }));
        // Missing integrity group entirely is also not trusted.
        let r = VerificationReport { profile: "p".into(), checks: vec![check("i", G::Identity, Pass)] };
        assert!(matches!(classify(&manifest(), &r, CAPS), ResumeVerdict::Unsupported { .. }));
    }

    #[test]
    fn verdict_environment_delta_is_adapted_not_equivalent() {
        let r = report([(G::Identity, Pass), (G::Integrity, Pass), (G::State, Pass), (G::DataContinuity, Pass), (G::Environment, Fail)]);
        assert_eq!(classify(&manifest(), &r, CAPS), ResumeVerdict::AdaptedContinuation { deltas: vec!["c4".into()] });
    }

    #[test]
    fn verdict_downgrades_without_target_support_or_protection() {
        let no_restore = TargetCaps { supports_state_restore: false };
        assert_eq!(classify(&manifest(), &all_pass(), no_restore), ResumeVerdict::WeightsOnlyWarmStart);
        let local_only = manifest().with_commit_status(CheckpointCommitStatus::LocalStaged);
        assert!(matches!(classify(&local_only, &all_pass(), CAPS), ResumeVerdict::Unsupported { .. }));
        let mut weights_less = manifest();
        weights_less.files.retain(|f| f.path != "model.safetensors");
        weights_less.resumability_level = ResumabilityLevel::ExportOnly;
        assert!(matches!(classify(&weights_less, &all_pass(), CAPS), ResumeVerdict::Unsupported { .. }));
    }
}
