//! Custos Local API Contracts & Client
//!
//! Canonical protocol types for IPC communication between CLI / UI / VS Code and custos-daemon.

use async_trait::async_trait;
use custos_domain::{
    ExecutionWorkspace, ExecuteCellParams, ExecuteCellResult, NotebookCell, NotebookKernelState,
    RecordReviewParams, ReviewerRecord, ReviewTargetType,
    Session, SessionJournalEntry, Task, TaskContract, TaskStatus,
    VerificationClaim, WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff,
    WorkspaceFileTree, WorkspaceKind, WorkspaceLineage,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod envelope;
pub use envelope::*;

pub const METHOD_TASKS_CREATE: &str = "v1.tasks.create";
pub const METHOD_TASKS_GET: &str = "v1.tasks.get";
pub const METHOD_TASKS_LIST: &str = "v1.tasks.list";
pub const METHOD_TASKS_SPANS: &str = "v1.tasks.spans";
pub const METHOD_TASKS_CANCEL: &str = "v1.tasks.cancel";
pub const METHOD_TASKS_ADVANCE: &str = "v1.tasks.advance";
pub const METHOD_TASKS_COMPLETE: &str = "v1.tasks.complete";
pub const METHOD_SESSIONS_CREATE: &str = "v1.sessions.create";
pub const METHOD_SESSIONS_GET: &str = "v1.sessions.get";
pub const METHOD_SESSIONS_LIST: &str = "v1.sessions.list";
pub const METHOD_SESSIONS_JOURNAL: &str = "v1.sessions.journal";
pub const METHOD_SESSIONS_MESSAGE: &str = "v1.sessions.message";
pub const METHOD_SESSIONS_PROMOTE: &str = "v1.sessions.promote";
pub const METHOD_BRIDGE_ATTACH: &str = "v1.bridge.attach";
pub const METHOD_BRIDGE_STEER: &str = "v1.bridge.steer";
pub const METHOD_OI_EXPLAIN: &str = "v1.oi.explain";
pub const METHOD_WORKFLOW_START_RUN: &str = "v1.workflow.start_run";
pub const METHOD_WORKFLOW_CANCEL_RUN: &str = "v1.workflow.cancel_run";
pub const METHOD_WORKSPACES_CREATE: &str = "v1.workspaces.create";
pub const METHOD_WORKSPACES_GET: &str = "v1.workspaces.get";
pub const METHOD_WORKSPACES_LIST: &str = "v1.workspaces.list";
pub const METHOD_WORKSPACES_ARCHIVE: &str = "v1.workspaces.archive";
pub const METHOD_WORKSPACES_INSPECT_DIRTY: &str = "v1.workspaces.inspect_dirty";
pub const METHOD_WORKSPACES_RECOVER: &str = "v1.workspaces.recover";

pub const METHOD_WORKSPACE_FILES_TREE: &str = "v1.workspace.files.tree";
pub const METHOD_WORKSPACE_FILES_READ: &str = "v1.workspace.files.read";
pub const METHOD_WORKSPACE_FILES_WRITE: &str = "v1.workspace.files.write";
pub const METHOD_WORKSPACE_DIFF: &str = "v1.workspace.diff";
pub const METHOD_WORKSPACE_FILE_DIFF: &str = "v1.workspace.diff.file";
pub const METHOD_WORKSPACE_GIT_STAGE: &str = "v1.workspace.git.stage";
pub const METHOD_WORKSPACE_GIT_UNSTAGE: &str = "v1.workspace.git.unstage";
pub const METHOD_WORKSPACE_GIT_DISCARD: &str = "v1.workspace.git.discard";

pub const METHOD_RESEARCH_SOURCES_LIST: &str = "v1.research.sources.list";
pub const METHOD_RESEARCH_SOURCES_SAVE: &str = "v1.research.sources.save";
pub const METHOD_RESEARCH_ANCHORS_SAVE: &str = "v1.research.anchors.save";
pub const METHOD_RESEARCH_ANCHORS_LIST: &str = "v1.research.anchors.list";
pub const METHOD_RESEARCH_CLAIMS_LIST: &str = "v1.research.claims.list";
pub const METHOD_RESEARCH_CLAIMS_SAVE: &str = "v1.research.claims.save";
pub const METHOD_RESEARCH_RUNS_LIST: &str = "v1.research.runs.list";
pub const METHOD_RESEARCH_RUNS_SAVE: &str = "v1.research.runs.save";
pub const METHOD_RESEARCH_LINEAGE_LIST: &str = "v1.research.lineage.list";
pub const METHOD_RESEARCH_HANDOFF_CODING: &str = "v1.research.handoff_coding";
pub const METHOD_CAPABILITIES_LIST: &str = "v1.capabilities.list";
pub const METHOD_CAPABILITIES_GET: &str = "v1.capabilities.get";
pub const METHOD_TERMINAL_SPAWN: &str = "v1.terminal.spawn";
pub const METHOD_TERMINAL_WRITE: &str = "v1.terminal.write";
pub const METHOD_TERMINAL_RESIZE: &str = "v1.terminal.resize";
pub const METHOD_TERMINAL_READ: &str = "v1.terminal.read";
pub const METHOD_TERMINAL_TERMINATE: &str = "v1.terminal.terminate";
pub const METHOD_TERMINAL_LIST: &str = "v1.terminal.list";
pub const METHOD_TERMINAL_GET: &str = "v1.terminal.get";

pub const METHOD_HARNESS_LIST: &str = "v1.harness.list";
pub const METHOD_HARNESS_GET: &str = "v1.harness.get";
pub const METHOD_HARNESS_RUN_NATIVE: &str = "v1.harness.run_native";
pub const METHOD_HARNESS_CANCEL: &str = "v1.harness.cancel";
pub const METHOD_HARNESS_STEER: &str = "v1.harness.steer";

pub const METHOD_ARTIFACTS_LIST: &str = "v1.artifacts.list";
pub const METHOD_ARTIFACTS_GET: &str = "v1.artifacts.get";
pub const METHOD_ARTIFACTS_RECORD_LINEAGE: &str = "v1.artifacts.record_lineage";
pub const METHOD_ARTIFACTS_LINEAGE_GRAPH: &str = "v1.artifacts.lineage_graph";
pub const METHOD_NOTES_LIST: &str = "v1.notes.list";
pub const METHOD_NOTES_GET: &str = "v1.notes.get";
pub const METHOD_NOTES_SAVE: &str = "v1.notes.save";
pub const METHOD_NOTES_HISTORY: &str = "v1.notes.history";

pub const METHOD_NOTEBOOK_CELLS_LIST: &str = "v1.notebook.cells.list";
pub const METHOD_NOTEBOOK_CELLS_SAVE: &str = "v1.notebook.cells.save";
pub const METHOD_NOTEBOOK_EXECUTE: &str = "v1.notebook.execute";
pub const METHOD_NOTEBOOK_INTERRUPT: &str = "v1.notebook.interrupt";
pub const METHOD_NOTEBOOK_RESET: &str = "v1.notebook.reset";
pub const METHOD_NOTEBOOK_STATUS: &str = "v1.notebook.status";

pub const METHOD_REVIEWS_LIST: &str = "v1.reviews.list";
pub const METHOD_REVIEWS_GET: &str = "v1.reviews.get";
pub const METHOD_REVIEWS_RECORD: &str = "v1.reviews.record";
pub const METHOD_REVIEWS_MARK_STALE: &str = "v1.reviews.mark_stale";

pub const METHOD_SYNTHESIS_PROPOSALS_LIST: &str = "v1.synthesis.proposals.list";
pub const METHOD_SYNTHESIS_PROPOSALS_GET: &str = "v1.synthesis.proposals.get";
pub const METHOD_SYNTHESIS_PROPOSALS_SAVE: &str = "v1.synthesis.proposals.save";
pub const METHOD_SYNTHESIS_HANDOFF_EXECUTE: &str = "v1.synthesis.handoff.execute";

pub const METHOD_BROWSER_SESSIONS_LIST: &str = "v1.browser.sessions.list";
pub const METHOD_BROWSER_SESSIONS_CREATE: &str = "v1.browser.sessions.create";
pub const METHOD_BROWSER_TABS_LIST: &str = "v1.browser.tabs.list";
pub const METHOD_BROWSER_TABS_CREATE: &str = "v1.browser.tabs.create";
pub const METHOD_BROWSER_TABS_NAVIGATE: &str = "v1.browser.tabs.navigate";
pub const METHOD_BROWSER_TABS_SNAPSHOT: &str = "v1.browser.tabs.snapshot";
pub const METHOD_BROWSER_TABS_CLOSE: &str = "v1.browser.tabs.close";

pub const METHOD_FLEET_HOSTS_LIST: &str = "v1.fleet.hosts.list";
pub const METHOD_FLEET_HOSTS_REGISTER: &str = "v1.fleet.hosts.register";
pub const METHOD_FLEET_HOSTS_PING: &str = "v1.fleet.hosts.ping";
pub const METHOD_FLEET_EXEC: &str = "v1.fleet.exec";

pub const METHOD_AUTOMATION_JOBS_LIST: &str = "v1.automation.jobs.list";
pub const METHOD_AUTOMATION_JOBS_CREATE: &str = "v1.automation.jobs.create";
pub const METHOD_AUTOMATION_JOBS_RUN: &str = "v1.automation.jobs.run";

pub const METHOD_PROVIDERS_LIST: &str = "v1.providers.list";
pub const METHOD_PROVIDERS_GET: &str = "v1.providers.get";
pub const METHOD_PROVIDERS_SAVE: &str = "v1.providers.save";
pub const METHOD_PROVIDERS_DELETE: &str = "v1.providers.delete";
pub const METHOD_MODELS_PROBE: &str = "v1.models.probe";
pub const METHOD_MODELS_CATALOG: &str = "v1.models.catalog";
pub const METHOD_MODELS_PRICING: &str = "v1.models.pricing";

pub const METHOD_OAUTH_AUTHORIZE: &str = "v1.oauth.authorize";
pub const METHOD_OAUTH_EXCHANGE: &str = "v1.oauth.exchange";
pub const METHOD_OAUTH_REFRESH: &str = "v1.oauth.refresh";
pub const METHOD_OAUTH_GET: &str = "v1.oauth.get";
pub const METHOD_OAUTH_DELETE: &str = "v1.oauth.delete";
pub const METHOD_OAUTH_STATUS: &str = "v1.oauth.status";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ApiRequest {
    pub fn new(
        id: impl Into<String>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

impl ApiResponse {
    pub fn ok(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self::ok(id, result)
    }

    pub fn error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(error.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }
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
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
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

/// Abstract transport for communicating with the Custos Daemon

#[async_trait]
pub trait ApiTransport: Send + Sync {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String>;
}

/// In-process closure transport for testing and local integration
pub struct FnTransport<F> {
    handler: F,
}

impl<F> FnTransport<F> {
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl<F, Fut> ApiTransport for FnTransport<F>
where
    F: Fn(ApiRequest) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ApiResponse> + Send,
{
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        Ok((self.handler)(req).await)
    }
}

/// Child process stdio-jsonl transport communicating with custos-daemon binary
pub struct ProcessTransport {
    stdin: tokio::sync::Mutex<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    lines: tokio::sync::Mutex<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>>,
    child: tokio::sync::Mutex<tokio::process::Child>,
}

impl ProcessTransport {
    pub fn spawn(daemon_binary: &str, db_path: Option<&str>) -> Result<Self, std::io::Error> {
        let mut cmd = tokio::process::Command::new(daemon_binary);
        if let Some(db) = db_path {
            cmd.env("CUSTOS_DB_PATH", db);
            if let Some(parent) = std::path::Path::new(db).parent() {
                cmd.env("CUSTOS_PROFILE_DIR", parent);
            }
        }
        cmd.env("CUSTOS_BIND", "127.0.0.1:0");
        cmd.env("CUSTOS_HTTP_BIND", "127.0.0.1:0");
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn()?;
        let stdin = tokio::io::BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("Failed to capture child stdin"))?,
        );
        let stdout = tokio::io::BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| std::io::Error::other("Failed to capture child stdout"))?,
        );
        let lines = tokio::io::AsyncBufReadExt::lines(stdout);

        Ok(Self {
            stdin: tokio::sync::Mutex::new(stdin),
            lines: tokio::sync::Mutex::new(lines),
            child: tokio::sync::Mutex::new(child),
        })
    }

    pub async fn kill(&self) -> Result<(), std::io::Error> {
        let mut child = self.child.lock().await;
        child.kill().await
    }
}

#[async_trait]
impl ApiTransport for ProcessTransport {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        use tokio::io::AsyncWriteExt;

        let mut encoded = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        encoded.push(b'\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(&encoded).await.map_err(|e| e.to_string())?;
            stdin.flush().await.map_err(|e| e.to_string())?;
        }

        let mut lines = self.lines.lock().await;
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str::<ApiResponse>(&line)
                .map_err(|e| format!("Invalid JSON response: {e}"));
        }
        Err("Daemon closed stdout stream without response".to_string())
    }
}

/// TCP socket transport communicating with custos-daemon local API server
pub struct TcpTransport {
    addr: String,
}

impl TcpTransport {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

#[async_trait]
impl ApiTransport for TcpTransport {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stream = tokio::net::TcpStream::connect(&self.addr)
            .await
            .map_err(|e| format!("Failed to connect to daemon at {}: {e}", self.addr))?;
        let (reader, mut writer) = stream.into_split();

        let mut encoded = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        encoded.push(b'\n');

        writer.write_all(&encoded).await.map_err(|e| e.to_string())?;
        writer.flush().await.map_err(|e| e.to_string())?;

        let mut lines = BufReader::new(reader).lines();
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str::<ApiResponse>(&line)
                .map_err(|e| format!("Invalid JSON response: {e}"));
        }
        Err("Daemon closed TCP connection without response".to_string())
    }
}

/// Strongly-typed client for the Custos Local API
#[derive(Clone)]
pub struct LocalApiClient {
    transport: Arc<dyn ApiTransport>,
}

impl LocalApiClient {
    pub fn new(transport: Box<dyn ApiTransport>) -> Self {
        Self {
            transport: Arc::from(transport),
        }
    }

    pub fn with_arc_transport(transport: Arc<dyn ApiTransport>) -> Self {
        Self { transport }
    }

    pub fn from_tcp(addr: impl Into<String>) -> Self {
        Self::with_arc_transport(Arc::new(TcpTransport::new(addr)))
    }

    pub fn from_profile(profile: &crate::profile::ProfileResolver) -> Result<Self, String> {
        let port = profile
            .read_port()
            .ok_or_else(|| "Daemon port file not found or daemon not running".to_string())?;
        Ok(Self::from_tcp(format!("127.0.0.1:{port}")))
    }

    pub async fn ping(&self) -> Result<(), String> {
        let req = ApiRequest::new("ping", "v1.ping", serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        self.transport.send_request(req).await
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let req: ApiRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = ApiResponse::error("", format!("Invalid JSON request: {e}"));
                return serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string());
            }
        };
        match self.transport.send_request(req).await {
            Ok(resp) => serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
            Err(err) => {
                let err_resp = ApiResponse::error("", err);
                serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }


    pub async fn create_task(
        &self,
        req_id: &str,
        title: &str,
        contract: Option<TaskContract>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CreateTaskRequest {
            title: title.to_string(),
            contract,
            metadata,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn get_task(&self, req_id: &str, task_id: &str) -> Result<Task, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn cancel_task(
        &self,
        req_id: &str,
        task_id: &str,
        reason: Option<String>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CancelTaskRequest {
            task_id: task_id.to_string(),
            reason,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CANCEL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn advance_task(
        &self,
        req_id: &str,
        task_id: &str,
        target_status: TaskStatus,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(AdvanceTaskRequest {
            task_id: task_id.to_string(),
            target_status,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_ADVANCE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn complete_task(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
    ) -> Result<Task, String> {
        self.complete_task_with_evidence(req_id, task_id, summary, Vec::new())
            .await
    }

    pub async fn complete_task_with_evidence(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
        evidence_claims: Vec<VerificationClaim>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CompleteTaskRequest {
            task_id: task_id.to_string(),
            summary,
            evidence_claims,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_COMPLETE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn list_tasks(&self, req_id: &str) -> Result<Vec<Task>, String> {
        let req = ApiRequest::new(req_id, METHOD_TASKS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse tasks list: {e}"))
    }

    pub async fn list_spans(
        &self,
        req_id: &str,
        task_id: &str,
    ) -> Result<Vec<custos_domain::Span>, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_SPANS, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse spans list: {e}"))
    }

    pub async fn create_session(
        &self,
        req_id: &str,
        mode: Option<&str>,
    ) -> Result<Session, String> {
        let params = serde_json::json!({
            "mode": mode.unwrap_or("bare")
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn get_session(&self, req_id: &str, session_id: &str) -> Result<Session, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn list_sessions(&self, req_id: &str) -> Result<Vec<Session>, String> {
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse sessions list: {e}"))
    }

    pub async fn append_session_message(
        &self,
        req_id: &str,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "role": role,
            "content": content,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_MESSAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn get_session_journal(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<SessionJournalEntry>, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_JOURNAL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        #[derive(Deserialize)]
        struct JournalWrapper {
            entries: Vec<SessionJournalEntry>,
        }
        if let Ok(wrapper) = serde_json::from_value::<JournalWrapper>(result.clone()) {
            return Ok(wrapper.entries);
        }
        serde_json::from_value(result).map_err(|e| format!("Failed to parse journal entries: {e}"))
    }

    pub async fn attach_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_ATTACH, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn steer_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
        message: &str,
    ) -> Result<serde_json::Value, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
            "message": message,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_STEER, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        resp.result.ok_or_else(|| "Empty result in response".into())
    }

    pub async fn promote_session(
        &self,
        req_id: &str,
        session_id: &str,
        contract: TaskContract,
    ) -> Result<Task, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "contract": contract,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_PROMOTE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn explain_plan(
        &self,
        req_id: &str,
        request: ExplainPlanRequest,
    ) -> Result<custos_runtime::oi::ExplainReport, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_OI_EXPLAIN, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExplainReport: {e}"))
    }

    pub async fn create_workspace(
        &self,
        req_id: &str,
        req: CreateWorkspaceApiRequest,
    ) -> Result<ExecutionWorkspace, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
    }

    pub async fn get_workspace(
        &self,
        req_id: &str,
        workspace_id: &str,
    ) -> Result<ExecutionWorkspace, String> {
        let params = serde_json::json!({ "workspace_id": workspace_id });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
    }

    pub async fn list_workspaces(
        &self,
        req_id: &str,
    ) -> Result<Vec<ExecutionWorkspace>, String> {
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Vec<ExecutionWorkspace>: {e}"))
    }

    pub async fn archive_workspace(
        &self,
        req_id: &str,
        workspace_id: &str,
        delete_physical: bool,
    ) -> Result<(), String> {
        let params = serde_json::to_value(ArchiveWorkspaceApiRequest {
            workspace_id: workspace_id.to_string(),
            delete_physical,
            force: false,
        })
        .map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_ARCHIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn get_workspace_file_tree(
        &self,
        req_id: &str,
        workspace_id: &str,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "relative_dir": relative_dir,
            "max_depth": max_depth,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_TREE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse WorkspaceFileTree: {e}"))
    }

    pub async fn read_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "max_bytes": max_bytes,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_READ, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse WorkspaceFileContent: {e}"))
    }

    pub async fn write_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        content: &str,
        create_parents: Option<bool>,
        overwrite: Option<bool>,
    ) -> Result<WorkspaceFileContent, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "content": content,
            "create_parents": create_parents.unwrap_or(true),
            "overwrite": overwrite.unwrap_or(true),
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_WRITE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse WorkspaceFileContent: {e}"))
    }

    pub async fn get_workspace_diff(
        &self,
        req_id: &str,
        workspace_id: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceDiffSummary, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "staged": staged,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_DIFF, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse WorkspaceDiffSummary: {e}"))
    }

    pub async fn get_workspace_file_diff(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceFileDiff, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "staged": staged,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILE_DIFF, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse WorkspaceFileDiff: {e}"))
    }

    pub async fn stage_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_STAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn unstage_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_UNSTAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn discard_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_DISCARD, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn list_harnesses(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_core::contracts::harness::HarnessDescriptor>, String> {
        let req = ApiRequest::new(req_id, METHOD_HARNESS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse harnesses list: {e}"))
    }

    pub async fn get_harness(
        &self,
        req_id: &str,
        harness_id: &str,
    ) -> Result<custos_core::contracts::harness::HarnessDescriptor, String> {
        let params = serde_json::json!({ "harness_id": harness_id });
        let req = ApiRequest::new(req_id, METHOD_HARNESS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse harness descriptor: {e}"))
    }

    pub async fn run_native_harness(
        &self,
        req_id: &str,
        request: RunNativeHarnessRequest,
    ) -> Result<custos_core::contracts::harness::HarnessExecutionResult, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_HARNESS_RUN_NATIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse run native result: {e}"))
    }

    pub async fn list_artifacts(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_domain::ArtifactSummary>, String> {
        let req = ApiRequest::new(req_id, METHOD_ARTIFACTS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact list: {e}"))
    }

    pub async fn get_artifact(
        &self,
        req_id: &str,
        path: &str,
    ) -> Result<ArtifactDetailResponse, String> {
        let req = ApiRequest::new(req_id, METHOD_ARTIFACTS_GET, serde_json::json!({ "artifact_path": path }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact detail: {e}"))
    }

    pub async fn get_artifact_lineage_graph(
        &self,
        req_id: &str,
        filter_path: Option<&str>,
    ) -> Result<custos_domain::ArtifactLineageGraph, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_ARTIFACTS_LINEAGE_GRAPH,
            serde_json::json!({ "artifact_path": filter_path }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse lineage graph: {e}"))
    }

    pub async fn list_notes(
        &self,
        req_id: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<custos_domain::NoteRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_LIST,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notes list: {e}"))
    }

    pub async fn get_note(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_domain::NoteRecord>, String> {
        let req = ApiRequest::new(req_id, METHOD_NOTES_GET, serde_json::json!({ "id": id }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note: {e}"))
    }

    pub async fn save_note(
        &self,
        req_id: &str,
        params: SaveNoteParams,
    ) -> Result<custos_domain::NoteRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTES_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved note: {e}"))
    }

    pub async fn list_note_versions(
        &self,
        req_id: &str,
        note_id: &str,
    ) -> Result<Vec<custos_domain::NoteVersionRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_HISTORY,
            serde_json::json!({ "note_id": note_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note versions: {e}"))
    }

    pub async fn list_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTEBOOK_CELLS_LIST,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notebook cells: {e}"))
    }

    pub async fn save_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
        cells: Vec<NotebookCell>,
    ) -> Result<(), String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTEBOOK_CELLS_SAVE,
            serde_json::json!({ "session_id": session_id, "cells": cells }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn execute_notebook_cell(
        &self,
        req_id: &str,
        params: ExecuteCellParams,
    ) -> Result<ExecuteCellResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse execution result: {e}"))
    }

    pub async fn interrupt_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<(), String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTEBOOK_INTERRUPT,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn reset_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTEBOOK_RESET,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn get_notebook_kernel_status(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTEBOOK_STATUS,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn list_reviews(
        &self,
        req_id: &str,
        target_type: Option<ReviewTargetType>,
        target_id: Option<&str>,
    ) -> Result<Vec<ReviewerRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_LIST,
            serde_json::json!({
                "target_type": target_type,
                "target_id": target_id,
            }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse reviews: {e}"))
    }

    pub async fn get_review(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<ReviewerRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_GET,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse review: {e}"))
    }

    pub async fn record_review(
        &self,
        req_id: &str,
        params: RecordReviewParams,
    ) -> Result<ReviewerRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_REVIEWS_RECORD, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse recorded review: {e}"))
    }

    pub async fn mark_review_stale(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<(), String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_MARK_STALE,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn list_synthesis_proposals(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_domain::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_PROPOSALS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposals: {e}"))
    }

    pub async fn get_synthesis_proposal(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_domain::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_SYNTHESIS_PROPOSALS_GET,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposal: {e}"))
    }

    pub async fn save_synthesis_proposal(
        &self,
        req_id: &str,
        params: custos_domain::SaveSynthesisProposalParams,
    ) -> Result<custos_domain::ResearchSynthesisProposal, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_PROPOSALS_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved proposal: {e}"))
    }

    pub async fn execute_synthesis_handoff(
        &self,
        req_id: &str,
        params: custos_domain::HandoffToCodingParams,
    ) -> Result<custos_domain::HandoffToCodingResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_HANDOFF_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse handoff result: {e}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetailResponse {
    pub artifact_path: String,
    pub latest_version: u32,
    pub latest_content_hash: String,
    pub versions: Vec<custos_domain::ArtifactLineageNode>,
    pub annotations: Vec<custos_domain::AnnotationRecord>,
    pub content: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport;

    #[async_trait]
    impl ApiTransport for MockTransport {
        async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
            match req.method.as_str() {
                METHOD_TASKS_CREATE => {
                    let task = Task::new(custos_domain::new_id("task"), "Mock Task".to_string());
                    Ok(ApiResponse::ok(
                        req.id,
                        serde_json::to_value(&task).unwrap(),
                    ))
                }
                _ => Ok(ApiResponse::error(req.id, "Unsupported method")),
            }
        }
    }

    #[tokio::test]
    async fn test_client_create_task() {
        let client = LocalApiClient::new(Box::new(MockTransport));
        let task = client
            .create_task("1", "Mock Task", None, None)
            .await
            .unwrap();
        assert_eq!(task.title, "Mock Task");
        assert_eq!(task.status, TaskStatus::Draft);
    }

    #[tokio::test]
    async fn test_tcp_transport_roundtrip() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();
                if let Ok(Some(line)) = lines.next_line().await {
                    let req: ApiRequest = serde_json::from_str(&line).unwrap();
                    assert_eq!(req.method, "v1.ping");
                    let resp = ApiResponse::success(req.id, serde_json::json!({ "status": "pong" }));
                    let mut data = serde_json::to_vec(&resp).unwrap();
                    data.push(b'\n');
                    let _ = writer.write_all(&data).await;
                    let _ = writer.flush().await;
                }
            }
        });

        let client = LocalApiClient::from_tcp(format!("127.0.0.1:{port}"));
        let req = ApiRequest::new("ping_1", "v1.ping", serde_json::json!({}));
        let resp = client.send_request(req).await.unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.result.unwrap(), serde_json::json!({ "status": "pong" }));
    }
}

