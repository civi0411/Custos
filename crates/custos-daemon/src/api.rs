//! Custos Local API Contracts & Dispatcher
//!
//! Protocol types and dispatcher for IPC communication between CLI / UI / VS Code and custosd.

use std::sync::Arc;
use base64::Engine;
use custos_domain::DomainError;

pub use crate::custos_local_api::{
    AdvanceTaskRequest, ApiRequest, ApiResponse, ArchiveWorkspaceApiRequest,
    ArtifactDetailResponse, CancelHarnessRunRequest, CancelRunRequest, CancelTaskRequest,
    CompleteTaskRequest, CreateTaskRequest, CreateWorkspaceApiRequest, GetCapabilityApiRequest,
    GetHarnessRequest, GetWorkspaceApiRequest, InspectWorkspaceDirtyApiRequest,
    OAuthAuthorizeParams, OAuthExchangeParams, OAuthRefreshParams, RecoverWorkspaceApiRequest,
    RunNativeHarnessRequest, SaveNoteParams, StartRunRequest, SteerHarnessRunRequest,
    METHOD_ARTIFACTS_GET, METHOD_ARTIFACTS_LINEAGE_GRAPH, METHOD_ARTIFACTS_LIST,
    METHOD_ARTIFACTS_RECORD_LINEAGE, METHOD_AUTOMATION_JOBS_CREATE, METHOD_AUTOMATION_JOBS_LIST,
    METHOD_AUTOMATION_JOBS_RUN, METHOD_BROWSER_SESSIONS_CREATE, METHOD_BROWSER_SESSIONS_LIST,
    METHOD_BROWSER_TABS_CLICK, METHOD_BROWSER_TABS_CLOSE, METHOD_BROWSER_TABS_CREATE,
    METHOD_BROWSER_TABS_FILL, METHOD_BROWSER_TABS_LIST, METHOD_BROWSER_TABS_NAVIGATE,
    METHOD_BROWSER_TABS_SCREENSHOT, METHOD_BROWSER_TABS_SNAPSHOT, METHOD_CAPABILITIES_GET,
    METHOD_CAPABILITIES_LIST, METHOD_EMULATOR_DEVICES_LIST, METHOD_EMULATOR_LAUNCH,
    METHOD_EMULATOR_NAVIGATE, METHOD_EMULATOR_SCREENSHOT, METHOD_EMULATOR_SNAPSHOT,
    METHOD_EMULATOR_TAP, METHOD_FLEET_EXEC, METHOD_FLEET_HOSTS_LIST, METHOD_FLEET_HOSTS_PING,
    METHOD_FLEET_HOSTS_REGISTER, METHOD_HARNESS_CANCEL, METHOD_HARNESS_GET, METHOD_HARNESS_LIST,
    METHOD_HARNESS_RUN_NATIVE, METHOD_HARNESS_STEER, METHOD_MODELS_CATALOG, METHOD_MODELS_PRICING,
    METHOD_MODELS_PROBE, METHOD_NOTEBOOK_CELLS_LIST, METHOD_NOTEBOOK_CELLS_SAVE,
    METHOD_NOTEBOOK_EXECUTE, METHOD_NOTEBOOK_INTERRUPT, METHOD_NOTEBOOK_RESET,
    METHOD_NOTEBOOK_STATUS, METHOD_NOTES_GET, METHOD_NOTES_HISTORY, METHOD_NOTES_LIST,
    METHOD_NOTES_SAVE, METHOD_OAUTH_AUTHORIZE, METHOD_OAUTH_CANCEL, METHOD_OAUTH_DELETE,
    METHOD_OAUTH_EXCHANGE, METHOD_OAUTH_GET, METHOD_OAUTH_REFRESH, METHOD_OAUTH_STATUS,
    METHOD_PROVIDERS_DELETE, METHOD_PROVIDERS_GET, METHOD_PROVIDERS_LIST, METHOD_PROVIDERS_SAVE,
    METHOD_RESEARCH_ANCHORS_LIST, METHOD_RESEARCH_ANCHORS_SAVE, METHOD_RESEARCH_CLAIMS_LIST,
    METHOD_RESEARCH_CLAIMS_SAVE, METHOD_RESEARCH_HANDOFF_CODING, METHOD_RESEARCH_LINEAGE_LIST,
    METHOD_RESEARCH_RUNS_LIST, METHOD_RESEARCH_RUNS_SAVE, METHOD_RESEARCH_SOURCES_LIST,
    METHOD_RESEARCH_SOURCES_SAVE, METHOD_REVIEWS_GET, METHOD_REVIEWS_LIST,
    METHOD_REVIEWS_MARK_STALE, METHOD_REVIEWS_RECORD, METHOD_SESSIONS_DELETE,
    METHOD_SYNTHESIS_HANDOFF_EXECUTE, METHOD_SYNTHESIS_PROPOSALS_GET,
    METHOD_SYNTHESIS_PROPOSALS_LIST, METHOD_SYNTHESIS_PROPOSALS_SAVE, METHOD_TERMINAL_GET,
    METHOD_TERMINAL_LIST, METHOD_TERMINAL_READ, METHOD_TERMINAL_RESIZE, METHOD_TERMINAL_SPAWN,
    METHOD_TERMINAL_TERMINATE, METHOD_TERMINAL_WRITE, METHOD_WORKFLOW_CANCEL_RUN,
    METHOD_WORKFLOW_START_RUN, METHOD_WORKSPACES_ARCHIVE, METHOD_WORKSPACES_CREATE,
    METHOD_WORKSPACES_GET, METHOD_WORKSPACES_INSPECT_DIRTY, METHOD_WORKSPACES_LIST,
    METHOD_WORKSPACES_RECOVER, METHOD_WORKSPACES_DETECT, METHOD_WORKSPACES_PICK_FOLDER,
    METHOD_WORKSPACES_CANDIDATES_SPAWN, METHOD_WORKSPACES_CANDIDATES_COMPARE,
    METHOD_WORKSPACES_INTEGRATE, METHOD_SETTINGS_GET, METHOD_SETTINGS_SET,
    METHOD_WORKSPACE_DIFF, METHOD_WORKSPACE_FILES_READ,
    METHOD_WORKSPACE_FILES_TREE, METHOD_WORKSPACE_FILES_WRITE, METHOD_WORKSPACE_FILE_DIFF,
    METHOD_WORKSPACE_GIT_COMMIT, METHOD_WORKSPACE_GIT_DISCARD, METHOD_WORKSPACE_GIT_STAGE, METHOD_WORKSPACE_GIT_UNSTAGE,
    JobOperationalStatus, PacksJobCatalogItem, METHOD_PACKS_JOBS_GET, METHOD_PACKS_JOBS_LIST,
    METHOD_PACKS_JOBS_RUN, METHOD_PACKS_MANIFESTS_LIST, METHOD_PACKS_ROLES_LIST,
    METHOD_ASSISTANT_DRAFTS_DELETE, METHOD_ASSISTANT_DRAFTS_GET, METHOD_ASSISTANT_DRAFTS_LIST,
    METHOD_ASSISTANT_DRAFTS_SAVE, METHOD_ASSISTANT_OUTBOX_CREATE, METHOD_ASSISTANT_OUTBOX_APPROVE, METHOD_ASSISTANT_OUTBOX_DISPATCH,
    METHOD_ASSISTANT_OUTBOX_LIST, METHOD_RESOURCES_CAPABILITIES, METHOD_WORKSPACES_AGENT_STEER,
    METHOD_GITHUB_AUTH_STATUS, METHOD_GITHUB_VIEWER, METHOD_GITHUB_LOGIN,
    METHOD_GITHUB_ACCOUNTS, METHOD_GIT_REPOSITORY, METHOD_GIT_SWITCH, METHOD_GIT_FETCH, METHOD_GIT_ACCOUNT, METHOD_GIT_GITHUB,
    METHOD_COMPUTE_TARGETS_LIST, METHOD_EXPERIMENT_PREFLIGHT, METHOD_EXPERIMENT_SUBMIT,
    METHOD_EXPERIMENT_OBSERVE, METHOD_EXPERIMENT_STOP, METHOD_EXPERIMENT_RELEASE,
    METHOD_EXPERIMENT_COLLECT,
};
use custos_adapters::compute::{ColabTargetAdapter, KaggleTargetAdapter, RentedGpuTargetAdapter};
use custos_adapters::harness::HarnessRegistry;
use custos_bridge::{AttachMode, BridgePort, BridgeService};
use custos_core::contracts::experiment::ExecutionTargetPort;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort};
use custos_core::contracts::terminal::TerminalPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_core::{AdvanceTask, CreateTask, TaskService};
use custos_domain::{
    new_id, validate_browser_url, AssistantDraft, Assurance, BrowserConsoleEntry,
    BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab, BrowserTabStatus,
    CreateHeadlessJobParams, ExecuteCellParams, ExecutionReceipt, FleetExecReceipt,
    HeadlessAutomationJob, HeadlessJobStatus, OutboxActionItem,
    ReceiptStatus, RegisterHostParams, RemoteHostNode, RemoteHostStatus,
    ResourceActionCapability, SessionId, SessionMode, SshAuthMethod, TaskContract,
    ComputeBackendKind, ExecEvent, ExecutionAttempt, ResourceEvent,
};
use custos_persistence::repositories::{AssistantRepository, OutboxRepository};
use custos_persistence::{FleetAutomationRepository, ProviderRepository, ResearchRepository};
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::{
    WorkspaceCoordinator, WorkspaceFilesCoordinator,
};
#[cfg(test)]
pub(crate) use custos_runtime::workspace::CreateWorkspaceRequest;
use custos_runtime::{PythonKernelCoordinator, TerminalCoordinator};

pub mod transport;
pub use transport::*;
pub mod capabilities;
pub mod commands;
pub mod tasks;
#[path = "application/mod.rs"]
mod application;

/// Local API Dispatcher wrapping TaskService, SessionManager, BridgeService, WorkflowPort, ResearchRepository, ProviderRepository, TerminalCoordinator, and WorkspaceFilesPort for IPC callers.
pub struct LocalApiDispatcher {
    task_service: Arc<TaskService>,
    session_manager: Arc<SessionManager>,
    bridge_service: Arc<BridgeService>,
    workflow: Option<Arc<dyn WorkflowPort>>,
    usage_runs: Option<Arc<dyn custos_core::contracts::storage::RunPort>>,
    workspace: Option<Arc<WorkspaceCoordinator>>,
    candidate_mutation: tokio::sync::Mutex<()>,
    research: Option<Arc<ResearchRepository>>,
    providers: Option<Arc<ProviderRepository>>,
    credentials: Option<Arc<crate::credentials::ProviderCredentials>>,
    terminal: Arc<TerminalCoordinator>,
    files: Arc<dyn WorkspaceFilesPort>,
    harnesses: Arc<HarnessRegistry>,
    python_kernel: Option<Arc<PythonKernelCoordinator>>,
    fleet_automation: Option<Arc<FleetAutomationRepository>>,
    model_availability: custos_adapters::providers::catalog::ModelAvailability,
    model_catalog: custos_adapters::providers::catalog::ModelCatalogService,
    oauth_callback_server: Option<Arc<crate::oauth_callback_server::OAuthCallbackServer>>,
    pack_registry: Arc<custos_packs::DomainPackRegistry>,
    assistant: Option<Arc<AssistantRepository>>,
    outbox: Option<Arc<OutboxRepository>>,
    sqlite_store: Option<Arc<custos_persistence::SqliteTaskStore>>,
    mcp_hub: Arc<custos_adapters::mcp::McpFederationHub>,
}

impl LocalApiDispatcher {
    pub fn new(
        task_service: Arc<TaskService>,
        session_manager: Arc<SessionManager>,
        bridge_service: Arc<BridgeService>,
    ) -> Self {
        Self {
            task_service,
            session_manager,
            bridge_service,
            workflow: None,
            usage_runs: None,
            workspace: None,
            candidate_mutation: tokio::sync::Mutex::new(()),
            research: None,
            providers: None,
            credentials: None,
            terminal: Arc::new(TerminalCoordinator::new(Arc::new(
                custos_adapters::terminal::NativePtyDriver,
            ))),
            files: Arc::new(WorkspaceFilesCoordinator::new()),
            harnesses: Arc::new(HarnessRegistry::new()),
            python_kernel: Some(Arc::new(PythonKernelCoordinator::new(Arc::new(
                custos_adapters::notebook::LocalPythonDriver,
            )))),
            fleet_automation: None,
            model_availability: Default::default(),
            model_catalog: custos_adapters::providers::catalog::ModelCatalogService::new(),
            oauth_callback_server: None,
            pack_registry: Arc::new(custos_packs::create_default_pack_registry()),
            assistant: None,
            outbox: None,
            sqlite_store: None,
            mcp_hub: Arc::new(custos_adapters::mcp::McpFederationHub::new()),
        }
    }

    pub fn with_mcp_hub(mut self, hub: Arc<custos_adapters::mcp::McpFederationHub>) -> Self {
        self.mcp_hub = hub;
        self
    }

    pub fn mcp_hub(&self) -> Arc<custos_adapters::mcp::McpFederationHub> {
        self.mcp_hub.clone()
    }

    pub fn with_model_availability(mut self, availability: custos_adapters::providers::catalog::ModelAvailability) -> Self {
        self.model_availability = availability;
        self
    }

    pub fn with_sqlite_store(mut self, store: Arc<custos_persistence::SqliteTaskStore>) -> Self {
        self.sqlite_store = Some(store);
        self
    }

    pub fn with_python_kernel(mut self, python_kernel: Arc<PythonKernelCoordinator>) -> Self {
        self.python_kernel = Some(python_kernel);
        self
    }

    pub fn with_harnesses(mut self, harnesses: Arc<HarnessRegistry>) -> Self {
        self.harnesses = harnesses;
        self
    }

    pub fn with_files(mut self, files: Arc<dyn WorkspaceFilesPort>) -> Self {
        self.files = files;
        self
    }

    pub fn with_terminal(mut self, terminal: Arc<TerminalCoordinator>) -> Self {
        self.terminal = terminal;
        self
    }

    pub fn with_usage_runs(mut self, runs: Arc<dyn custos_core::contracts::storage::RunPort>) -> Self {
        self.usage_runs = Some(runs);
        self
    }

    pub fn with_workflow(mut self, workflow: Arc<dyn WorkflowPort>) -> Self {
        self.workflow = Some(workflow);
        self
    }

    pub fn with_workspace(mut self, workspace: Arc<WorkspaceCoordinator>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn with_research(mut self, research: Arc<ResearchRepository>) -> Self {
        self.research = Some(research);
        self
    }

    pub fn with_assistant(mut self, assistant: Arc<AssistantRepository>) -> Self {
        self.assistant = Some(assistant);
        self
    }

    pub fn with_outbox(mut self, outbox: Arc<OutboxRepository>) -> Self {
        self.outbox = Some(outbox);
        self
    }

    fn outbox_repo(&self) -> Option<&OutboxRepository> {
        self.outbox.as_deref().or_else(|| self.sqlite_store.as_ref().map(|s| s.outbox()))
    }

    pub fn with_providers(mut self, providers: Arc<ProviderRepository>) -> Self {
        #[cfg(test)] { self.credentials=Some(crate::credential_tests::fake_credentials(providers.clone())); }
        self.providers = Some(providers);
        self
    }

    pub fn with_credentials(mut self, credentials: Arc<crate::credentials::ProviderCredentials>) -> Self {
        self.credentials = Some(credentials);
        self
    }
    fn credential_service(&self) -> Result<&Arc<crate::credentials::ProviderCredentials>, DomainError> {
        self.credentials.as_ref().ok_or_else(|| DomainError::Validation(
            "secure_storage_unavailable: Credential service is not configured".into()))
    }
    async fn read_provider_token(&self, account: &str) -> Result<Option<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.get_oauth_token(account).await
    }
    async fn refresh_provider_token(&self, previous: &custos_domain::OAuthTokenRecord, refreshed: &custos_domain::OAuthTokenRecord) -> Result<(), DomainError> {
        self.credential_service()?.save_refreshed(previous, refreshed).await
    }
    async fn write_provider_connection(&self, token: &custos_domain::OAuthTokenRecord, config: &custos_domain::ProviderConfig) -> Result<(), DomainError> {
        self.credential_service()?.save_oauth_connection(token, Some(config)).await
    }
    async fn remove_provider_credential(&self, account: &str, remove_provider: bool) -> Result<(), DomainError> {
        self.credential_service()?.delete(account, remove_provider).await
    }
    async fn provider_tokens(&self) -> Result<Vec<custos_domain::OAuthTokenRecord>, DomainError> {
        self.credential_service()?.list_oauth_tokens().await
    }

    pub fn with_fleet_automation(
        mut self,
        fleet_automation: Arc<FleetAutomationRepository>,
    ) -> Self {
        self.fleet_automation = Some(fleet_automation);
        self
    }

    pub fn with_oauth_callback_server(
        mut self,
        oauth_callback_server: Arc<crate::oauth_callback_server::OAuthCallbackServer>,
    ) -> Self {
        self.oauth_callback_server = Some(oauth_callback_server);
        self
    }

    pub fn with_packs(mut self, pack_registry: Arc<custos_packs::DomainPackRegistry>) -> Self {
        self.pack_registry = pack_registry;
        self
    }

    async fn resolve_workspace_for_dispatch(
        &self,
        req_id: &str,
        ws_id_str: &str,
    ) -> Result<custos_domain::ExecutionWorkspace, ApiResponse> {
        let coordinator = match self.workspace.as_ref() {
            Some(c) => c,
            None => {
                return Err(ApiResponse::error(
                    req_id,
                    "WorkspaceCoordinator not configured on daemon",
                ))
            }
        };
        let ws_id = custos_domain::WorkspaceId::new(ws_id_str);
        match coordinator.get_workspace(&ws_id).await {
            Ok(Some(ws)) => Ok(ws),
            Ok(None) => Err(ApiResponse::error(
                req_id,
                format!("Workspace {ws_id_str} not found"),
            )),
            Err(e) => Err(ApiResponse::error(
                req_id,
                format!("Workspace {ws_id_str} lookup failed: {e}"),
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_target_adapter(
        &self,
        backend: ComputeBackendKind,
        work_dir: &std::path::Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
        force_mock: bool,
    ) -> Arc<dyn ExecutionTargetPort> {
        match backend {
            ComputeBackendKind::Colab => {
                let mut a = ColabTargetAdapter::new(work_dir);
                if force_mock {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
            ComputeBackendKind::Kaggle => {
                let mut a = KaggleTargetAdapter::new(work_dir);
                if force_mock {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
            ComputeBackendKind::RunPod | ComputeBackendKind::VastAi | ComputeBackendKind::DedicatedSsh => {
                let host = ssh_host.unwrap_or("127.0.0.1");
                let port = ssh_port.unwrap_or(22);
                let user = ssh_user.unwrap_or("root");
                let mut a = RentedGpuTargetAdapter::new(host, port, user, backend, work_dir);
                if let Some(p) = pod_id {
                    a = a.with_pod_lease(p, cloud_api_key.map(|s| s.to_string()));
                }
                if force_mock || host == "mock" {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
            _ => {
                let mut a = ColabTargetAdapter::new(work_dir);
                if force_mock {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
        }
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let response = match serde_json::from_str::<ApiRequest>(raw) {
            Ok(request) => self.handle_request(request).await,
            Err(error) => ApiResponse::error("", format!("Invalid request JSON: {error}")),
        };
        serde_json::to_string(&response).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    pub async fn live_run(&self, session_id: &str, run_id: &str, cursor: u64) -> Result<custos_domain::LiveRunReplay, DomainError> {
        if self.session_manager.get_session(&SessionId(session_id.into())).await.is_none() {
            return Err(DomainError::Validation("Unknown session".into()));
        }
        self.workflow.as_ref().ok_or_else(|| DomainError::Validation("Workflow unavailable".into()))?.live_run(session_id, run_id, cursor).await
    }

    pub async fn handle_request(&self, req: ApiRequest) -> ApiResponse {
        let method_kind = commands::classify_method(&req.method);
        let command_id = req
            .params
            .get("command_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let (admitted_cmd_id, payload_digest) = if method_kind == commands::MethodKind::Mutation {
            let cmd_id = command_id.clone().unwrap_or_else(|| custos_domain::new_id("cmd"));
            let digest = commands::compute_command_fingerprint(&req.params);
            (Some(cmd_id), Some(digest))
        } else {
            (command_id.clone(), None)
        };

        if let (Some(cmd_id), Some(digest), Some(store)) = (
            admitted_cmd_id.as_deref(),
            payload_digest.as_deref(),
            self.sqlite_store.as_ref(),
        ) {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let ttl_ms = 7 * 24 * 60 * 60 * 1000; // 7 days
            let scope = req.method.split('.').nth(1);

            match store.command_ledger().admit(cmd_id, &req.method, digest, scope, now_ms, ttl_ms) {
                Ok(custos_core::contracts::storage::CommandAdmission::New) => {}
                Ok(custos_core::contracts::storage::CommandAdmission::Replay(result_json)) => {
                    let val: serde_json::Value = match serde_json::from_str(&result_json) {
                        Ok(value) => value,
                        Err(error) => return ApiResponse::error(req.id, format!("Stored command result is corrupt: {error}")),
                    };
                    if val.get("custos_command_response") == Some(&serde_json::json!(1)) {
                        return match serde_json::from_value::<ApiResponse>(val["response"].clone()) {
                            Ok(mut response) => { response.id = req.id; response }
                            Err(error) => ApiResponse::error(req.id, format!("Stored command response is corrupt: {error}")),
                        };
                    }
                    return ApiResponse::success(req.id, val);
                }
                Ok(custos_core::contracts::storage::CommandAdmission::InFlight) => {
                    return ApiResponse::error(
                        req.id,
                        format!("Command ID '{cmd_id}' is currently in-flight"),
                    );
                }
                Ok(custos_core::contracts::storage::CommandAdmission::Conflict(msg)) => {
                    return ApiResponse::error(req.id, msg);
                }
                Err(e) => {
                    tracing::error!(command_id = %cmd_id, error = %e, "Command admission failed; mutation denied");
                    return ApiResponse::error(req.id, format!("Command admission failed: {e}"));
                }
            }
        }

        let resp = self.dispatch_application(req).await;

        if let (Some(cmd_id), Some(store)) = (
            admitted_cmd_id.as_deref(),
            self.sqlite_store.as_ref(),
        ) {
            // A handler error may follow an external effect; never make it retryable by deleting admission.
            let result_json = serde_json::json!({"custos_command_response": 1, "response": &resp}).to_string();
            if let Err(e) = store.command_ledger().complete(cmd_id, &result_json) {
                tracing::error!(command_id = %cmd_id, error = %e, "Command settlement failed; admission remains fenced");
                return ApiResponse::error(resp.id, format!("Command '{cmd_id}' may have taken effect, but its result could not be persisted: {e}. Do not retry with a new command ID; reconcile its state first."));
            }
        }

        resp
    }
}

#[async_trait::async_trait]
impl crate::custos_local_api::ApiTransport for LocalApiDispatcher {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        Ok(self.handle_request(req).await)
    }
}


#[cfg(test)]
#[path = "api/tests/mod.rs"]
mod tests;
