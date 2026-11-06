//! Custos Local API Contracts & Client
//!
//! Canonical protocol types for IPC communication between CLI / UI / VS Code and custos-daemon.

use async_trait::async_trait;
use custos_domain::{
    ExecuteCellParams, ExecuteCellResult, ExecutionWorkspace, NotebookCell, NotebookKernelState,
    RecordReviewParams, ReviewTargetType, ReviewerRecord, Session, SessionJournalEntry, Task,
    TaskContract, TaskStatus, VerificationClaim, WorkspaceDiffSummary, WorkspaceFileContent,
    WorkspaceFileDiff, WorkspaceFileTree, WorkspaceKind, WorkspaceLineage,
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
pub const METHOD_SESSIONS_OPEN_IN_LENS: &str = "v1.sessions.open_in_lens";
pub const METHOD_SESSIONS_MESSAGE: &str = "v1.sessions.message";
pub const METHOD_SESSIONS_PROMOTE: &str = "v1.sessions.promote";
pub const METHOD_SESSIONS_DELETE: &str = "v1.sessions.delete";
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
pub const METHOD_WORKSPACES_DETECT: &str = "v1.workspaces.detect";
pub const METHOD_WORKSPACES_PICK_FOLDER: &str = "v1.workspaces.pick_folder";
pub const METHOD_WORKSPACES_CANDIDATES_SPAWN: &str = "v1.workspaces.candidates.spawn";
pub const METHOD_WORKSPACES_CANDIDATES_COMPARE: &str = "v1.workspaces.candidates.compare";
pub const METHOD_WORKSPACES_CANDIDATES_RACE: &str = "v1.workspaces.candidates.race";
pub const METHOD_WORKSPACES_INTEGRATE: &str = "v1.workspaces.integrate";
pub const METHOD_SETTINGS_GET: &str = "v1.settings.get";
pub const METHOD_SETTINGS_SET: &str = "v1.settings.set";

pub const METHOD_QUANT_BACKTEST_SUBMIT: &str = "v1.quant.backtest.submit";
pub const METHOD_QUANT_TEARSHEET_GET: &str = "v1.quant.tearsheet.get";
pub const METHOD_QUANT_ORDERS_CREATE: &str = "v1.quant.orders.create";
pub const METHOD_QUANT_ORDERS_LIST: &str = "v1.quant.orders.list";
pub const METHOD_QUANT_ORDERS_APPROVE: &str = "v1.quant.orders.approve";
pub const METHOD_QUANT_FACTORS_DISCOVER: &str = "v1.quant.factors.discover";
pub const METHOD_QUANT_MARKET_BARS_INGEST: &str = "v1.quant.market_bars.ingest";
pub const METHOD_QUANT_MARKET_BARS_GET: &str = "v1.quant.market_bars.get";
pub const METHOD_PIPELINE_STATUS: &str = "v1.pipeline.status";
pub const METHOD_PIPELINE_STRUCTURE_DETECT: &str = "v1.pipeline.structure.detect";
pub const METHOD_PIPELINE_RUN: &str = "v1.pipeline.run";
pub const METHOD_QUANT_KILL_SWITCH: &str = "v1.quant.kill_switch";
pub const METHOD_QUANT_GRID_EVALUATE: &str = "v1.quant.grid.evaluate";
pub const METHOD_QUANT_INDICATORS_COMPUTE: &str = "v1.quant.indicators.compute";
pub const METHOD_QUANT_GATE_EVALUATE: &str = "v1.quant.gate.evaluate";
pub const METHOD_QUANT_REVIEW_COMPUTE: &str = "v1.quant.review.compute";
pub const METHOD_QUANT_ORDERBOOK_GET: &str = "v1.quant.orderbook.get";
pub const METHOD_QUANT_PBO_COMPUTE: &str = "v1.quant.pbo.compute";

pub const METHOD_WORKSPACE_FILES_TREE: &str = "v1.workspace.files.tree";
pub const METHOD_WORKSPACE_FILES_READ: &str = "v1.workspace.files.read";
pub const METHOD_WORKSPACE_FILES_WRITE: &str = "v1.workspace.files.write";
pub const METHOD_WORKSPACE_DIFF: &str = "v1.workspace.diff";
pub const METHOD_WORKSPACE_FILE_DIFF: &str = "v1.workspace.diff.file";
pub const METHOD_WORKSPACE_GIT_STAGE: &str = "v1.workspace.git.stage";
pub const METHOD_WORKSPACE_GIT_UNSTAGE: &str = "v1.workspace.git.unstage";
pub const METHOD_WORKSPACE_GIT_DISCARD: &str = "v1.workspace.git.discard";
pub const METHOD_WORKSPACE_GIT_COMMIT: &str = "v1.workspace.git.commit";

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
pub const METHOD_BROWSER_TABS_CLICK: &str = "v1.browser.tabs.click";
pub const METHOD_BROWSER_TABS_FILL: &str = "v1.browser.tabs.fill";
pub const METHOD_BROWSER_TABS_SCREENSHOT: &str = "v1.browser.tabs.screenshot";
pub const METHOD_BROWSER_TABS_CLOSE: &str = "v1.browser.tabs.close";

pub const METHOD_EMULATOR_DEVICES_LIST: &str = "v1.emulator.devices.list";
pub const METHOD_EMULATOR_LAUNCH: &str = "v1.emulator.devices.launch";
pub const METHOD_EMULATOR_NAVIGATE: &str = "v1.emulator.devices.navigate";
pub const METHOD_EMULATOR_SNAPSHOT: &str = "v1.emulator.devices.snapshot";
pub const METHOD_EMULATOR_SCREENSHOT: &str = "v1.emulator.devices.screenshot";
pub const METHOD_EMULATOR_TAP: &str = "v1.emulator.devices.tap";

pub const METHOD_FLEET_HOSTS_LIST: &str = "v1.fleet.hosts.list";
pub const METHOD_FLEET_HOSTS_REGISTER: &str = "v1.fleet.hosts.register";
pub const METHOD_FLEET_HOSTS_PING: &str = "v1.fleet.hosts.ping";
pub const METHOD_FLEET_EXEC: &str = "v1.fleet.exec";

pub const METHOD_AUTOMATION_JOBS_LIST: &str = "v1.automation.jobs.list";
pub const METHOD_AUTOMATION_JOBS_CREATE: &str = "v1.automation.jobs.create";
pub const METHOD_AUTOMATION_JOBS_RUN: &str = "v1.automation.jobs.run";

pub const METHOD_COMPUTE_TARGETS_LIST: &str = "v1.compute.targets.list";
pub const METHOD_EXPERIMENT_PREFLIGHT: &str = "v1.experiment.preflight";
pub const METHOD_EXPERIMENT_SUBMIT: &str = "v1.experiment.submit";
pub const METHOD_EXPERIMENT_OBSERVE: &str = "v1.experiment.observe";
pub const METHOD_EXPERIMENT_STOP: &str = "v1.experiment.stop";
pub const METHOD_EXPERIMENT_RELEASE: &str = "v1.experiment.release";
pub const METHOD_EXPERIMENT_COLLECT: &str = "v1.experiment.collect";

pub const METHOD_PROVIDERS_LIST: &str = "v1.providers.list";
pub const METHOD_PROVIDERS_GET: &str = "v1.providers.get";
pub const METHOD_PROVIDERS_SAVE: &str = "v1.providers.save";
pub const METHOD_PROVIDERS_DELETE: &str = "v1.providers.delete";
pub const METHOD_MODELS_PROBE: &str = "v1.models.probe";
pub const METHOD_MODELS_CATALOG: &str = "v1.models.catalog";
pub const METHOD_PROVIDERS_USAGE: &str = "v1.providers.usage";
pub const METHOD_MODELS_PRICING: &str = "v1.models.pricing";

pub const METHOD_OAUTH_AUTHORIZE: &str = "v1.oauth.authorize";
pub const METHOD_OAUTH_EXCHANGE: &str = "v1.oauth.exchange";
pub const METHOD_OAUTH_REFRESH: &str = "v1.oauth.refresh";
pub const METHOD_OAUTH_GET: &str = "v1.oauth.get";
pub const METHOD_OAUTH_DELETE: &str = "v1.oauth.delete";
pub const METHOD_OAUTH_CANCEL: &str = "v1.oauth.cancel";
pub const METHOD_OAUTH_STATUS: &str = "v1.oauth.status";

pub const METHOD_GITHUB_ACCOUNTS: &str = "v1.github.accounts";
pub const METHOD_GIT_REPOSITORY: &str = "v1.workspace.git.repository";
pub const METHOD_GIT_SWITCH: &str = "v1.workspace.git.switch";
pub const METHOD_GIT_FETCH: &str = "v1.workspace.git.fetch";
pub const METHOD_GIT_ACCOUNT: &str = "v1.workspace.git.account";
pub const METHOD_GIT_GITHUB: &str = "v1.workspace.git.github";
pub const METHOD_GITHUB_LOGIN: &str = "v1.github.login";
pub const METHOD_GITHUB_AUTH_STATUS: &str = "v1.github.auth_status";
pub const METHOD_GITHUB_VIEWER: &str = "v1.github.viewer";

pub const METHOD_PACKS_MANIFESTS_LIST: &str = "v1.packs.manifests.list";
pub const METHOD_PACKS_JOBS_LIST: &str = "v1.packs.jobs.list";
pub const METHOD_PACKS_JOBS_GET: &str = "v1.packs.jobs.get";
pub const METHOD_PACKS_ROLES_LIST: &str = "v1.packs.roles.list";
pub const METHOD_PACKS_JOBS_RUN: &str = "v1.packs.jobs.run";

pub const METHOD_ASSISTANT_DRAFTS_LIST: &str = "v1.assistant.drafts.list";
pub const METHOD_ASSISTANT_DRAFTS_GET: &str = "v1.assistant.drafts.get";
pub const METHOD_ASSISTANT_DRAFTS_SAVE: &str = "v1.assistant.drafts.save";
pub const METHOD_ASSISTANT_DRAFTS_DELETE: &str = "v1.assistant.drafts.delete";
pub const METHOD_ASSISTANT_OUTBOX_LIST: &str = "v1.assistant.outbox.list";
pub const METHOD_ASSISTANT_OUTBOX_CREATE: &str = "v1.assistant.outbox.create";
pub const METHOD_ASSISTANT_OUTBOX_APPROVE: &str = "v1.assistant.outbox.approve";
pub const METHOD_ASSISTANT_OUTBOX_DISPATCH: &str = "v1.assistant.outbox.dispatch";

pub const METHOD_WORKSPACES_AGENT_STEER: &str = "v1.workspaces.agent_steer";
pub const METHOD_RESOURCES_CAPABILITIES: &str = "v1.resources.capabilities";

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

        writer
            .write_all(&encoded)
            .await
            .map_err(|e| e.to_string())?;
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
pub mod client;
pub use client::*;

#[cfg(test)]
mod tests;
