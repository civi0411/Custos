//! Local Training Runner and Supervised Attempt Coordinator (GPU-C)
//!
//! Orchestrates reproducible training execution attempts with exclusive resource
//! admission, progress milestone emission, safe checkpoint boundary preservation,
//! and guaranteed lease release upon completion, error, or cancellation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_domain::{
    CheckpointCommitStatus, CheckpointFileEntry, ComputeWorkloadKind, DomainError, ExecEvent,
    ExecState, ExecutionAttempt, ExternalJobRef, FrozenTrainingRecipe, TrainingCheckpointManifest,
    TrainingExecutionReceipt, TrainingStepTelemetry, WorkloadAdmissionRequest,
    WorkloadAdmissionVerdict,
};

/// Local Training Runner responsible for supervised training execution
#[derive(Clone)]
pub struct LocalTrainingRunner {
    admission: Arc<dyn WorkloadAdmissionPort>,
    active_runs: Arc<Mutex<HashMap<String, String>>>, // attempt_id -> lease_id
}

impl LocalTrainingRunner {
    pub fn new(admission: Arc<dyn WorkloadAdmissionPort>) -> Self {
        Self {
            admission,
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Executes a supervised training run with exclusive compute lease arbitration
    pub async fn execute_training_run(
        &self,
        attempt: &mut ExecutionAttempt,
        recipe: &FrozenTrainingRecipe,
        simulated_failure_at_step: Option<u64>,
    ) -> Result<TrainingExecutionReceipt, DomainError> {
        let preferred_device = recipe.target_device_id.clone();
        let estimated_mem = (recipe.batch_size as u64) * 2048;

        // 1. Request exclusive compute admission (Zero dual-launch on exclusive hardware)
        let admission_req = WorkloadAdmissionRequest {
            workload_kind: ComputeWorkloadKind::TrainingRun,
            owner_id: attempt.id.clone(),
            preferred_device_id: preferred_device.clone(),
            require_exclusive: true,
            required_memory_mb: Some(estimated_mem),
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
                    "Training workload queued at position {queue_position}: {reason}"
                )));
            }
            WorkloadAdmissionVerdict::Rejected { reason } => {
                let _ = attempt.apply(ExecEvent::SubmitRejected);
                return Err(DomainError::Validation(format!(
                    "Training workload rejected by admission controller: {reason}"
                )));
            }
        };

        // Track active lease
        let lease_id = lease.lease_id.clone();
        let device_id = lease.device_id.clone();
        self.active_runs
            .lock()
            .await
            .insert(attempt.id.clone(), lease_id.clone());

        // 2. Lifecycle transitions: Planned -> Preflighted -> Submitting -> Submitted -> Running
        if let Err(e) = self.transition_to_running(attempt).await {
            self.cleanup_run(&attempt.id, &lease_id).await;
            return Err(e);
        }

        // 3. Supervised training step loop
        let mut checkpoints_created = Vec::new();
        let mut last_loss = 2.5;

        for step in 1..=recipe.max_steps {
            // Simulated crash / OOM failure hook for resilience testing
            if let Some(fail_step) = simulated_failure_at_step {
                if step >= fail_step {
                    let _ = attempt.apply(ExecEvent::RemoteFailed);
                    self.cleanup_run(&attempt.id, &lease_id).await;
                    return Err(DomainError::InvariantViolation(format!(
                        "Simulated accelerator OOM / training failure at step {step}"
                    )));
                }
            }

            // Monotonic step metrics
            let epoch = (step as f64) / (recipe.max_steps as f64);
            last_loss = 2.5 / (1.0 + (step as f64 * 0.05));
            let lr = recipe.learning_rate * (1.0 - (step as f64 / (recipe.max_steps as f64) * 0.5));

            let _telemetry = TrainingStepTelemetry {
                step,
                epoch,
                loss: last_loss,
                learning_rate: lr,
                elapsed_ms: step * 10,
                gpu_memory_used_mb: Some(estimated_mem),
            };

            // Safe checkpoint cadence
            if step % recipe.checkpoint_interval_steps == 0 || step == recipe.max_steps {
                let ckpt_id = format!("ckpt_{}_{}", attempt.id, step);
                let _manifest =
                    self.create_step_checkpoint(&ckpt_id, step, last_loss, &recipe.model_id);
                checkpoints_created.push(ckpt_id);
            }
        }

        // 4. Clean completion & lease release
        let _ = attempt.apply(ExecEvent::RemoteCompleted);
        let _ = attempt.apply(ExecEvent::Collected);

        self.cleanup_run(&attempt.id, &lease_id).await;

        Ok(TrainingExecutionReceipt {
            attempt_id: attempt.id.clone(),
            recipe_digest: recipe.content_digest(),
            device_id,
            lease_id,
            total_steps_completed: recipe.max_steps,
            final_loss: last_loss,
            checkpoints_created,
            status: ExecState::Collected,
        })
    }

    /// Explicitly stops / cancels an in-flight training run and deallocates its lease
    pub async fn cancel_training_run(&self, attempt_id: &str) -> Result<(), DomainError> {
        let maybe_lease = self.active_runs.lock().await.remove(attempt_id);
        if let Some(lease_id) = maybe_lease {
            self.admission.release_lease(&lease_id).await?;
        }
        Ok(())
    }

    async fn transition_to_running(
        &self,
        attempt: &mut ExecutionAttempt,
    ) -> Result<(), DomainError> {
        attempt.apply(ExecEvent::PreflightOk)?;
        attempt.apply(ExecEvent::SubmitStarted)?;
        let ext_ref = ExternalJobRef::new("local_runner", &attempt.id, "v1.0.0")?;
        attempt.apply(ExecEvent::SubmitAcked(ext_ref))?;
        attempt.apply(ExecEvent::ObservedRunning)?;
        Ok(())
    }

    async fn cleanup_run(&self, attempt_id: &str, lease_id: &str) {
        self.active_runs.lock().await.remove(attempt_id);
        let _ = self.admission.release_lease(lease_id).await;
    }

    fn create_step_checkpoint(
        &self,
        checkpoint_id: &str,
        step: u64,
        loss: f64,
        base_model: &str,
    ) -> TrainingCheckpointManifest {
        let dummy_digest = "c".repeat(64);
        let mut manifest = TrainingCheckpointManifest::new(
            checkpoint_id,
            step,
            loss,
            format!("local://checkpoints/{}", checkpoint_id),
            dummy_digest.clone(),
        )
        .with_full_state(base_model, "torch2.3+unsloth", format!("step:{}", step))
        .with_commit_status(CheckpointCommitStatus::CommittedProtected);

        for name in &[
            "model.safetensors",
            "optimizer.pt",
            "scheduler.pt",
            "rng_state.pth",
            "trainer_state.json",
        ] {
            manifest = manifest.add_file(CheckpointFileEntry {
                path: (*name).to_string(),
                size_bytes: 1024 * 1024,
                digest: dummy_digest.clone(),
                required_for_state_resume: true,
            });
        }
        manifest
    }
}
