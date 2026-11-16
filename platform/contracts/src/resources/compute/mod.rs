//! Sovereign Compute Tier & Checkpoint Relay Models (RFC 005)
//!
//! Provides strongly-typed domain definitions for heterogeneous compute backends,
//! accelerator specifications, safe checkpoint manifests, cloud pod leases,
//! and hardware adaptation planning for cross-platform failover.

mod adaptation;
mod admission;
mod backend;
mod checkpoint;
pub mod training;

#[cfg(test)]
mod tests;

pub use self::adaptation::{
    CompatibilityGateEvaluation, CompatibilityVerdict, HardwareAdaptationPlan,
};
pub use self::admission::{
    ComputeWorkloadKind, OptionalConsumerPlacement, WorkloadAdmissionRequest,
    WorkloadAdmissionVerdict, WorkloadLease, WorkloadLeaseStatus,
};
pub use self::backend::{
    AcceleratorSpec, CloudPodLease, CloudPodStatus, ComputeBackendKind, ComputeDeviceRef, DeviceKind,
};
pub use self::checkpoint::{
    CheckpointCommitStatus, CheckpointFileEntry, ResumabilityLevel, TrainingCheckpointManifest,
};
pub use self::training::{
    FrozenTrainingRecipe, ModelEvaluationReport, ModelEvaluationSpec, TrainingExecutionReceipt,
    TrainingStepTelemetry,
};

