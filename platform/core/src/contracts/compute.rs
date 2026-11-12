//! Sovereign Compute & Relay Contracts (RFC 005)
//!
//! Defines ports for managing heterogeneous compute backends,
//! cloud pod leases, checkpoint verification, and failover adaptation.

use async_trait::async_trait;
use custos_domain::{
    AcceleratorSpec, CloudPodLease, CompatibilityGateEvaluation, CompatibilityVerdict,
    ComputeBackendKind, ComputeDeviceRef, DomainError, ExecState, ExecutionArtifact,
    ExecutionAttempt, ExternalJobRef, HardwareAdaptationPlan, ProgressTriplet, ResourceState,
    TrainingCheckpointManifest, WorkloadAdmissionRequest, WorkloadAdmissionVerdict, WorkloadLease,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct CloudPodRequest {
    pub provider: ComputeBackendKind,
    pub desired_accelerator: String,
    pub container_image: String,
    pub volume_size_gb: u32,
    pub max_budget_usd: Option<f32>,
    pub auto_stop_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeTelemetry {
    pub backend: ComputeBackendKind,
    pub host_id: String,
    pub gpu_utilization_percent: u8,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature_celsius: Option<u8>,
    pub power_watts: Option<u16>,
}

#[async_trait]
pub trait ComputePort: Send + Sync {
    /// Probes availability and hardware capability of a compute backend
    async fn probe_accelerator(
        &self,
        backend: ComputeBackendKind,
        target_ref: &str,
    ) -> Result<AcceleratorSpec, DomainError>;

    /// Fetches structured real-time GPU telemetry (NVML / nvidia-smi)
    async fn get_telemetry(
        &self,
        backend: ComputeBackendKind,
        target_ref: &str,
    ) -> Result<ComputeTelemetry, DomainError>;

    /// Allocates an on-demand cloud GPU pod with budget guardrails
    async fn allocate_pod(&self, request: &CloudPodRequest) -> Result<CloudPodLease, DomainError>;

    /// Stops or terminates an active cloud pod lease to prevent financial leakage
    async fn release_pod(&self, lease_id: &str, terminate: bool) -> Result<(), DomainError>;

    /// Returns a hardware proposal only; does not attest checkpoint bytes or admit execution.
    fn evaluate_compatibility(
        &self,
        source: &AcceleratorSpec,
        target: &AcceleratorSpec,
        source_batch: u32,
        source_grad_accum: u32,
        checkpoint: &TrainingCheckpointManifest,
    ) -> Result<CompatibilityGateEvaluation, DomainError> {
        Ok(HardwareAdaptationPlan::evaluate_gate(
            source,
            target,
            source_batch,
            source_grad_accum,
            checkpoint,
        ))
    }

    /// Computes hardware adaptation plan for cross-platform failover, rejecting incompatible targets
    fn plan_failover(
        &self,
        source: &AcceleratorSpec,
        target: &AcceleratorSpec,
        source_batch: u32,
        source_grad_accum: u32,
        checkpoint: &TrainingCheckpointManifest,
    ) -> Result<HardwareAdaptationPlan, DomainError> {
        checkpoint.validate_for_state_resume()?;
        let eval = self.evaluate_compatibility(
            source,
            target,
            source_batch,
            source_grad_accum,
            checkpoint,
        )?;
        match eval.verdict {
            CompatibilityVerdict::Incompatible { reasons } => {
                Err(DomainError::Validation(format!(
                    "Hardware failover rejected by compatibility gate: {}",
                    reasons.join("; ")
                )))
            }
            CompatibilityVerdict::RequiresAdaptation { warnings } => {
                Err(DomainError::Validation(format!(
                    "Adaptation requires an explicit recipe revision and engine validation: {}",
                    warnings.join("; ")
                )))
            }
            CompatibilityVerdict::Compatible => eval.plan.ok_or_else(|| {
                DomainError::Validation("No adaptation plan available for target".to_string())
            }),
        }
    }
}

/// Port for hardware resource inventory and workload admission arbitration (GPU-A / GPU-B)
#[async_trait]
pub trait WorkloadAdmissionPort: Send + Sync {
    /// Discovers all available compute devices on the local host or configured targets
    async fn discover_devices(&self) -> Result<Vec<ComputeDeviceRef>, DomainError>;

    /// Requests admission for a compute workload (notebook, training, eval, inference)
    async fn admit_workload(
        &self,
        request: &WorkloadAdmissionRequest,
    ) -> Result<WorkloadAdmissionVerdict, DomainError>;

    /// Releases an active workload lease
    async fn release_lease(&self, lease_id: &str) -> Result<(), DomainError>;

    /// Lists active and queued leases, optionally filtered by device
    async fn list_active_leases(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<WorkloadLease>, DomainError>;
}

/// Advertised hardware and control capabilities of a compute target
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetCapabilities {
    pub supports_interactive_control: bool,
    pub supports_checkpoint_request: bool,
    pub supports_direct_storage_upload: bool,
    pub supports_live_telemetry_streaming: bool,
    pub supports_graceful_stop: bool,
}

impl Default for TargetCapabilities {
    fn default() -> Self {
        Self {
            supports_interactive_control: false,
            supports_checkpoint_request: false,
            supports_direct_storage_upload: true,
            supports_live_telemetry_streaming: false,
            supports_graceful_stop: true,
        }
    }
}

/// Preflight admission report before submission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub allowed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl PreflightReport {
    pub fn ok() -> Self {
        Self {
            allowed: true,
            blockers: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn blocked(reasons: Vec<String>) -> Self {
        Self {
            allowed: false,
            blockers: reasons,
            warnings: Vec::new(),
        }
    }
}

/// Periodic or reconciled observation from a compute target
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptObservation {
    pub exec_state: ExecState,
    pub resource_state: ResourceState,
    pub progress: ProgressTriplet,
    pub exit_code: Option<i32>,
}

/// Discrete operations contract for remote or local compute targets (F10 / P3).
/// Decouples stop_job and release_compute to prevent cloud billing leakage.
#[async_trait]
pub trait ExecutionTargetPort: Send + Sync {
    async fn capabilities(&self) -> TargetCapabilities;
    async fn preflight(&self, recipe_id: &str) -> Result<PreflightReport, DomainError>;
    async fn submit(
        &self,
        attempt: &ExecutionAttempt,
        idempotency_key: &str,
    ) -> Result<ExternalJobRef, DomainError>;
    async fn observe(
        &self,
        external_ref: &ExternalJobRef,
    ) -> Result<AttemptObservation, DomainError>;
    async fn request_checkpoint(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;
    async fn stop_job(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;
    async fn release_compute(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;
    async fn collect_artifacts(
        &self,
        external_ref: &ExternalJobRef,
    ) -> Result<Vec<ExecutionArtifact>, DomainError>;
}

/// Factory / resolver for execution target adapters based on target kind and credentials.
pub trait ExecutionTargetResolver: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn resolve(
        &self,
        backend: ComputeBackendKind,
        work_dir: &Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
    ) -> Arc<dyn ExecutionTargetPort>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{CheckpointCommitStatus, CheckpointFileEntry};

    struct OfflineCompute;

    #[async_trait]
    impl ComputePort for OfflineCompute {
        async fn probe_accelerator(
            &self,
            _: ComputeBackendKind,
            _: &str,
        ) -> Result<AcceleratorSpec, DomainError> {
            Err(DomainError::Validation("offline test".into()))
        }
        async fn get_telemetry(
            &self,
            _: ComputeBackendKind,
            _: &str,
        ) -> Result<ComputeTelemetry, DomainError> {
            Err(DomainError::Validation("offline test".into()))
        }
        async fn allocate_pod(&self, _: &CloudPodRequest) -> Result<CloudPodLease, DomainError> {
            Err(DomainError::Validation("offline test".into()))
        }
        async fn release_pod(&self, _: &str, _: bool) -> Result<(), DomainError> {
            Err(DomainError::Validation("offline test".into()))
        }
    }

    #[test]
    fn failover_requires_protected_state_and_no_implicit_adaptation() {
        let source = AcceleratorSpec::a100_40gb();
        let mut checkpoint =
            TrainingCheckpointManifest::new("c400", 400, 1.0, "s3://b/c", "a".repeat(64))
                .with_full_state("model@revision", "engine@revision", "cursor400")
                .with_precision("bf16");
        for path in [
            "adapter_model.safetensors",
            "optimizer.pt",
            "scheduler.pt",
            "rng_state.pth",
            "trainer_state.json",
        ] {
            checkpoint.files.push(CheckpointFileEntry {
                path: path.into(),
                size_bytes: 1,
                digest: "a".repeat(64),
                required_for_state_resume: true,
            });
        }
        let port = OfflineCompute;
        assert!(port
            .plan_failover(&source, &source, 2, 2, &checkpoint)
            .is_err());
        checkpoint.commit_status = CheckpointCommitStatus::CommittedProtected;
        assert!(port
            .plan_failover(&source, &source, 2, 2, &checkpoint)
            .is_ok());
        let changed_precision =
            port.plan_failover(&source, &AcceleratorSpec::tesla_t4(), 2, 2, &checkpoint);
        assert!(
            matches!(changed_precision, Err(DomainError::Validation(message)) if message.contains("explicit recipe revision"))
        );
        let smaller_bf16 = AcceleratorSpec::new("small-bf16", 8192, true);
        assert!(port
            .plan_failover(&source, &smaller_bf16, 8, 2, &checkpoint)
            .is_err());
        checkpoint.optimizer_state_preserved = false;
        assert!(port
            .plan_failover(&source, &source, 2, 2, &checkpoint)
            .is_err());
    }
}
