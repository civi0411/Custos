//! Optional Consumer & Acceleration Placement Engine (GPU-G / F11).
//!
//! Evaluates placement decisions for optional compute consumers such as:
//! - Background embedding and vector indexing (`ComputeWorkloadKind::BackgroundEmbedding`)
//! - Optical Character Recognition and document transcription (`ComputeWorkloadKind::DocumentOcr`)
//!
//! Core Invariants:
//! 1. Cooperative Prioritization: Interactive kernels and training runs strictly outrank background consumers.
//! 2. Fail-Closed Egress Protection: Local datasets cannot be egressed to cloud targets without explicit consent.
//! 3. Graceful CPU Fallback: When an accelerator is held exclusively, background work drops to CPU rather than stalling or competing for VRAM.

use custos_domain::{
    ComputeDeviceRef, ComputeWorkloadKind, DeviceKind, DomainError, OptionalConsumerPlacement,
    WorkloadLease, WorkloadLeaseStatus,
};

/// Placement engine evaluating execution target and batch bounds for optional consumers
#[derive(Debug, Default, Clone)]
pub struct OptionalConsumerPlacementEngine;

impl OptionalConsumerPlacementEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates execution placement for a background consumer against active fleet leases
    pub fn evaluate_placement(
        &self,
        workload_kind: ComputeWorkloadKind,
        requested_device_id: Option<&str>,
        active_leases: &[WorkloadLease],
        available_devices: &[ComputeDeviceRef],
        allow_cloud_egress: bool,
        _corpus_item_count: usize,
    ) -> Result<OptionalConsumerPlacement, DomainError> {
        // 1. Only cooperative background workloads are governed by this engine
        if !workload_kind.is_cooperative_background() {
            return Err(DomainError::Validation(
                "Only cooperative background workloads (embedding, OCR) can be evaluated by placement engine"
                    .to_string(),
            ));
        }

        // 2. Cloud egress guardrail: prohibit egressing local datasets without explicit consent
        if let Some(target_id) = requested_device_id {
            if (target_id.contains("colab")
                || target_id.contains("kaggle")
                || target_id.contains("cloud")
                || target_id.contains("rented"))
                && !allow_cloud_egress
            {
                return Err(DomainError::Validation(
                    "Cloud egress prohibited for local dataset indexing / OCR without explicit user consent"
                        .to_string(),
                ));
            }
        }

        // 3. Locate candidate compute device
        let cpu_device = available_devices
            .iter()
            .find(|d| d.device_kind == DeviceKind::Cpu)
            .map(|d| d.device_id.clone())
            .unwrap_or_else(|| "dev_cpu_0".to_string());

        let candidate_gpu = if let Some(target_id) = requested_device_id {
            available_devices.iter().find(|d| d.device_id == target_id)
        } else {
            available_devices
                .iter()
                .find(|d| d.device_kind == DeviceKind::Cuda || d.device_kind == DeviceKind::Mps)
        };

        // If no GPU is available or requested, fall back cleanly to CPU
        let target_gpu = match candidate_gpu {
            Some(gpu) if gpu.device_kind != DeviceKind::Cpu => gpu,
            _ => {
                return Ok(OptionalConsumerPlacement {
                    workload_kind,
                    target_device_id: cpu_device,
                    is_accelerated: false,
                    batch_size: 16,
                    allows_cloud_egress: false,
                    reason: "No accelerator available; routed to deterministic CPU execution"
                        .to_string(),
                });
            }
        };

        // 4. Check whether candidate GPU is held by an exclusive lease (e.g. TrainingRun)
        let is_held_exclusively = active_leases.iter().any(|lease| {
            lease.device_id == target_gpu.device_id
                && lease.is_exclusive
                && (lease.status == WorkloadLeaseStatus::Admitted
                    || lease.status == WorkloadLeaseStatus::Active)
        });

        if is_held_exclusively {
            // Cooperative CPU fallback: do NOT starve or crash training run
            Ok(OptionalConsumerPlacement {
                workload_kind,
                target_device_id: cpu_device,
                is_accelerated: false,
                batch_size: 16,
                allows_cloud_egress: false,
                reason: format!(
                    "Target accelerator '{}' held exclusively by active training run; routed to CPU fallback without disrupting user workflow",
                    target_gpu.device_id
                ),
            })
        } else {
            // Admitted on GPU with bounded batch size
            Ok(OptionalConsumerPlacement {
                workload_kind,
                target_device_id: target_gpu.device_id.clone(),
                is_accelerated: true,
                batch_size: 64,
                allows_cloud_egress: false,
                reason: format!(
                    "Target accelerator '{}' admitted for cooperative background indexing",
                    target_gpu.device_id
                ),
            })
        }
    }

    /// Determines if an incoming higher-priority workload should preempt the background lease
    pub fn can_preempt_background_for_interactive(
        &self,
        background_lease: &WorkloadLease,
        incoming_workload: ComputeWorkloadKind,
    ) -> bool {
        !incoming_workload.is_cooperative_background()
            && incoming_workload.priority_rank() > background_lease.workload_kind.priority_rank()
    }
}
