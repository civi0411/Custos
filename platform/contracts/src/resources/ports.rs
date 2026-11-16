//! Resources (Workspace, Files, Git, Cas, Checkpoint, Compute, Terminal, Notebook) SPI Trait Ports

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::identity::error::DomainError;
use crate::resources::workspace::WorkspaceId;
use crate::resources::artifact::ArtifactRef;
use crate::resources::compute::{
    CloudPodLease, HardwareAdaptationPlan, ModelEvaluationReport, ModelEvaluationSpec,
    OptionalConsumerPlacement, TrainingCheckpointManifest,
    TrainingExecutionReceipt, TrainingStepTelemetry,
};
use crate::resources::terminal::{TerminalOutputChunk, TerminalSession};
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
    async fn create_workspace(
        &self,
        parent_repo: &str,
        branch: &str,
        kind: WorkspaceKind,
    ) -> Result<ExecutionWorkspace, DomainError>;

    async fn inspect_workspace(&self, workspace: &ExecutionWorkspace)
        -> Result<bool, DomainError>;

    async fn check_dirty(
        &self,
        workspace: &ExecutionWorkspace,
    ) -> Result<DirtyManifest, DomainError>;

    async fn cleanup_workspace(
        &self,
        workspace: &ExecutionWorkspace,
        force: bool,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait RepositoryGitPort: Send + Sync {
    async fn clone_repository(
        &self,
        remote_url: &str,
        target_dir: &Path,
        branch: Option<&str>,
    ) -> Result<(), DomainError>;

    async fn create_branch(&self, repo_dir: &Path, branch_name: &str) -> Result<(), DomainError>;
    async fn current_branch(&self, repo_dir: &Path) -> Result<String, DomainError>;
    async fn head_commit(&self, repo_dir: &Path) -> Result<String, DomainError>;
    async fn is_dirty(&self, repo_dir: &Path) -> Result<bool, DomainError>;
    async fn stage_all(&self, repo_dir: &Path) -> Result<(), DomainError>;
    async fn commit(&self, repo_dir: &Path, message: &str) -> Result<String, DomainError>;
    async fn push(&self, repo_dir: &Path, remote: &str, branch: &str) -> Result<(), DomainError>;
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
// 6. Terminal PTY Port
// ---------------------------------------------------------------------------

#[async_trait]
pub trait TerminalPort: Send + Sync {
    async fn spawn(
        &self,
        workspace_id: &str,
        working_dir: &Path,
        command: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<TerminalSession, DomainError>;

    async fn write(&self, session_id: &str, data: &[u8]) -> Result<usize, DomainError>;
    async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), DomainError>;
    async fn read_output(
        &self,
        session_id: &str,
        from_seq: u64,
        max_bytes: usize,
    ) -> Result<TerminalOutputChunk, DomainError>;
    async fn terminate(&self, session_id: &str) -> Result<(), DomainError>;
}

pub trait TerminalOutputConsumer: Send + Sync {
    fn consume_output(&self, session_id: &str, chunk: TerminalOutputChunk);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalProcessHandle {
    pub session_id: String,
    pub pid: u32,
}

#[async_trait]
pub trait TerminalDriverPort: Send + Sync {
    async fn create_pty(
        &self,
        working_dir: &Path,
        command: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<(TerminalProcessHandle, Box<dyn TerminalPort>), DomainError>;
}

// ---------------------------------------------------------------------------
// 7. Notebook Driver Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookWorkerResponse {
    pub status: String,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait NotebookWorkerSession: Send + Sync {
    async fn execute(
        &self,
        code: String,
        cwd: Option<String>,
    ) -> Result<NotebookWorkerResponse, DomainError>;
    fn interrupt(&self);
}

#[async_trait]
pub trait NotebookDriverPort: Send + Sync {
    fn discover_python_path(&self) -> PathBuf {
        PathBuf::from("python3")
    }
    async fn start_kernel(
        &self,
        python_path: &Path,
        working_dir: &Path,
    ) -> Result<std::sync::Arc<dyn NotebookWorkerSession>, DomainError>;
}

#[async_trait]
pub trait NotebookStoragePort: Send + Sync {
    async fn get_document(
        &self,
        doc_id: &str,
    ) -> Result<Option<crate::execution::notebook::NotebookDocument>, DomainError>;
    async fn save_document(
        &self,
        doc: &crate::execution::notebook::NotebookDocument,
    ) -> Result<(), DomainError>;
    async fn list_documents(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<crate::execution::notebook::NotebookDocument>, DomainError>;
}

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

