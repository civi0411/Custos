use super::*;
use crate::compute::{
    CheckpointCommitStatus, CheckpointFileEntry, ComputeBackendKind, ResumabilityLevel,
    TrainingCheckpointManifest,
};

fn digest() -> String {
    "a".repeat(64)
}

fn file(path: &str, req: bool) -> CheckpointFileEntry {
    CheckpointFileEntry {
        path: path.into(),
        size_bytes: 1,
        digest: digest(),
        required_for_state_resume: req,
    }
}

fn manifest() -> TrainingCheckpointManifest {
    let mut m = TrainingCheckpointManifest::new("ck1", 50, 0.5, "file:///ck/1", digest())
        .with_commit_status(CheckpointCommitStatus::CommittedProtected)
        .with_full_state("m@r", "e@r", "cursor50");
    for (p, r) in [
        ("model.safetensors", true),
        ("optimizer.pt", true),
        ("scheduler.pt", true),
        ("rng_state.pth", true),
        ("trainer_state.json", true),
    ] {
        m = m.add_file(file(p, r));
    }
    m
}

fn check(id: &str, g: CheckGroup, s: CheckStatus) -> Check {
    Check {
        id: id.into(),
        group: g,
        status: s,
        evidence: String::new(),
    }
}

fn report(statuses: [(CheckGroup, CheckStatus); 5]) -> VerificationReport {
    VerificationReport {
        profile: "hf-trainer/cpu".into(),
        checks: statuses
            .iter()
            .enumerate()
            .map(|(i, (g, s))| check(&format!("c{i}"), *g, *s))
            .collect(),
    }
}

use CheckGroup as G;
use CheckStatus::{Fail, Pass, Unverified};
const CAPS: TargetCaps = TargetCaps {
    supports_state_restore: true,
};

fn all_pass() -> VerificationReport {
    report([
        (G::Identity, Pass),
        (G::Integrity, Pass),
        (G::State, Pass),
        (G::DataContinuity, Pass),
        (G::Environment, Pass),
    ])
}

fn attempt() -> ExecutionAttempt {
    ExecutionAttempt::new("exp", "rcp", ComputeBackendKind::Local, None)
}

fn job() -> ExternalJobRef {
    ExternalJobRef::new("acct", "job", "v3").unwrap()
}

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
    for e in [
        ExecEvent::PreflightOk,
        ExecEvent::SubmitStarted,
        ExecEvent::SubmitAcked(job()),
        ExecEvent::ObservedRunning,
        ExecEvent::StopRequested,
        ExecEvent::StopConfirmed,
    ] {
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
    let p = ProgressTriplet {
        observed: Some(920),
        local: Some(900),
        protected: Some(850),
    };
    p.validate().unwrap();
    assert_eq!(p.recovery_point(), Some(850));
    assert!(ProgressTriplet {
        observed: Some(10),
        local: Some(20),
        protected: None
    }
    .validate()
    .is_err());
    assert!(ProgressTriplet {
        observed: Some(20),
        local: Some(10),
        protected: Some(15)
    }
    .validate()
    .is_err());
}

#[test]
fn protection_status_boundaries() {
    let pol = ProtectionPolicy {
        max_unprotected_steps: 100,
        max_unprotected_secs: 600,
        max_inflight_uploads: 2,
    };
    let at = |o, p| ProgressTriplet {
        observed: Some(o),
        local: Some(o),
        protected: Some(p),
    };
    assert_eq!(
        protection_status(&at(950, 850), &pol, 10, 0),
        ProtectionStatus::Protected
    );
    assert_eq!(
        protection_status(&at(951, 850), &pol, 10, 0),
        ProtectionStatus::AtRisk
    );
    assert_eq!(
        protection_status(&at(860, 850), &pol, 601, 0),
        ProtectionStatus::AtRisk
    );
    // Nothing unprotected: age alone does not alarm.
    assert_eq!(
        protection_status(&at(850, 850), &pol, 9999, 0),
        ProtectionStatus::Protected
    );
    assert_eq!(
        protection_status(&at(850, 850), &pol, 0, 3),
        ProtectionStatus::Degraded
    );
}

#[test]
fn verdict_state_resume_requires_every_group_to_pass() {
    assert_eq!(
        classify(&manifest(), &all_pass(), CAPS),
        ResumeVerdict::StateResumeSupported {
            profile: "hf-trainer/cpu".into()
        }
    );
}

#[test]
fn verdict_unverified_is_never_pass() {
    // D6: production cannot prove cursor continuity -> must not claim state resume.
    let r = report([
        (G::Identity, Pass),
        (G::Integrity, Pass),
        (G::State, Pass),
        (G::DataContinuity, Unverified),
        (G::Environment, Pass),
    ]);
    assert_eq!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::WeightsOnlyWarmStart
    );
    assert_eq!(r.unverified().len(), 1);
}

#[test]
fn verdict_valid_empty_optimizer_state_is_accepted() {
    // D5: nothing in classify inspects optimizer norms; engine reports State=Pass.
    let mut r = all_pass();
    r.checks[2].evidence = "SGD without momentum: optimizer state has no tensors".into();
    assert!(matches!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::StateResumeSupported { .. }
    ));
}

#[test]
fn verdict_integrity_or_identity_failure_is_unsupported() {
    let r = report([
        (G::Identity, Pass),
        (G::Integrity, Fail),
        (G::State, Pass),
        (G::DataContinuity, Pass),
        (G::Environment, Pass),
    ]);
    assert!(matches!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::Unsupported { .. }
    ));
    let r = report([
        (G::Identity, Fail),
        (G::Integrity, Pass),
        (G::State, Pass),
        (G::DataContinuity, Pass),
        (G::Environment, Pass),
    ]);
    assert!(matches!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::Unsupported { .. }
    ));
    // Missing integrity group entirely is also not trusted.
    let r = VerificationReport {
        profile: "p".into(),
        checks: vec![check("i", G::Identity, Pass)],
    };
    assert!(matches!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::Unsupported { .. }
    ));
}

#[test]
fn verdict_environment_delta_is_adapted_not_equivalent() {
    let r = report([
        (G::Identity, Pass),
        (G::Integrity, Pass),
        (G::State, Pass),
        (G::DataContinuity, Pass),
        (G::Environment, Fail),
    ]);
    assert_eq!(
        classify(&manifest(), &r, CAPS),
        ResumeVerdict::AdaptedContinuation {
            deltas: vec!["c4".into()]
        }
    );
}

#[test]
fn verdict_downgrades_without_target_support_or_protection() {
    let no_restore = TargetCaps {
        supports_state_restore: false,
    };
    assert_eq!(
        classify(&manifest(), &all_pass(), no_restore),
        ResumeVerdict::WeightsOnlyWarmStart
    );
    let local_only = manifest().with_commit_status(CheckpointCommitStatus::LocalStaged);
    assert!(matches!(
        classify(&local_only, &all_pass(), CAPS),
        ResumeVerdict::Unsupported { .. }
    ));
    let mut weights_less = manifest();
    weights_less.files.retain(|f| f.path != "model.safetensors");
    weights_less.resumability_level = ResumabilityLevel::ExportOnly;
    assert!(matches!(
        classify(&weights_less, &all_pass(), CAPS),
        ResumeVerdict::Unsupported { .. }
    ));
}
