use custos_domain::{
    TaskContract, TaskStatus, VerificationClaim, WorkspaceKind, WorkspaceLineage,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOperationalStatus {
    Available,
    Degraded,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacksJobCatalogItem {
    pub job_id: String,
    pub lens: custos_domain::WorkbenchLens,
    pub name: String,
    pub description: String,
    pub primary_role: String,
    pub recommended_topology: custos_domain::oi::ExecutionTopology,
    pub status: JobOperationalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
    pub required_inputs: Vec<String>,
    pub expected_artifacts: Vec<String>,
    pub required_verifiers: Vec<String>,
    pub acceptance_rubric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPacksJobsParams {
    #[serde(default)]
    pub lens: Option<String>,
    #[serde(default)]
    pub pack_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPacksJobParams {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPacksRolesParams {
    #[serde(default)]
    pub pack_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPacksJobRequest {
    pub job_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub inputs: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthorizeParams {
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub service_type: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub auth_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthExchangeParams {
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub service_type: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    pub code_or_url: String,
    pub code_verifier: String,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRefreshParams {
    pub provider_id: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
}

// Request and Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub contract: Option<TaskContract>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskRequest {
    pub task_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceTaskRequest {
    pub task_id: String,
    pub target_status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub summary: Option<String>,
    #[serde(default)]
    pub evidence_claims: Vec<VerificationClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGetRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDeleteRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAttachRequest {
    pub session_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSteerRequest {
    pub session_id: String,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPlanRequest {
    pub title: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub contract: Option<TaskContract>,
    #[serde(default)]
    pub estimated_files_count: Option<u32>,
    #[serde(default)]
    pub estimated_complexity: Option<u32>,
    #[serde(default)]
    pub budget_limit_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunRequest {
    #[serde(default)]
    pub background: bool,
    pub task_id: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    #[serde(default)]
    pub preferred_mode: Option<String>,
    #[serde(default)]
    pub harness_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub provider_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRunRequest {
    pub run_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceApiRequest {
    pub name: String,
    pub kind: WorkspaceKind,
    pub path: String,
    #[serde(default)]
    pub lineage: Option<WorkspaceLineage>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub setup_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWorkspaceApiRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveWorkspaceApiRequest {
    pub workspace_id: String,
    #[serde(default)]
    pub delete_physical: bool,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectWorkspaceDirtyApiRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoverWorkspaceApiRequest {
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetCapabilityApiRequest {
    pub capability_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHarnessRequest {
    pub harness_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunNativeHarnessRequest {
    pub harness_id: String,
    pub instruction: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelHarnessRunRequest {
    pub harness_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerHarnessRunRequest {
    pub harness_id: String,
    pub run_id: String,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetailResponse {
    pub artifact_path: String,
    pub latest_version: u32,
    pub latest_content_hash: String,
    pub versions: Vec<crate::research::ArtifactLineageNode>,
    pub annotations: Vec<custos_domain::AnnotationRecord>,
    pub content: Option<String>,
    /// Current workspace bytes, not an archived version snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<custos_domain::WorkspaceFileContent>,
    /// Present only when current bytes match the recorded version hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNoteParams {
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

