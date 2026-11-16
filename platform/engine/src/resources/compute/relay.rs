//! Checkpoint Relay & Heterogeneous Failover Coordinator (F10 / P3 / GPU-F).
//!
//! Orchestrates safe checkpoint continuity, cross-target handoff,
//! hardware adaptation planning, single-writer generation-CAS promotion fencing,
//! and crash recovery without data loss or silent divergence.

use custos_core::contracts::checkpoint_store::{CheckpointStorePort, PromotionOutcome};
use custos_domain::{
    new_id, AcceleratorSpec, CompatibilityVerdict, ComputeBackendKind, DomainError, ExecEvent,
    ExecState, ExecutionAttempt, HardwareAdaptationPlan, ResourceState, TrainingCheckpointManifest,
};

/// Coordinator for cross-platform checkpoint relay and failover
#[derive(Clone, Default)]
pub struct CheckpointRelayCoordinator;

impl CheckpointRelayCoordinator {
    pub fn new() -> Self {
        Self
    }

    /// Prepares a verified continuation attempt linked to a parent checkpoint
    /// and generates a hardware adaptation plan across heterogeneous accelerators.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_continuation_attempt(
        &self,
        parent_attempt: &ExecutionAttempt,
        target_backend: ComputeBackendKind,
        checkpoint: &TrainingCheckpointManifest,
        source_accelerator: &AcceleratorSpec,
        target_accelerator: &AcceleratorSpec,
        source_batch_size: u32,
        source_gradient_accumulation: u32,
    ) -> Result<(ExecutionAttempt, HardwareAdaptationPlan), DomainError> {
        // 1. Strict pre-resume state validation gate (reject uncommitted, incomplete, or corrupt bundles)
        checkpoint.validate_for_state_resume()?;

        // 2. Hardware compatibility evaluation gate
        let gate_eval = HardwareAdaptationPlan::evaluate_gate(
            source_accelerator,
            target_accelerator,
            source_batch_size,
            source_gradient_accumulation,
            checkpoint,
        );

        let plan = match gate_eval.verdict {
            CompatibilityVerdict::Incompatible { reasons } => {
                return Err(DomainError::Validation(format!(
                    "Hardware failover from '{}' to '{}' rejected by compatibility gate: {}",
                    source_accelerator.name,
                    target_accelerator.name,
                    reasons.join("; ")
                )));
            }
            CompatibilityVerdict::RequiresAdaptation { warnings: _ }
            | CompatibilityVerdict::Compatible => gate_eval.plan.ok_or_else(|| {
                DomainError::Validation(
                    "No adaptation plan generated for compatible hardware target".into(),
                )
            })?,
        };

        // 3. Child attempt creation with incremented generation (anti-split-brain fencing)
        let child_id = new_id("attempt");
        let child_attempt = ExecutionAttempt {
            output_prefix: format!(
                "experiments/{}/attempts/{}/",
                parent_attempt.experiment_id, child_id
            ),
            id: child_id,
            experiment_id: parent_attempt.experiment_id.clone(),
            recipe_id: parent_attempt.recipe_id.clone(),
            backend: target_backend,
            external_ref: None,
            exec_state: ExecState::Planned,
            resource_state: ResourceState::Unknown,
            parent_checkpoint_id: Some(checkpoint.checkpoint_id.clone()),
            generation: parent_attempt.generation + 1,
        };

        Ok((child_attempt, plan))
    }

    /// Promotes a checkpoint with generation CAS fencing, rejecting stale writers
    pub async fn promote_checkpoint_with_fencing(
        &self,
        store: &dyn CheckpointStorePort,
        experiment_id: &str,
        checkpoint_id: &str,
        generation: u64,
    ) -> Result<PromotionOutcome, DomainError> {
        store
            .promote(experiment_id, checkpoint_id, generation)
            .await
    }

    /// Hands off a failed/crashed attempt to a fallback compute target using the last protected checkpoint
    #[allow(clippy::too_many_arguments)]
    pub fn handoff_crashed_attempt(
        &self,
        crashed_attempt: &mut ExecutionAttempt,
        fallback_backend: ComputeBackendKind,
        last_protected_checkpoint: &TrainingCheckpointManifest,
        source_accelerator: &AcceleratorSpec,
        target_accelerator: &AcceleratorSpec,
        source_batch_size: u32,
        source_gradient_accumulation: u32,
    ) -> Result<(ExecutionAttempt, HardwareAdaptationPlan), DomainError> {
        // 1. Mark crashed attempt as failed if not already terminal
        if matches!(
            crashed_attempt.exec_state,
            ExecState::Submitting
                | ExecState::Submitted
                | ExecState::Running
                | ExecState::Uncertain
        ) {
            let _ = crashed_attempt.apply(ExecEvent::RemoteFailed);
        }

        // 2. Prepare continuation child attempt
        self.prepare_continuation_attempt(
            crashed_attempt,
            fallback_backend,
            last_protected_checkpoint,
            source_accelerator,
            target_accelerator,
            source_batch_size,
            source_gradient_accumulation,
        )
    }
}
