//! Sovereign Compute & Relay Contracts (RFC 005)
//!
//! Defines ports for managing heterogeneous compute backends,
//! cloud pod leases, checkpoint verification, and failover adaptation.

use async_trait::async_trait;
use custos_domain::{
    AcceleratorSpec, CloudPodLease, CompatibilityGateEvaluation, CompatibilityVerdict,
    ComputeBackendKind, DomainError, HardwareAdaptationPlan, TrainingCheckpointManifest,
};

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

    /// Evaluates hardware compatibility gate without silent precision/batch mutation
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
            _ => eval.plan.ok_or_else(|| {
                DomainError::Validation("No adaptation plan available for target".to_string())
            }),
        }
    }
}
