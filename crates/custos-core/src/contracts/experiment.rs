//! Execution Target Port & Contracts (F10 / P3).
//!
//! Separates the 7 discrete lifecycle operations of supervised experiment execution:
//! Preflight, Submit, Observe/Reconcile, Request Checkpoint, Stop Job, Release Compute, Collect.
//!
//! Stop Job and Release Compute are explicitly decoupled to prevent cloud resource leakage.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use custos_domain::{
    DomainError, ExecState, ExecutionArtifact, ExecutionAttempt, ExternalJobRef, ProgressTriplet,
    ResourceState,
};

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

/// Discrete operations contract for remote or local compute targets
#[async_trait]
pub trait ExecutionTargetPort: Send + Sync {
    /// Capabilities probe
    async fn capabilities(&self) -> TargetCapabilities;

    /// 1. Preflight: checks engine, credentials, dataset paths, quotas and budget
    async fn preflight(&self, recipe_id: &str) -> Result<PreflightReport, DomainError>;

    /// 2. Submit: requests execution creation; returns stable external reference
    async fn submit(
        &self,
        attempt: &ExecutionAttempt,
        idempotency_key: &str,
    ) -> Result<ExternalJobRef, DomainError>;

    /// 3. Observe / Reconcile: gets honest status of exact job
    async fn observe(&self, external_ref: &ExternalJobRef) -> Result<AttemptObservation, DomainError>;

    /// 4. Request Checkpoint: signals engine to save at next optimizer-step boundary
    async fn request_checkpoint(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;

    /// 5. Stop Job: terminates training process
    async fn stop_job(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;

    /// 6. Release Compute: deallocates pod/VM/runtime to prevent financial leakage
    async fn release_compute(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError>;

    /// 7. Collect: fetches output artifacts and telemetry summaries
    async fn collect_artifacts(
        &self,
        external_ref: &ExternalJobRef,
    ) -> Result<Vec<ExecutionArtifact>, DomainError>;
}
