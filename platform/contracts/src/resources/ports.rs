//! Resources (Workspace, Files, Git, Cas, Checkpoint, Compute, Terminal, Notebook) SPI Trait Ports

use crate::conversation::ports::SessionStore;
use crate::identity::error::DomainError;
use crate::resources::artifact::ArtifactRef;
use crate::resources::compute::{
    CloudPodLease, OptionalConsumerPlacement, TrainingCheckpointManifest, TrainingStepTelemetry,
};
use crate::resources::workspace::WorkspaceId;
use crate::resources::workspace::{
    DirtyManifest, ExecutionWorkspace, WorkspaceKind, WorkspaceStatus,
};
use crate::resources::workspace_files::{
    WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff, WorkspaceFileTree,
    WriteWorkspaceFileParams,
};
use crate::task::oi::{DecisionRecord, ReplanRecord};
use crate::task::ports::{RunPort, TaskStore, WorkflowRevisionPort};
use crate::task::run::{
    RunStepSpan, StepError, SupervisionFinding, SupervisionLogEntry, SupervisionRunRecord,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. Workspace Repository & Provider Ports
// ---------------------------------------------------------------------------

#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn save_workspace(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;
    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError>;
    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError>;
    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError>;
    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait WorkspaceProvider: Send + Sync {
    async fn validate(&self, kind: &WorkspaceKind, path: &str) -> Result<(), DomainError>;
    async fn provision(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;
    async fn setup(
        &self,
        workspace: &ExecutionWorkspace,
        script: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn inspect_dirty(
        &self,
        _workspace: &ExecutionWorkspace,
    ) -> Result<DirtyManifest, DomainError> {
        Ok(DirtyManifest::default())
    }
    async fn diff(&self, _workspace: &ExecutionWorkspace) -> Result<String, DomainError> {
        Ok(String::new())
    }
    async fn recover(&self, _workspace: &ExecutionWorkspace) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn inspect_environment_assurance(
        &self,
        _workspace: &ExecutionWorkspace,
    ) -> Result<crate::execution::assurance::EnvironmentAssurance, DomainError> {
        Ok(crate::execution::assurance::EnvironmentAssurance::unconstrained())
    }
    async fn teardown(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError>;
}

pub trait RepositoryGitPort: Send + Sync {
    fn add_worktree(
        &self,
        repo_dir: &std::path::Path,
        worktree_path: &std::path::Path,
    ) -> Result<(), DomainError>;
    fn remove_worktree(
        &self,
        repo_dir: &std::path::Path,
        worktree_path: &std::path::Path,
    ) -> Result<(), DomainError>;
    fn diff_head(&self, _worktree_path: &std::path::Path) -> Result<String, DomainError> {
        Ok(String::new())
    }
}

// ---------------------------------------------------------------------------
// 2. Workspace Files Port
// ---------------------------------------------------------------------------

#[async_trait]
pub trait WorkspaceFilesPort: Send + Sync {
    async fn get_file_tree(
        &self,
        workspace: &ExecutionWorkspace,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, DomainError>;

    async fn read_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, DomainError>;

    async fn write_file(
        &self,
        workspace: &ExecutionWorkspace,
        params: WriteWorkspaceFileParams,
    ) -> Result<WorkspaceFileContent, DomainError>;

    async fn get_diff(
        &self,
        workspace: &ExecutionWorkspace,
        staged: Option<bool>,
    ) -> Result<WorkspaceDiffSummary, DomainError>;

    async fn get_file_diff(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceFileDiff, DomainError>;

    async fn stage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;

    async fn unstage_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;

    async fn discard_file(
        &self,
        workspace: &ExecutionWorkspace,
        relative_path: &str,
    ) -> Result<(), DomainError>;

    async fn commit(
        &self,
        workspace: &ExecutionWorkspace,
        message: &str,
    ) -> Result<String, DomainError>;
}

// ---------------------------------------------------------------------------
// 3. Content-Addressable Storage (CAS) & Persistence Composite Ports
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CasPort: Send + Sync {
    async fn put(&self, data: &[u8], mime: Option<String>) -> Result<ArtifactRef, DomainError>;
    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, DomainError>;
    async fn exists(&self, hash: &str) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait DecisionPort: Send + Sync {
    async fn record_decision(&self, record: &DecisionRecord) -> Result<(), DomainError>;
    async fn get_decision(&self, id: &str) -> Result<Option<DecisionRecord>, DomainError>;
    async fn list_decisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<DecisionRecord>, DomainError>;
}

#[async_trait]
pub trait ReplanPort: Send + Sync {
    async fn record_replan(&self, record: &ReplanRecord) -> Result<(), DomainError>;
    async fn get_replan(&self, id: &str) -> Result<Option<ReplanRecord>, DomainError>;
    async fn list_replans_for_task(&self, task_id: &str) -> Result<Vec<ReplanRecord>, DomainError>;
}

#[async_trait]
pub trait SupervisionStorePort: Send + Sync {
    async fn persist_batch(
        &self,
        runs: &[SupervisionRunRecord],
        spans: &[RunStepSpan],
        logs: &[SupervisionLogEntry],
        errors: &[StepError],
        findings: &[SupervisionFinding],
    ) -> Result<(), DomainError>;

    async fn get_run_record(
        &self,
        run_id: &str,
    ) -> Result<Option<SupervisionRunRecord>, DomainError>;

    async fn list_runs(
        &self,
        filter: &crate::task::run::SupervisionRunFilter,
    ) -> Result<Vec<SupervisionRunRecord>, DomainError>;

    async fn list_run_spans(&self, run_id: &str) -> Result<Vec<RunStepSpan>, DomainError>;

    async fn list_logs(
        &self,
        filter: &crate::task::run::SupervisionLogFilter,
    ) -> Result<Vec<SupervisionLogEntry>, DomainError>;

    async fn list_run_errors(&self, run_id: &str) -> Result<Vec<StepError>, DomainError>;

    async fn list_errors(
        &self,
        workbench_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StepError>, DomainError>;

    async fn list_findings(
        &self,
        filter: &crate::task::run::SupervisionFindingFilter,
    ) -> Result<Vec<SupervisionFinding>, DomainError>;

    async fn count_overview_stats(
        &self,
    ) -> Result<(usize, usize, usize, usize, usize), DomainError>;

    async fn prune_before(
        &self,
        log_cutoff_ms: i64,
        span_cutoff_ms: i64,
    ) -> Result<usize, DomainError>;
}

pub trait StoragePort:
    TaskStore
    + SessionStore
    + RunPort
    + DecisionPort
    + WorkflowRevisionPort
    + ReplanPort
    + SupervisionStorePort
{
}

impl<T> StoragePort for T where
    T: TaskStore
        + SessionStore
        + RunPort
        + DecisionPort
        + WorkflowRevisionPort
        + ReplanPort
        + SupervisionStorePort
{
}

// ---------------------------------------------------------------------------
// 4. Checkpoint Store Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreCapabilities {
    pub supports_conditional_put: bool,
    pub supports_prefix_fencing: bool,
    pub supports_multipart_checksum: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMetadata {
    pub size_bytes: u64,
    pub digest: String,
    pub created_at_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PromotionOutcome {
    Promoted { canonical_step: u64 },
    StaleGeneration { actual_generation: u64 },
    Quarantined { reason: String },
}

#[async_trait]
pub trait CheckpointStorePort: Send + Sync {
    async fn capabilities(&self) -> StoreCapabilities;
    async fn put_blob(
        &self,
        prefix: &str,
        name: &str,
        data: &[u8],
        expected_digest: &str,
    ) -> Result<(), DomainError>;
    async fn get_blob(&self, prefix: &str, name: &str) -> Result<Option<Vec<u8>>, DomainError>;
    async fn head_blob(
        &self,
        prefix: &str,
        name: &str,
    ) -> Result<Option<BlobMetadata>, DomainError>;
    async fn publish_manifest(
        &self,
        prefix: &str,
        manifest: &TrainingCheckpointManifest,
    ) -> Result<(), DomainError>;
    async fn promote(
        &self,
        experiment_id: &str,
        checkpoint_id: &str,
        expected_generation: u64,
    ) -> Result<PromotionOutcome, DomainError>;
    async fn list_checkpoints(
        &self,
        experiment_id: &str,
    ) -> Result<Vec<TrainingCheckpointManifest>, DomainError>;
    async fn delete_attempt_prefix(&self, prefix: &str) -> Result<(), DomainError>;
}

// ---------------------------------------------------------------------------
// 5. Compute & Workload Admission Ports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeTelemetry {
    pub step: u64,
    pub loss: Option<f32>,
    pub throughput_tokens_per_sec: Option<f64>,
    pub memory_allocated_bytes: u64,
    pub timestamp_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptObservation {
    pub exec_state: crate::evidence::experiment_attempt::ExecState,
    pub resource_state: crate::evidence::experiment_attempt::ResourceState,
    pub progress: crate::evidence::experiment_attempt::ProgressTriplet,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPodRequest {
    pub instance_type: String,
    pub region: String,
    pub duration_hours: u32,
    pub disk_size_gb: u32,
}

#[async_trait]
pub trait ComputePort: Send + Sync {
    async fn request_cloud_pod(
        &self,
        request: &CloudPodRequest,
    ) -> Result<CloudPodLease, DomainError>;
    async fn release_cloud_pod(&self, pod_id: &str) -> Result<(), DomainError>;
    async fn observe_attempt(&self, attempt_id: &str) -> Result<AttemptObservation, DomainError>;
    async fn capture_telemetry(&self, run_id: &str) -> Result<TrainingStepTelemetry, DomainError>;
    async fn cancel_workload(&self, attempt_id: &str) -> Result<(), DomainError>;
}

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

    pub fn admitted(verdict: &str) -> Self {
        Self {
            allowed: true,
            blockers: Vec::new(),
            warnings: vec![verdict.to_string()],
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

#[async_trait]
pub trait WorkloadAdmissionPort: Send + Sync {
    /// Discovers all available compute devices on the local host or configured targets
    async fn discover_devices(
        &self,
    ) -> Result<Vec<crate::resources::compute::ComputeDeviceRef>, DomainError>;

    /// Requests admission for a compute workload (notebook, training, eval, inference)
    async fn admit_workload(
        &self,
        request: &crate::resources::compute::WorkloadAdmissionRequest,
    ) -> Result<crate::resources::compute::WorkloadAdmissionVerdict, DomainError>;

    /// Releases an active workload lease
    async fn release_lease(&self, lease_id: &str) -> Result<(), DomainError>;

    /// Lists all active workload leases, optionally filtered by device
    async fn list_active_leases(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<crate::resources::compute::WorkloadLease>, DomainError>;

    async fn admit_recipe(
        &self,
        _recipe: &crate::resources::compute::FrozenTrainingRecipe,
        _placement: &OptionalConsumerPlacement,
    ) -> Result<PreflightReport, DomainError> {
        Ok(PreflightReport::admitted("admission_ok"))
    }
}

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

#[async_trait]
pub trait ExecutionTargetPort: Send + Sync {
    async fn capabilities(&self) -> TargetCapabilities;
    async fn preflight(&self, recipe_id: &str) -> Result<PreflightReport, DomainError>;
    async fn submit(
        &self,
        attempt: &crate::evidence::experiment_attempt::ExecutionAttempt,
        idempotency_key: &str,
    ) -> Result<crate::evidence::experiment_attempt::ExternalJobRef, DomainError>;
    async fn observe(
        &self,
        external_ref: &crate::evidence::experiment_attempt::ExternalJobRef,
    ) -> Result<AttemptObservation, DomainError>;
    async fn request_checkpoint(
        &self,
        external_ref: &crate::evidence::experiment_attempt::ExternalJobRef,
    ) -> Result<(), DomainError>;
    async fn stop_job(
        &self,
        external_ref: &crate::evidence::experiment_attempt::ExternalJobRef,
    ) -> Result<(), DomainError>;
    async fn release_compute(
        &self,
        external_ref: &crate::evidence::experiment_attempt::ExternalJobRef,
    ) -> Result<(), DomainError>;
    async fn collect_artifacts(
        &self,
        external_ref: &crate::evidence::experiment_attempt::ExternalJobRef,
    ) -> Result<Vec<crate::evidence::execution_record::ExecutionArtifact>, DomainError>;
}

pub trait ExecutionTargetResolver: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn resolve(
        &self,
        backend: crate::resources::compute::ComputeBackendKind,
        work_dir: &std::path::Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
    ) -> std::sync::Arc<dyn ExecutionTargetPort>;
}

// ---------------------------------------------------------------------------
// 6. Terminal PTY Port & 7. Notebook Driver Port
// ---------------------------------------------------------------------------

pub use crate::execution::notebook::{
    NotebookDriverPort, NotebookStoragePort, NotebookWorkerResponse, NotebookWorkerSession,
};
pub use crate::execution::terminal::{
    TerminalDriverPort, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle,
};

// ---------------------------------------------------------------------------
// 8. Source Acquisition Port
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SourceAcquisitionPort: Send + Sync {
    async fn fetch_source_text(
        &self,
        path_or_uri: &str,
        base_path: Option<&str>,
    ) -> Result<String, DomainError>;
}
