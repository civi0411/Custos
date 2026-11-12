use serde::{Deserialize, Serialize};

use crate::compute::ComputeBackendKind;
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
        let r = Self {
            account_ref: account_ref.into(),
            job_id: job_id.into(),
            version_id: version_id.into(),
        };
        let bad = |s: &str| s.trim().is_empty();
        if bad(&r.account_ref)
            || bad(&r.job_id)
            || bad(&r.version_id)
            || r.version_id.trim().eq_ignore_ascii_case("latest")
        {
            return Err(invalid(
                "External job ref needs account, job and a pinned (non-'latest') version",
            ));
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
            (S::Submitting | S::Submitted | S::Running | S::Stopping, E::ConnectionLost) => {
                S::Uncertain
            }
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

