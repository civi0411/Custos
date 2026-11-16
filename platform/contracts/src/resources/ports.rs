//! Resources (Workspace, Files, Git, Cas, Checkpoint, Compute, Terminal, Notebook) SPI Trait Ports

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::identity::error::DomainError;
use crate::resources::workspace::WorkspaceId;
use crate::resources::artifact::ArtifactRef;
use crate::resources::compute::{
    CloudPodLease, HardwareAdaptationPlan, ModelEvaluationReport, ModelEvaluationSpec,
    OptionalConsumerPlacement, TrainingCheckpointManifest,
    TrainingExecutionReceipt, TrainingStepTelemetry,
};
use crate::resources::workspace::{
    DirtyManifest, ExecutionWorkspace, WorkspaceKind, WorkspaceStatus,
};
use crate::resources::workspace_files::{
    WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff, WorkspaceFileTree,
    WriteWorkspaceFileParams,
};
use crate::task::ports::{RunPort, TaskStore, WorkflowRevisionPort};
use crate::task::run::{
    RunStepSpan, StepError, SupervisionFinding, SupervisionLogEntry, SupervisionRunRecord,
};
use crate::task::oi::{DecisionRecord, ReplanRecord};
use crate::conversation::ports::SessionStore;

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
    async fn list_spans_for_run(&self, run_id: &str) -> Result<Vec<RunStepSpan>, DomainError>;
    async fn list_logs_for_run(
        &self,
        run_id: &str,
        severity: Option<crate::task::run::LogSeverity>,
    ) -> Result<Vec<SupervisionLogEntry>, DomainError>;
    async fn list_findings_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<SupervisionFinding>, DomainError>;
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
    async fn head_blob(&self, prefix: &str, name: &str) -> Result<Option<BlobMetadata>, DomainError>;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptObservation {
    pub attempt_id: String,
    pub telemetry: Vec<ComputeTelemetry>,
    pub latest_checkpoint: Option<TrainingCheckpointManifest>,
    pub error: Option<String>,
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
    async fn request_cloud_pod(&self, request: &CloudPodRequest)
        -> Result<CloudPodLease, DomainError>;
    async fn release_cloud_pod(&self, pod_id: &str) -> Result<(), DomainError>;
    async fn observe_attempt(&self, attempt_id: &str) -> Result<AttemptObservation, DomainError>;
    async fn capture_telemetry(&self, run_id: &str) -> Result<TrainingStepTelemetry, DomainError>;
    async fn cancel_workload(&self, attempt_id: &str) -> Result<(), DomainError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub admitted: bool,
    pub verdict: String,
    pub plan: Option<HardwareAdaptationPlan>,
    pub violations: Vec<String>,
}

#[async_trait]
pub trait WorkloadAdmissionPort: Send + Sync {
    async fn admit_recipe(
        &self,
        recipe: &crate::resources::compute::FrozenTrainingRecipe,
        placement: &OptionalConsumerPlacement,
    ) -> Result<PreflightReport, DomainError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetCapabilities {
    pub accelerator_type: String,
    pub vram_mb: u64,
    pub supports_half_precision: bool,
    pub supports_flash_attention: bool,
    pub can_offload_cpu: bool,
}

#[async_trait]
pub trait ExecutionTargetPort: Send + Sync {
    async fn capabilities(&self) -> TargetCapabilities;
    async fn evaluate_model(
        &self,
        spec: &ModelEvaluationSpec,
    ) -> Result<ModelEvaluationReport, DomainError>;
    async fn execute_step(
        &self,
        step: u64,
        batch_size: u32,
    ) -> Result<TrainingExecutionReceipt, DomainError>;
}

#[async_trait]
pub trait ExecutionTargetResolver: Send + Sync {
    async fn resolve(
        &self,
        placement: &OptionalConsumerPlacement,
    ) -> Result<std::sync::Arc<dyn ExecutionTargetPort>, DomainError>;
}

// ---------------------------------------------------------------------------
// 6. Terminal PTY Port & 7. Notebook Driver Port
// ---------------------------------------------------------------------------

pub use crate::execution::terminal::{
    TerminalDriverPort, TerminalOutputConsumer, TerminalPort, TerminalProcessHandle,
};
pub use crate::execution::notebook::{
    NotebookDriverPort, NotebookStoragePort, NotebookWorkerResponse, NotebookWorkerSession,
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

