//! Remote Execution Coordinator (SADE GPU-E)
//!
//! Orchestrates the end-to-end lifecycle of supervised training and fine-tuning
//! on remote compute targets (RunPod, Vast.ai, Dedicated SSH).
//!
//! Enforces:
//! 1. Preflight validation before compute allocation.
//! 2. Exclusive workload admission lease on compute targets.
//! 3. Idempotent submission with stable external references.
//! 4. Decoupled process stop and compute release to prevent billing leakage.
//! 5. Disconnection tolerance with honest uncertain state transitions (no blind re-submits).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_core::contracts::experiment::{
    AttemptObservation, ExecutionTargetPort, PreflightReport, TargetCapabilities,
};
use custos_domain::{
    ComputeWorkloadKind, DomainError, ExecEvent, ExecState, ExecutionArtifact, ExecutionAttempt,
    ExternalJobRef, ProgressTriplet, ResourceEvent, WorkloadAdmissionRequest,
    WorkloadAdmissionVerdict,
};

/// Coordinator for remote execution attempts across heterogeneous GPU targets
#[derive(Clone)]
pub struct RemoteExecutionCoordinator {
    admission: Arc<dyn WorkloadAdmissionPort>,
    target: Arc<dyn ExecutionTargetPort>,
    active_leases: Arc<Mutex<HashMap<String, String>>>, // attempt_id -> lease_id
}

impl RemoteExecutionCoordinator {
    pub fn new(
        admission: Arc<dyn WorkloadAdmissionPort>,
        target: Arc<dyn ExecutionTargetPort>,
    ) -> Self {
        Self {
            admission,
            target,
            active_leases: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Queries target hardware and control capabilities
    pub async fn capabilities(&self) -> TargetCapabilities {
        self.target.capabilities().await
    }

    /// Performs preflight validation for a recipe before allocating compute
    pub async fn preflight(&self, recipe_id: &str) -> Result<PreflightReport, DomainError> {
        self.target.preflight(recipe_id).await
    }

    /// Requests exclusive admission and submits execution attempt to remote target
    pub async fn admit_and_submit(
        &self,
        attempt: &mut ExecutionAttempt,
        preferred_device_id: Option<String>,
        idempotency_key: &str,
    ) -> Result<ExternalJobRef, DomainError> {
        // 1. Target preflight check
        let preflight = self.target.preflight(&attempt.recipe_id).await?;
        if !preflight.allowed {
            return Err(DomainError::Validation(format!(
                "Remote preflight blocked: {}",
                preflight.blockers.join("; ")
            )));
        }
        attempt.apply(ExecEvent::PreflightOk)?;

        // 2. Hardware admission lease (Exclusive accelerator lock)
        let admission_req = WorkloadAdmissionRequest {
            workload_kind: ComputeWorkloadKind::TrainingRun,
            owner_id: attempt.id.clone(),
            preferred_device_id,
            require_exclusive: true,
            required_memory_mb: Some(16384),
        };

        let verdict = self.admission.admit_workload(&admission_req).await?;
        let lease = match verdict {
            WorkloadAdmissionVerdict::Admitted { lease } => lease,
            WorkloadAdmissionVerdict::Queued {
                queue_position,
                reason,
            } => {
                let _ = attempt.apply(ExecEvent::SubmitRejected);
                return Err(DomainError::Conflict(format!(
                    "Remote compute admission queued at position {queue_position}: {reason}"
                )));
            }
            WorkloadAdmissionVerdict::Rejected { reason } => {
                let _ = attempt.apply(ExecEvent::SubmitRejected);
                return Err(DomainError::Validation(format!(
                    "Remote compute admission rejected: {reason}"
                )));
            }
        };

        let lease_id = lease.lease_id.clone();
        self.active_leases
            .lock()
            .await
            .insert(attempt.id.clone(), lease_id.clone());

        // 3. Submit attempt to target
        attempt.apply(ExecEvent::SubmitStarted)?;
        match self.target.submit(attempt, idempotency_key).await {
            Ok(ext_ref) => {
                attempt.apply(ExecEvent::SubmitAcked(ext_ref.clone()))?;
                attempt.external_ref = Some(ext_ref.clone());
                Ok(ext_ref)
            }
            Err(e) => {
                let _ = attempt.apply(ExecEvent::SubmitRejected);
                self.cleanup_lease(&attempt.id).await;
                Err(e)
            }
        }
    }

    /// Observes remote execution progress and reconciles attempt states
    pub async fn observe_and_reconcile(
        &self,
        attempt: &mut ExecutionAttempt,
    ) -> Result<AttemptObservation, DomainError> {
        let ext_ref = match attempt.external_ref.as_ref() {
            Some(r) => r.clone(),
            None => {
                return Err(DomainError::Validation(
                    "Attempt does not have an external job reference to observe".into(),
                ))
            }
        };

        match self.target.observe(&ext_ref).await {
            Ok(obs) => {
                match obs.exec_state {
                    ExecState::Running => {
                        let _ = attempt
                            .apply(ExecEvent::ObservedRunning)
                            .or_else(|_| attempt.apply(ExecEvent::ReconciledRunning));
                    }
                    ExecState::Completed => {
                        let _ = attempt
                            .apply(ExecEvent::RemoteCompleted)
                            .or_else(|_| attempt.apply(ExecEvent::ReconciledCompleted));
                        self.cleanup_lease(&attempt.id).await;
                    }
                    ExecState::Failed => {
                        let _ = attempt
                            .apply(ExecEvent::RemoteFailed)
                            .or_else(|_| attempt.apply(ExecEvent::ReconciledFailed));
                        self.cleanup_lease(&attempt.id).await;
                    }
                    ExecState::Stopped => {
                        let _ = attempt.apply(ExecEvent::StopConfirmed);
                    }
                    ExecState::Uncertain => {
                        let _ = attempt.apply(ExecEvent::ConnectionLost);
                    }
                    _ => {}
                }
                attempt.exec_state = obs.exec_state;
                attempt.resource_state = obs.resource_state;
                Ok(obs)
            }
            Err(_) => {
                // Network disconnect or probe failure: transition to Uncertain
                let _ = attempt.apply(ExecEvent::ConnectionLost);
                attempt.exec_state = ExecState::Uncertain;
                Ok(AttemptObservation {
                    exec_state: ExecState::Uncertain,
                    resource_state: attempt.resource_state,
                    progress: ProgressTriplet::default(),
                    exit_code: None,
                })
            }
        }
    }

    /// Decoupled Stop: Terminates remote process group without destroying cloud pod
    pub async fn stop_job(&self, attempt: &mut ExecutionAttempt) -> Result<(), DomainError> {
        let ext_ref = match attempt.external_ref.as_ref() {
            Some(r) => r.clone(),
            None => {
                return Err(DomainError::Validation(
                    "Attempt does not have an external job reference to stop".into(),
                ))
            }
        };

        self.target.stop_job(&ext_ref).await?;
        attempt.apply(ExecEvent::StopRequested)?;
        attempt.apply(ExecEvent::StopConfirmed)?;
        // Note: compute resource remains Held until explicit release!
        Ok(())
    }

    /// Decoupled Release: Destroys remote cloud pod or deallocates compute lease
    pub async fn release_compute(&self, attempt: &mut ExecutionAttempt) -> Result<(), DomainError> {
        let ext_ref = match attempt.external_ref.as_ref() {
            Some(r) => r.clone(),
            None => {
                return Err(DomainError::Validation(
                    "Attempt does not have an external job reference to release".into(),
                ))
            }
        };

        self.target.release_compute(&ext_ref).await?;
        attempt.apply_resource(ResourceEvent::ReleaseRequested)?;
        attempt.apply_resource(ResourceEvent::ReleaseConfirmed)?;

        self.cleanup_lease(&attempt.id).await;
        Ok(())
    }

    /// Collects final artifacts from remote target and marks attempt collected
    pub async fn collect_artifacts(
        &self,
        attempt: &mut ExecutionAttempt,
    ) -> Result<Vec<ExecutionArtifact>, DomainError> {
        let ext_ref = match attempt.external_ref.as_ref() {
            Some(r) => r.clone(),
            None => {
                return Err(DomainError::Validation(
                    "Attempt does not have an external job reference to collect".into(),
                ))
            }
        };

        let artifacts = self.target.collect_artifacts(&ext_ref).await?;
        attempt.apply(ExecEvent::Collected)?;
        self.cleanup_lease(&attempt.id).await;
        Ok(artifacts)
    }

    async fn cleanup_lease(&self, attempt_id: &str) {
        let mut leases = self.active_leases.lock().await;
        if let Some(lease_id) = leases.remove(attempt_id) {
            let _ = self.admission.release_lease(&lease_id).await;
        }
    }
}
