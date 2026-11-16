use serde::{Deserialize, Serialize};

use crate::ExecutionLane;

/// Category of computational workload claiming resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeWorkloadKind {
    NotebookKernel,
    TrainingRun,
    EvaluationRun,
    LocalInference,
    BackgroundEmbedding,
    DocumentOcr,
}

impl ComputeWorkloadKind {
    pub fn is_cooperative_background(&self) -> bool {
        matches!(self, Self::BackgroundEmbedding | Self::DocumentOcr)
    }

    pub fn priority_rank(&self) -> u8 {
        match self {
            Self::NotebookKernel => 10,
            Self::LocalInference => 9,
            Self::TrainingRun => 8,
            Self::EvaluationRun => 7,
            Self::DocumentOcr => 3,
            Self::BackgroundEmbedding => 2,
        }
    }

    /// Default execution lane mapped to this workload kind (SADE R7)
    pub fn default_lane(&self) -> ExecutionLane {
        match self {
            Self::NotebookKernel => ExecutionLane::Interactive,
            Self::LocalInference => ExecutionLane::Interactive,
            Self::TrainingRun | Self::EvaluationRun => ExecutionLane::Batch,
            Self::BackgroundEmbedding | Self::DocumentOcr => ExecutionLane::Ambient,
        }
    }
}

/// Placement decision for an optional consumer workload (Embeddings / OCR)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalConsumerPlacement {
    pub workload_kind: ComputeWorkloadKind,
    pub target_device_id: String,
    pub is_accelerated: bool,
    pub batch_size: u32,
    pub allows_cloud_egress: bool,
    pub reason: String,
}

/// Lifecycle state of a workload resource lease
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadLeaseStatus {
    Admitted,
    Queued,
    Active,
    Released,
    Preempted,
}

/// Admitted or active lease for a compute workload on a target device
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadLease {
    pub lease_id: String,
    pub device_id: String,
    pub workload_kind: ComputeWorkloadKind,
    pub owner_id: String,
    pub is_exclusive: bool,
    pub status: WorkloadLeaseStatus,
    pub acquired_at: i64,
    pub released_at: Option<i64>,
    #[serde(default)]
    pub lane: ExecutionLane,
}

/// Inbound request to reserve compute capacity for a workload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadAdmissionRequest {
    pub workload_kind: ComputeWorkloadKind,
    pub owner_id: String,
    pub preferred_device_id: Option<String>,
    pub require_exclusive: bool,
    pub required_memory_mb: Option<u64>,
    #[serde(default)]
    pub lane: Option<ExecutionLane>,
}

impl WorkloadAdmissionRequest {
    pub fn new(workload_kind: ComputeWorkloadKind, owner_id: impl Into<String>) -> Self {
        Self {
            workload_kind,
            owner_id: owner_id.into(),
            preferred_device_id: None,
            require_exclusive: false,
            required_memory_mb: None,
            lane: None,
        }
    }

    pub fn with_preferred_device(mut self, device_id: impl Into<String>) -> Self {
        self.preferred_device_id = Some(device_id.into());
        self
    }

    pub fn with_exclusive(mut self, exclusive: bool) -> Self {
        self.require_exclusive = exclusive;
        self
    }

    pub fn with_memory_mb(mut self, mb: u64) -> Self {
        self.required_memory_mb = Some(mb);
        self
    }

    pub fn with_lane(mut self, lane: ExecutionLane) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn effective_lane(&self) -> ExecutionLane {
        self.lane.unwrap_or_else(|| self.workload_kind.default_lane())
    }
}

/// Structured outcome of an admission decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadAdmissionVerdict {
    Admitted {
        lease: WorkloadLease,
    },
    Queued {
        queue_position: usize,
        reason: String,
    },
    Rejected {
        reason: String,
    },
}

