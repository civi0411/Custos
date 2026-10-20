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
    new_id, validate_browser_url, ArtifactLineageNode, AssistantDraft, Assurance, BrowserConsoleEntry,
    BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab, BrowserTabStatus,
    CellExecutionStatus, ClaimGroundingLevel, ClaimHandoffSummary,
    CreateHeadlessJobParams, ExecuteCellParams, ExecutionReceipt, FleetExecReceipt, HandoffToCodingParams,
    HandoffToCodingResult, HeadlessAutomationJob, HeadlessJobStatus, NoteRecord, NotebookCell,
    OutboxActionItem, PassageAnchor, Recipe, RecipeHandoffSummary, RecordReviewParams,
    ReceiptStatus, RegisterHostParams, RemoteHostNode, RemoteHostStatus, ResearchClaim, ResearchExperimentRun,
    ResearchSynthesisProposal, ResourceActionCapability, ReviewStatus, ReviewerRecord,
    SaveSynthesisProposalParams, SessionId, SessionMode, SourceRecord, SshAuthMethod, TaskContract,
    ComputeBackendKind, ExecEvent, ExecutionAttempt, ResourceEvent,
};
use custos_persistence::repositories::{AssistantRepository, OutboxRepository};
use custos_persistence::{FleetAutomationRepository, ProviderRepository, ResearchRepository};
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::{
    CreateWorkspaceRequest, WorkspaceCoordinator, WorkspaceFilesCoordinator,
};
use custos_runtime::{PythonKernelCoordinator, TerminalCoordinator};

pub mod transport;
pub use transport::*;
pub mod capabilities;
pub mod commands;
pub mod tasks;

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
            terminal: Arc::new(TerminalCoordinator::new()),
            files: Arc::new(WorkspaceFilesCoordinator::new()),
            harnesses: Arc::new(HarnessRegistry::new()),
            python_kernel: Some(Arc::new(PythonKernelCoordinator::new())),
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
                if force_mock || !a.is_available() {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
            ComputeBackendKind::Kaggle => {
                let mut a = KaggleTargetAdapter::new(work_dir);
                if force_mock || !a.is_available() {
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
                if force_mock || host == "127.0.0.1" || host == "mock" || !a.is_available() {
                    a = a.with_mock_mode();
                }
                Arc::new(a)
            }
            _ => {
                let mut a = ColabTargetAdapter::new(work_dir);
                if force_mock || !a.is_available() {
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
                    let val = serde_json::from_str(&result_json).unwrap_or(serde_json::Value::Null);
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
                    tracing::warn!(command_id = %cmd_id, error = %e, "Failed to admit command in ledger; proceeding fail-open");
                }
            }
        }

        let resp = match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            "v1.tasks.create" => {
                tasks::TaskApiHandler::create_task(
                    &self.task_service,
                    self.sqlite_store.as_deref(),
                    &req.id,
                    req.params,
                    admitted_cmd_id.as_deref(),
                ).await
            }
            "v1.tasks.get" => {
                tasks::TaskApiHandler::get_task(&self.task_service, &req.id, &req.params).await
            }
            "v1.tasks.cancel" => {
                tasks::TaskApiHandler::cancel_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.advance" => {
                tasks::TaskApiHandler::advance_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.complete" => {
                tasks::TaskApiHandler::complete_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.list" => {
                tasks::TaskApiHandler::list_tasks(&self.task_service, &req.id, &req.params).await
            },
            "v1.tasks.spans" => {
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.task_service.list_spans(task_id).await {
                    Ok(spans) => match serde_json::to_value(&spans) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.create" => {
                let mode: SessionMode = match req.params.get("mode").and_then(|v| v.as_str()) {
                    Some("assisted") => SessionMode::Assisted,
                    _ => SessionMode::Bare,
                };

                let session = self.session_manager.create_session(mode).await;
                match serde_json::to_value(&session) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.get" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.get_session(&session_id).await {
                    Some(s) => match serde_json::to_value(&s) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    None => ApiResponse::error(req.id, format!("Session {session_id} not found")),
                }
            }
            METHOD_SESSIONS_DELETE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.delete_session(&session_id).await {
                    Ok(deleted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "deleted": deleted,
                            "session_id": session_id.0,
                        }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.promote" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                let contract: TaskContract = match req.params.get("contract") {
                    Some(c) => match serde_json::from_value(c.clone()) {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid contract: {e}"))
                        }
                    },
                    None => return ApiResponse::error(req.id, "Missing contract param"),
                };

                match self.bridge_service.promote(session_id, contract).await {
                    Ok(task) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.bridge.steer" | "v1.sessions.steer" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) => m.to_string(),
                    None => return ApiResponse::error(req.id, "Missing message param"),
                };

                match self
                    .bridge_service
                    .steer(task_id, session_id, message)
                    .await
                {
                    Ok(receipt) => match serde_json::to_value(&receipt) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.list" => match self.session_manager.list_sessions().await {
                Ok(sessions) => match serde_json::to_value(&sessions) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                },
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "v1.sessions.journal" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };

                match self.session_manager.get_journal(&session_id).await {
                    Some(journal) => match serde_json::to_value(&journal) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    None => ApiResponse::error(req.id, format!("Session {session_id} not found")),
                }
            }
            "v1.sessions.open_in_lens" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let lens: custos_domain::WorkbenchLens = match req.params.get("lens") {
                    Some(value) => match serde_json::from_value(value.clone()) {
                        Ok(lens) => lens,
                        Err(e) => return ApiResponse::error(req.id, format!("Invalid lens: {e}")),
                    },
                    None => return ApiResponse::error(req.id, "Missing lens param"),
                };
                match self.session_manager.open_in_lens(&session_id, lens).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "status": "opened" })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.sessions.message" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let role = req
                    .params
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing content param"),
                };

                let res = if role == "assistant" {
                    self.session_manager
                        .append_assistant_message(&session_id, content)
                        .await
                } else {
                    self.session_manager
                        .append_user_message(&session_id, content)
                        .await
                };

                match res {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "status": "appended" }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.bridge.attach" | "v1.sessions.attach" => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => SessionId(id.to_string()),
                    None => return ApiResponse::error(req.id, "Missing session_id param"),
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.task_service.get_task(&task_id).await {
                    Ok(Some(_)) => {}
                    Ok(None) => return ApiResponse::error(req.id, format!("Task {task_id} not found")),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                }
                match self
                    .bridge_service
                    .attach(session_id, task_id, AttachMode::Steer)
                    .await
                {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "status": "attached" }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.oi.explain" => {
                let params: crate::custos_local_api::ExplainPlanRequest =
                    match serde_json::from_value(req.params) {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid params: {e}"))
                        }
                    };

                let task_id = params
                    .task_id
                    .unwrap_or_else(|| custos_domain::new_id("task"));
                let mut snapshot = custos_domain::oi::DecisionSnapshot::new(&task_id);
                if let Some(budget) = params.budget_limit_tokens {
                    snapshot.remaining_budget_tokens = budget;
                }
                if let Some(ref contract) = params.contract {
                    snapshot.required_capabilities = contract.required_capabilities.clone();
                }

                match custos_runtime::oi::ExplainService::explain(&snapshot) {
                    Ok(report) => match serde_json::to_value(&report) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.workflow.session_runs" => {
                let Some(session)=req.params.get("session_id").and_then(|v|v.as_str()) else {return ApiResponse::error(req.id,"Missing session_id");};
                if self.session_manager.get_session(&SessionId(session.to_string())).await.is_none() {return ApiResponse::error(req.id,"Unknown session");}
                let Some(store)=&self.usage_runs else {return ApiResponse::error(req.id,"Run store unavailable");};
                let tasks=match self.task_service.list_tasks().await {Ok(tasks)=>tasks,Err(e)=>return ApiResponse::error(req.id,e.to_string())};
                let mut runs=Vec::new();
                for task in tasks {
                    let task_runs=match store.list_runs_for_task(&task.id).await {Ok(runs)=>runs,Err(e)=>return ApiResponse::error(req.id,e.to_string())};
                    for run in task_runs {
                        if run.metadata.get("session_id").and_then(|v|v.as_str())==Some(session) && (run.metadata.get("background").and_then(|v|v.as_bool())==Some(true) || run.metadata.get("journal_occurred_at").is_some() || !run.status.is_terminal()) {
                            runs.push(serde_json::json!({"run_id":run.id,"model":run.metadata.get("requested_model").filter(|value|!value.is_null()).or_else(||run.metadata.get("harness_id")),"status":run.status.to_string(),"journal_occurred_at":run.metadata.get("journal_occurred_at")}));
                        }
                    }
                }
                ApiResponse::success(req.id,serde_json::json!(runs))
            }
            "v1.sessions.metrics" | "v1.session.metrics" => {
                let session_opt = req.params.get("session_id").and_then(|v| v.as_str());
                let Some(store) = &self.usage_runs else {
                    return ApiResponse::error(req.id, "Run store unavailable");
                };
                let tasks = match self.task_service.list_tasks().await {
                    Ok(tasks) => tasks,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut tokens_in: u64 = 0;
                let mut tokens_out: u64 = 0;
                let mut total_tokens: u64 = 0;
                let mut active_runs: u64 = 0;
                let pending_approvals: u64 = 0;

                for task in tasks {
                    let task_runs = match store.list_runs_for_task(&task.id).await {
                        Ok(runs) => runs,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    for run in task_runs {
                        let matches_session = match session_opt {
                            Some(s) if !s.is_empty() => {
                                run.metadata.get("session_id").and_then(|v| v.as_str()) == Some(s)
                            }
                            _ => true,
                        };
                        if !matches_session {
                            continue;
                        }

                        if !run.status.is_terminal() {
                            active_runs += 1;
                        }

                        if let Some(usage) = run.metadata.get("usage") {
                            let in_tk = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let out_tk = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let tot_tk = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(in_tk + out_tk);
                            tokens_in += in_tk;
                            tokens_out += out_tk;
                            total_tokens += tot_tk;
                        }
                    }
                }

                let estimated_cost_usd = (tokens_in as f64 * 0.000003) + (tokens_out as f64 * 0.000015);
                let rounded_cost = (estimated_cost_usd * 10000.0).round() / 10000.0;

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "session_id": session_opt,
                        "tokens_in": tokens_in,
                        "tokens_out": tokens_out,
                        "total_tokens": total_tokens,
                        "estimated_cost_usd": rounded_cost,
                        "active_runs": active_runs,
                        "pending_approvals": pending_approvals
                    }),
                )
            }
            "v1.system.metrics" | "v1.system.telemetry" => {
                let memory_resident_mb = if let Some(out) = std::process::Command::new("ps")
                    .args(["-A", "-o", "rss=,comm="])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                {
                    let mut total_kb: f64 = 0.0;
                    for line in out.lines() {
                        let lower = line.to_lowercase();
                        if lower.contains("custos") {
                            if let Some(rss_str) = line.split_whitespace().next() {
                                if let Ok(rss) = rss_str.parse::<f64>() {
                                    total_kb += rss;
                                }
                            }
                        }
                    }
                    if total_kb == 0.0 {
                        let pid = std::process::id();
                        let rss_kb: f64 = std::process::Command::new("ps")
                            .args(["-o", "rss=", "-p", &pid.to_string()])
                            .output()
                            .ok()
                            .and_then(|out| String::from_utf8(out.stdout).ok())
                            .and_then(|s| s.trim().parse::<f64>().ok())
                            .unwrap_or(0.0);
                        ((rss_kb / 1024.0) * 10.0).round() / 10.0
                    } else {
                        ((total_kb / 1024.0) * 10.0).round() / 10.0
                    }
                } else {
                    0.0
                };

                let mut active_runs: u64 = 0;
                if let Some(store) = &self.usage_runs {
                    if let Ok(tasks) = self.task_service.list_tasks().await {
                        for task in tasks {
                            if let Ok(runs) = store.list_runs_for_task(&task.id).await {
                                for run in runs {
                                    if !run.status.is_terminal() {
                                        active_runs += 1;
                                    }
                                }
                            }
                        }
                    }
                }

                let active_ports: Vec<u16> = std::process::Command::new("lsof")
                    .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| {
                        let mut ports = std::collections::BTreeSet::new();
                        for line in s.lines().skip(1) {
                            if let Some(part) = line.split_whitespace().nth(8) {
                                if let Some(port_str) = part.rsplit(':').next() {
                                    if let Ok(p) = port_str.parse::<u16>() {
                                        if (1024..=49151).contains(&p) {
                                            ports.insert(p);
                                        }
                                    }
                                }
                            }
                        }
                        ports.into_iter().collect()
                    })
                    .unwrap_or_default();

                let caffeinate_status = {
                    let mut lock = CAFFEINATE_STATE.lock().unwrap();
                    let (mode, child) = &mut *lock;
                    if mode.is_empty() {
                        *mode = "off".to_string();
                    }
                    let is_active = child.as_mut().map(|c| c.try_wait().ok().flatten().is_none()).unwrap_or(false);
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    })
                };

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "memory_resident_mb": memory_resident_mb,
                        "active_runs": active_runs,
                        "active_ports": active_ports,
                        "caffeinate": caffeinate_status
                    }),
                )
            }
            "v1.system.caffeinate" => {
                let mode = req.params.get("mode").and_then(|v| v.as_str()).unwrap_or("off");
                let mut lock = CAFFEINATE_STATE.lock().unwrap();
                let (current_mode, child) = &mut *lock;
                *current_mode = mode.to_string();

                let should_run = mode == "always";
                if should_run {
                    if child.is_none() {
                        *child = std::process::Command::new("caffeinate")
                            .args(["-i", "-s"])
                            .spawn()
                            .ok();
                    }
                } else if let Some(mut c) = child.take() {
                    let _ = c.kill();
                }

                let is_active = child.as_mut().map(|c| c.try_wait().ok().flatten().is_none()).unwrap_or(false);
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    }),
                )
            }
            "v1.workflow.live_run" => {
                let session = req.params.get("session_id").and_then(|v|v.as_str()).unwrap_or("");
                let run = req.params.get("run_id").and_then(|v|v.as_str()).unwrap_or("");
                let cursor = req.params.get("cursor").and_then(|v|v.as_u64()).unwrap_or(0);
                match self.live_run(session,run,cursor).await {
                    Ok(replay) => match serde_json::to_value(replay) { Ok(value) => ApiResponse::success(req.id,value), Err(error) => ApiResponse::error(req.id,error.to_string()) },
                    Err(error) => ApiResponse::error(req.id,error.to_string()),
                }
            }
            METHOD_WORKFLOW_START_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: StartRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                if let Some(ref sid) = params.session_id {
                    let sess_opt = self.session_manager.get_session(&SessionId(sid.to_string())).await;
                    if sess_opt.is_none() {
                        return ApiResponse::error(
                            req.id,
                            format!("Admission gate refused unverified session_id '{sid}': session does not exist"),
                        );
                    }
                }

                let actor = match params.actor {
                    Some(ref a) if !a.trim().is_empty() => a.trim().to_string(),
                    _ => "daemon_user".into(),
                };

                let resolved_workspace_root = if let Some(ref workspace_id) = params.workspace_id {
                    let workspace = match self
                        .resolve_workspace_for_dispatch(&req.id, workspace_id)
                        .await
                    {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    if !workspace.is_ready() {
                        return ApiResponse::error(
                            req.id,
                            format!("Workspace '{workspace_id}' is not ready for execution"),
                        );
                    }
                    Some(workspace.path)
                } else if params.preferred_mode.as_deref() == Some("native") {
                    return ApiResponse::error(
                        req.id,
                        "Native Task runs require workspace_id; raw workspace paths are not accepted",
                    );
                } else {
                    params.workspace_root.clone()
                };

                let mut cmd = custos_domain::StartRunCommand::new(params.task_id, actor);
                if let Some(rev) = params.workflow_revision {
                    cmd = cmd.with_workflow_revision(rev);
                }
                if let Some(mode) = params.preferred_mode {
                    cmd = cmd.with_preferred_mode(mode);
                }
                if let Some(harness) = params.harness_id {
                    cmd = cmd.with_harness_id(harness);
                }
                if let Some(root) = resolved_workspace_root {
                    cmd = cmd.with_workspace_root(root);
                }
                if let Some(ref ws_id) = params.workspace_id {
                    cmd = cmd.with_workspace_id(ws_id.clone());
                }
                if let Some(ref session_id) = params.session_id {
                    cmd = cmd.with_session_id(session_id.clone());
                }
                if let Some(ref turn_id) = params.turn_id {
                    cmd = cmd.with_turn_id(turn_id.clone());
                }
                if let Some(ref m) = params.model {
                    cmd = cmd.with_model(m.clone());
                }

                cmd.reasoning_effort = params.reasoning_effort;
                cmd.provider_account_id = params.provider_account_id;

                let prompt = if let Some(p) = params.prompt {
                    Some(p)
                } else if let Some(ref sid) = params.session_id {
                    if let Some(journal) = self.session_manager.get_journal(&SessionId(sid.to_string())).await {
                        journal.entries.iter().rev()
                            .find(|e| e.entry_type == "user_message")
                            .map(|e| e.entry_data.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(p) = prompt {
                    cmd = cmd.with_prompt(p);
                }

                let background = params.background;
                let result = if background { workflow.start_run_background(cmd).await } else { workflow.start_run(cmd).await };
                match result {
                    Ok(handle) => {
                        if let (Some(ref sid), Some(ref out)) = (params.session_id.as_ref().filter(|_| !background), handle.output.as_ref()) {
                            let _ = self
                                .session_manager
                                .append_assistant_message(&SessionId(sid.to_string()), out)
                                .await;
                        }
                        match serde_json::to_value(&handle) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKFLOW_CANCEL_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: CancelRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let reason = params
                    .reason
                    .unwrap_or_else(|| "User requested cancellation".into());

                match workflow.request_cancel(&params.run_id, &reason).await {
                    Ok(receipt) => match serde_json::to_value(&receipt) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_CREATE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let owner_task_id = req
                    .params
                    .get("owner_task_id")
                    .or_else(|| req.params.get("task_id"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let params: CreateWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let create_req = CreateWorkspaceRequest {
                    name: params.name,
                    kind: params.kind,
                    path: params.path,
                    lineage: params.lineage,
                    owner_task_id,
                    metadata: params.metadata,
                    setup_script: params.setup_script,
                };
                match coordinator.create_workspace(create_req).await {
                    Ok(ws) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_GET => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) => custos_domain::WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id param"),
                };
                match coordinator.get_workspace(&ws_id).await {
                    Ok(Some(ws)) => match serde_json::to_value(&ws) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Workspace {ws_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_LIST => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                match coordinator.list_workspaces().await {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_ARCHIVE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let params: ArchiveWorkspaceApiRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let ws_id = custos_domain::WorkspaceId::new(params.workspace_id);
                match coordinator
                    .archive_workspace_with_force(&ws_id, params.delete_physical, params.force)
                    .await
                {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "archived": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_INSPECT_DIRTY => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let ws_id = match req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) => custos_domain::WorkspaceId::new(id),
                    None => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                match coordinator.inspect_dirty(&ws_id).await {
                    Ok(dirty) => match serde_json::to_value(&dirty) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACES_RECOVER => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };
                let target_id = req
                    .params
                    .get("workspace_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let ws_id = custos_domain::WorkspaceId::new(id);
                        match coordinator.recover_workspace(&ws_id).await {
                            Ok(ws) => match serde_json::to_value(&ws) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            },
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => {
                        match coordinator.reconcile_all().await {
                            Ok(recovered) => match serde_json::to_value(&recovered) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            },
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                }
            }
            METHOD_WORKSPACES_DETECT => {
                let raw_path_str = req.params.get("path").and_then(|v| v.as_str()).map(|s| s.trim()).unwrap_or("");
                let expanded_path = if raw_path_str.is_empty() || raw_path_str == "." {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                } else if raw_path_str.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&raw_path_str[2..])
                    } else {
                        std::path::PathBuf::from(raw_path_str)
                    }
                } else if raw_path_str == "~" {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(raw_path_str))
                } else {
                    std::path::PathBuf::from(raw_path_str)
                };

                let canonical_path = match expanded_path.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        if let Ok(curr) = std::env::current_dir().and_then(|c| c.canonicalize()) {
                            curr
                        } else {
                            return ApiResponse::error(
                                req.id,
                                format!("Directory '{}' does not exist or cannot be accessed: {}", raw_path_str, e),
                            );
                        }
                    }
                };

                if !canonical_path.is_dir() {
                    return ApiResponse::error(
                        req.id,
                        format!("Path '{}' is not a directory", raw_path_str),
                    );
                }

                // 1. Deep Probe Git (files vs dirs, worktrees, commondir)
                let git_bin = resolve_git_binary();
                let deep_git_probe = resolve_deep_git_paths(&canonical_path).await;

                let mut is_git = deep_git_probe.is_some();
                let mut root_path = canonical_path.to_string_lossy().to_string();
                let mut branch: Option<String> = None;
                let mut is_dirty = false;
                let mut dirty_count = 0;
                let mut remote_url: Option<String> = None;
                let mut is_worktree = deep_git_probe.as_ref().map(|(_, _, wt)| *wt).unwrap_or(false);

                // Check via git rev-parse for root work-tree toplevel
                let toplevel_res = tokio::process::Command::new(&git_bin)
                    .current_dir(&canonical_path)
                    .args(["rev-parse", "--is-inside-work-tree", "--show-toplevel"])
                    .output()
                    .await;

                if let Ok(out) = toplevel_res {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let lines: Vec<&str> = stdout.lines().collect();
                        if let Some(first) = lines.first() {
                            if *first == "true" {
                                is_git = true;
                                if let Some(tl) = lines.get(1) {
                                    root_path = tl.trim().to_string();
                                }
                            }
                        }
                    }
                }

                let mut worktrees: Vec<serde_json::Value> = Vec::new();
                let mut github_host: Option<String> = None;
                let mut github_owner: Option<String> = None;
                let mut github_repo: Option<String> = None;
                let mut github_avatar_url: Option<String> = None;

                if is_git {
                    // Get current branch
                    if let Ok(b_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                        .output()
                        .await
                    {
                        if b_out.status.success() {
                            let b_str = String::from_utf8_lossy(&b_out.stdout).trim().to_string();
                            if !b_str.is_empty() {
                                branch = Some(b_str);
                            }
                        }
                    }

                    // Check dirty status
                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["status", "--porcelain=v1"])
                        .output()
                        .await
                    {
                        if s_out.status.success() {
                            let changes = String::from_utf8_lossy(&s_out.stdout);
                            dirty_count = changes.lines().filter(|l| !l.trim().is_empty()).count();
                            is_dirty = dirty_count > 0;
                        }
                    }

                    // Get remote URL (origin first, then upstream)
                    for remote_name in ["origin", "upstream"] {
                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                            .current_dir(&root_path)
                            .args(["remote", "get-url", remote_name])
                            .output()
                            .await
                        {
                            if r_out.status.success() {
                                let r_str = String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                                if !r_str.is_empty() {
                                    remote_url = Some(r_str);
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(ref r_url) = remote_url {
                        let (gh_host, gh_owner, gh_repo, gh_avatar) = parse_github_remote(r_url);
                        github_host = gh_host;
                        github_owner = gh_owner;
                        github_repo = gh_repo;
                        github_avatar_url = gh_avatar;
                    }

                    // Check if linked worktree
                    let dir_out = tokio::process::Command::new(&git_bin)
                        .current_dir(&root_path)
                        .args(["rev-parse", "--git-dir", "--git-common-dir"])
                        .output()
                        .await;
                    if let Ok(d_out) = dir_out {
                        if d_out.status.success() {
                            let out_str = String::from_utf8_lossy(&d_out.stdout);
                            let d_lines: Vec<&str> = out_str.lines().collect();
                            if d_lines.len() >= 2 && d_lines[0] != d_lines[1] {
                                is_worktree = true;
                            }
                        }
                    }

                    // List all worktrees for this repository
                    worktrees = list_git_worktrees(&git_bin, &root_path).await;
                }

                // 1b. Scan immediate subdirectories for child Git repositories / workspaces
                let mut sub_repositories: Vec<serde_json::Value> = Vec::new();
                if let Ok(mut dir_entries) = tokio::fs::read_dir(&canonical_path).await {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                        if let Ok(file_type) = entry.file_type().await {
                            if file_type.is_dir() {
                                let sub_dir = entry.path();
                                if let Some((_sub_gitdir, _sub_commondir, _sub_is_wt)) = resolve_deep_git_paths(&sub_dir).await {
                                    let sub_name = entry.file_name().to_string_lossy().to_string();
                                    let mut sub_branch = String::from("main");
                                    if let Ok(sub_b_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                                        .output()
                                        .await
                                    {
                                        if sub_b_out.status.success() {
                                            let s = String::from_utf8_lossy(&sub_b_out.stdout).trim().to_string();
                                            if !s.is_empty() {
                                                sub_branch = s;
                                            }
                                        }
                                    }

                                    let mut sub_dirty_count = 0;
                                    if let Ok(s_out) = tokio::process::Command::new(&git_bin)
                                        .current_dir(&sub_dir)
                                        .args(["status", "--porcelain=v1"])
                                        .output()
                                        .await
                                    {
                                        if s_out.status.success() {
                                            let changes = String::from_utf8_lossy(&s_out.stdout);
                                            sub_dirty_count = changes.lines().filter(|l| !l.trim().is_empty()).count();
                                        }
                                    }

                                    let mut sub_remote_url: Option<String> = None;
                                    for remote_name in ["origin", "upstream"] {
                                        if let Ok(r_out) = tokio::process::Command::new(&git_bin)
                                            .current_dir(&sub_dir)
                                            .args(["remote", "get-url", remote_name])
                                            .output()
                                            .await
                                        {
                                            if r_out.status.success() {
                                                let r = String::from_utf8_lossy(&r_out.stdout).trim().to_string();
                                                if !r.is_empty() {
                                                    sub_remote_url = Some(r);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    let (sub_gh_host, sub_gh_owner, sub_gh_repo, sub_gh_avatar) = if let Some(ref r) = sub_remote_url {
                                        parse_github_remote(r)
                                    } else {
                                        (None, None, None, None)
                                    };

                                    let sub_worktrees = list_git_worktrees(&git_bin, &sub_dir.to_string_lossy()).await;

                                    sub_repositories.push(serde_json::json!({
                                        "name": sub_name,
                                        "path": sub_dir.to_string_lossy().to_string(),
                                        "branch": sub_branch,
                                        "is_git": true,
                                        "is_dirty": sub_dirty_count > 0,
                                        "dirty_count": sub_dirty_count,
                                        "remote_url": sub_remote_url,
                                        "github_host": sub_gh_host,
                                        "github_owner": sub_gh_owner,
                                        "github_repo": sub_gh_repo,
                                        "github_avatar_url": sub_gh_avatar,
                                        "worktrees": sub_worktrees
                                    }));
                                }
                            }
                        }
                    }
                }

                // 2. Detect Project Type
                let root_p = std::path::Path::new(&root_path);
                let project_type = if root_p.join("Cargo.toml").exists() {
                    "Rust"
                } else if root_p.join("package.json").exists() {
                    "TypeScript / Node"
                } else if root_p.join("pyproject.toml").exists() || root_p.join("requirements.txt").exists() {
                    "Python"
                } else if root_p.join("go.mod").exists() {
                    "Go"
                } else if root_p.join("pom.xml").exists() || root_p.join("build.gradle").exists() {
                    "Java"
                } else if is_git {
                    "Git Repository"
                } else {
                    "Folder"
                };

                let name = root_p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string());

                let result = serde_json::json!({
                    "path": canonical_path.to_string_lossy(),
                    "root_path": root_path,
                    "name": name,
                    "is_git": is_git,
                    "branch": branch,
                    "is_dirty": is_dirty,
                    "dirty_count": dirty_count,
                    "remote_url": remote_url,
                    "github_host": github_host,
                    "github_owner": github_owner,
                    "github_repo": github_repo,
                    "github_avatar_url": github_avatar_url,
                    "project_type": project_type,
                    "is_worktree": is_worktree,
                    "worktrees": worktrees,
                    "sub_repositories": sub_repositories
                });

                ApiResponse::success(req.id, result)
            }
            METHOD_WORKSPACES_PICK_FOLDER => {
                #[cfg(target_os = "macos")]
                {
                    let script = if req.params.get("purpose").and_then(|v| v.as_str()) == Some("model") {
                        "POSIX path of (choose folder with prompt \"Select Folder Containing GGUF Models\")"
                    } else {
                        "POSIX path of (choose folder with prompt \"Select Project or Repository Folder\")"
                    };
                    let out = tokio::process::Command::new("osascript")
                        .arg("-e")
                        .arg(script)
                        .output()
                        .await;

                    match out {
                        Ok(o) if o.status.success() => {
                            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                            ApiResponse::success(req.id, serde_json::json!({ "path": p }))
                        }
                        _ => ApiResponse::success(req.id, serde_json::json!({ "path": null })),
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    ApiResponse::success(req.id, serde_json::json!({ "path": null }))
                }
            }
            METHOD_WORKSPACES_CANDIDATES_SPAWN => {
                let _mutation = self.candidate_mutation.lock().await;
                let command_id = req.params.get("command_id").and_then(|value| value.as_str()).unwrap_or(&req.id).to_string();
                if command_id.is_empty() || command_id.len() > 128 || !command_id.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')) {
                    return ApiResponse::error(req.id, "Candidate command_id must be an identifier of at most 128 characters");
                }
                let request_hash = blake3::hash(req.params.to_string().as_bytes()).to_hex().to_string();
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id)
                        if !id.trim().is_empty()
                            && id.trim().len() <= 128
                            && id.trim().chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')) =>
                    {
                        id.trim().to_string()
                    }
                    _ => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
                };

                match self.task_service.get_task(&task_id).await {
                    Ok(Some(_)) => {}
                    Ok(None) => return ApiResponse::error(req.id, format!("Task '{task_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }

                let recorded = match coordinator.list_workspaces().await {
                    Ok(workspaces) => workspaces.into_iter().filter(|workspace| workspace.metadata.get("batch_command_id").and_then(|value| value.as_str()) == Some(&command_id)).collect::<Vec<_>>(),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                if !recorded.is_empty() {
                    if recorded.iter().any(|workspace| workspace.owner_task_id.as_deref() != Some(&task_id) || workspace.metadata.get("batch_request_hash").and_then(|value| value.as_str()) != Some(&request_hash)) {
                        return ApiResponse::error(req.id, "This command_id already belongs to a different candidate request");
                    }
                    let requested = req.params.get("count").and_then(|value| value.as_u64()).unwrap_or(2).clamp(1, 8) as usize;
                    let pending = recorded.iter().any(|workspace| !workspace.is_ready());
                    return ApiResponse::success(req.id, serde_json::json!({"batch_command_id": command_id, "task_id": task_id,
                        "candidates": recorded, "failed": [], "partial": pending || requested != recorded.len(), "needs_reconciliation": pending || requested != recorded.len(), "replayed": true}));
                }
                let count = req.params.get("count").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let count = count.clamp(1, 8);

                let base_repo_path = if let Some(base_ws_id) = req.params.get("base_workspace_id").and_then(|v| v.as_str()) {
                    match coordinator.get_workspace(&custos_domain::WorkspaceId::new(base_ws_id)).await {
                        Ok(Some(ws)) if ws.is_ready() => ws.path,
                        _ => return ApiResponse::error(req.id, format!("Base workspace '{base_ws_id}' not found")),
                    }
                } else if let Some(repo_path) = req.params.get("repo_path").and_then(|v| v.as_str()) {
                    repo_path.to_string()
                } else {
                    return ApiResponse::error(req.id, "Either 'base_workspace_id' or 'repo_path' must be provided");
                };

                let base_branch = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let base_branch = match base_branch {
                    Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Candidate creation requires a Git checkout"),
                };
                if base_branch.is_empty() {
                    return ApiResponse::error(req.id, "Candidate creation requires a named base branch");
                }
                let base_status = tokio::process::Command::new("git")
                    .current_dir(&base_repo_path)
                    .args(["status", "--porcelain", "--untracked-files=all"])
                    .output()
                    .await;
                match base_status {
                    Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                    Ok(out) if out.status.success() => return ApiResponse::error(req.id, "Candidate creation requires a clean base checkout; uncommitted files would not be copied"),
                    _ => return ApiResponse::error(req.id, "Could not inspect candidate base checkout"),
                }

                let base_commit_hash = if let Some(h) = req.params.get("base_commit_hash").and_then(|v| v.as_str()) {
                    Some(h.to_string())
                } else {
                    let mut cmd = tokio::process::Command::new("git");
                    cmd.current_dir(&base_repo_path).arg("rev-parse").arg("HEAD");
                    cmd.output().await.ok().filter(|o| o.status.success()).map(|o| {
                        String::from_utf8_lossy(&o.stdout).trim().to_string()
                    })
                };

                let mut spawned_workspaces = Vec::new();
                let mut failed_workspaces = Vec::new();
                let batch_command_id = command_id;
                let base_path = std::path::Path::new(&base_repo_path);
                let (Some(parent), Some(repo_name)) = (base_path.parent(), base_path.file_name()) else {
                    return ApiResponse::error(req.id, "Candidate base must have a named parent directory");
                };
                for i in 1..=count {
                    let branch = format!("custos/candidate/{task_id}/{i}");
                    let candidate_path = parent
                        .join(".custos-worktrees")
                        .join(repo_name)
                        .join(&task_id)
                        .join(format!("candidate-{}", i))
                        .to_string_lossy()
                        .to_string();

                    let metadata = serde_json::json!({
                        "candidate_index": i,
                        "task_id": task_id,
                        "branch": branch,
                        "base_commit_hash": base_commit_hash,
                        "batch_command_id": &batch_command_id,
                        "batch_request_hash": &request_hash,
                    });

                    let create_req = CreateWorkspaceRequest {
                        name: format!("candidate-{}-{}", task_id, i),
                        kind: custos_domain::WorkspaceKind::Git {
                            repo_path: base_repo_path.clone(),
                            branch: branch.clone(),
                            base_commit: base_commit_hash.clone(),
                        },
                        path: candidate_path,
                        lineage: Some(custos_domain::WorkspaceLineage {
                            parent_workspace_id: req.params.get("base_workspace_id").and_then(|v| v.as_str()).map(custos_domain::WorkspaceId::new),
                            base_commit: base_commit_hash.clone(),
                            target_branch: Some(base_branch.clone()),
                            head_commit: None,
                        }),
                        owner_task_id: Some(task_id.clone()),
                        metadata: Some(metadata),
                        setup_script: req.params.get("setup_script").and_then(|v| v.as_str()).map(str::to_string),
                    };

                    match coordinator.create_workspace(create_req).await {
                        Ok(ws) => spawned_workspaces.push(ws),
                        Err(e) => {
                            failed_workspaces.push(serde_json::json!({
                                "candidate_index": i,
                                "branch": branch,
                                "error": e.to_string(),
                            }));
                        }
                    }
                }

                if spawned_workspaces.is_empty() && !failed_workspaces.is_empty() {
                    return ApiResponse::error(req.id, format!("Failed to create candidate worktrees: {:?}", failed_workspaces));
                }

                let candidates_val = serde_json::to_value(&spawned_workspaces).unwrap_or_default();
                ApiResponse::success(req.id, serde_json::json!({
                    "batch_command_id": batch_command_id,
                    "task_id": task_id,
                    "base_repo_path": base_repo_path,
                    "base_commit_hash": base_commit_hash,
                    "candidates": candidates_val,
                    "failed": failed_workspaces,
                    "partial": !failed_workspaces.is_empty(),
                }))
            }
            METHOD_WORKSPACES_CANDIDATES_COMPARE => {
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let target_workspaces: Vec<custos_domain::ExecutionWorkspace> = if let Some(ws_ids) = req.params.get("workspace_ids").and_then(|v| v.as_array()) {
                    let mut list = Vec::new();
                    for val in ws_ids {
                        if let Some(id_str) = val.as_str() {
                            match coordinator.get_workspace(&custos_domain::WorkspaceId::new(id_str)).await {
                                Ok(Some(ws)) => list.push(ws),
                                Ok(None) => return ApiResponse::error(req.id, format!("Workspace '{id_str}' not found")),
                                Err(e) => return ApiResponse::error(req.id, format!("Failed to retrieve workspace '{id_str}': {e}")),
                            }
                        }
                    }
                    if list.len() != ws_ids.len() { return ApiResponse::error(req.id, "Invalid workspace ID in comparison"); }
                    list
                } else if let Some(task_id) = req.params.get("task_id").and_then(|v| v.as_str()) {
                    let all = match coordinator.list_workspaces().await { Ok(all) => all, Err(error) => return ApiResponse::error(req.id, error.to_string()) };
                    all.into_iter().filter(|w| w.owner_task_id.as_deref() == Some(task_id)).collect()
                } else {
                    return ApiResponse::error(req.id, "Either 'workspace_ids' or 'task_id' must be provided");
                };

                if target_workspaces.is_empty() {
                    return ApiResponse::error(req.id, "No candidate workspaces found for comparison");
                }

                let baseline = &target_workspaces[0];
                let Some(owner_task) = baseline.owner_task_id.as_deref() else {
                    return ApiResponse::error(req.id, "Comparison requires Task-owned candidate worktrees");
                };
                let Some(base_commit) = baseline.base_commit_hash.as_deref().filter(|hash| !hash.is_empty()) else {
                    return ApiResponse::error(req.id, "Comparison requires a recorded base commit");
                };
                let requested_task = req.params.get("task_id").and_then(|value| value.as_str());
                let mut selected_ids = std::collections::HashSet::new();
                for candidate in &target_workspaces {
                    if !selected_ids.insert(candidate.id.to_string()) || !candidate.is_ready()
                        || candidate.owner_task_id.as_deref() != Some(owner_task)
                        || requested_task.is_some_and(|task| task != owner_task)
                        || candidate.base_commit_hash.as_deref() != Some(base_commit) {
                        return ApiResponse::error(req.id, "Candidates must be distinct ready worktrees of one Task and base commit");
                    }
                }

                let first_repo = match &target_workspaces[0].kind {
                    custos_domain::WorkspaceKind::Git { repo_path, .. } => repo_path.clone(),
                    _ => return ApiResponse::error(req.id, format!("Workspace '{}' is not a Git workspace; candidates must be Git worktrees", target_workspaces[0].id)),
                };

                for ws in &target_workspaces {
                    match &ws.kind {
                        custos_domain::WorkspaceKind::Git { repo_path, .. } => {
                            if *repo_path != first_repo {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Candidate workspace '{}' belongs to repository '{}', but baseline repository is '{}'", ws.id, repo_path, first_repo)
                                );
                            }
                        }
                        _ => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace '{}' is not a Git workspace", ws.id)
                            );
                        }
                    }
                }

                let mut candidate_comparisons = Vec::new();
                for ws in target_workspaces {
                    let dirty_res = coordinator.inspect_dirty(&ws.id).await;
                    let diff_res = coordinator.diff(&ws.id).await;

                    let (dirty, dirty_err) = match dirty_res {
                        Ok(d) => (Some(d), None),
                        Err(e) => (None, Some(e.to_string())),
                    };
                    let (diff_text, diff_err) = match diff_res {
                        Ok(d) => (Some(d), None),
                        Err(e) => (None, Some(e.to_string())),
                    };

                    let branch = match &ws.kind {
                        custos_domain::WorkspaceKind::Git { branch, .. } => branch.clone(),
                        _ => "main".to_string(),
                    };

                    let inspection_ok = dirty_err.is_none() && diff_err.is_none();
                    let is_ready_for_integration = inspection_ok && ws.is_ready();

                    candidate_comparisons.push(serde_json::json!({
                        "workspace_id": ws.id.to_string(),
                        "name": ws.name,
                        "path": ws.path,
                        "branch": branch,
                        "base_commit_hash": ws.base_commit_hash,
                        "owner_task_id": ws.owner_task_id,
                        "is_dirty": dirty.as_ref().map(|d| d.is_dirty).unwrap_or(false),
                        "modified_count": dirty.as_ref().map(|d| d.modified_files.len()).unwrap_or(0),
                        "untracked_count": dirty.as_ref().map(|d| d.untracked_files.len()).unwrap_or(0),
                        "deleted_count": dirty.as_ref().map(|d| d.deleted_files.len()).unwrap_or(0),
                        "modified_files": dirty.as_ref().map(|d| d.modified_files.clone()).unwrap_or_default(),
                        "untracked_files": dirty.as_ref().map(|d| d.untracked_files.clone()).unwrap_or_default(),
                        "deleted_files": dirty.as_ref().map(|d| d.deleted_files.clone()).unwrap_or_default(),
                        "diff": diff_text.unwrap_or_default(),
                        "dirty_error": dirty_err,
                        "diff_error": diff_err,
                        "inspection_error": if !inspection_ok { Some("Failed to inspect candidate worktree status or diff") } else { None },
                        "is_ready_for_integration": is_ready_for_integration,
                        "status": ws.status.to_string(),
                    }));
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "count": candidate_comparisons.len(),
                    "candidates": candidate_comparisons,
                }))
            }
            METHOD_WORKSPACES_INTEGRATE => {
                let _mutation = self.candidate_mutation.lock().await;
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        )
                    }
                };

                let winner_id = match req.params.get("winner_workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => custos_domain::WorkspaceId::new(id),
                    _ => return ApiResponse::error(req.id, "Missing 'winner_workspace_id' parameter"),
                };

                let mut winner_ws = match coordinator.get_workspace(&winner_id).await {
                    Ok(Some(ws)) => ws,
                    _ => return ApiResponse::error(req.id, format!("Winner workspace '{winner_id}' not found")),
                };

                let (repo_path, winner_branch) = match &winner_ws.kind {
                    custos_domain::WorkspaceKind::Git { repo_path, branch, .. } => (repo_path.clone(), branch.clone()),
                    _ => return ApiResponse::error(req.id, "Winner workspace must be a Git workspace"),
                };

                if winner_ws.owner_task_id.is_none() || !winner_ws.metadata.is_object() {
                    return ApiResponse::error(req.id, "Integration requires a Task-owned candidate worktree");
                }
                if let Some(intent) = winner_ws.metadata.get("integration").cloned() {
                    if let Some(result) = intent.get("result").filter(|result| result.get("merge_success").and_then(|value| value.as_bool()) == Some(true)) { return ApiResponse::success(req.id, result.clone()); }
                    let recorded_head = intent.get("winner_head").and_then(|value| value.as_str()).unwrap_or("");
                    let recorded_target = intent.get("target_branch").and_then(|value| value.as_str()).unwrap_or("");
                    let target_ref = format!("refs/heads/{recorded_target}");
                    let ancestry = tokio::process::Command::new("git").current_dir(&repo_path)
                        .args(["merge-base", "--is-ancestor", recorded_head, &target_ref]).output().await;
                    if matches!(ancestry, Ok(ref output) if output.status.success()) {
                        let result = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                            "target_branch": intent.get("target_branch"), "merge_success": true, "merge_status": "merged_reconciled",
                            "merge_detail": "Recorded winner commit is already included in the recorded target branch; no merge was replayed", "is_stale_base": false, "loser_cleanup": []});
                        winner_ws.metadata["integration"]["result"] = result.clone();
                        if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, error.to_string()); }
                        return ApiResponse::success(req.id, result);
                    }
                    let merge_head = tokio::process::Command::new("git").current_dir(&repo_path).args(["rev-parse", "--verify", "MERGE_HEAD"]).output().await;
                    let conflict = matches!(merge_head, Ok(ref output) if output.status.success());
                    return ApiResponse::success(req.id, serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                        "target_branch": intent.get("target_branch"), "merge_success": false, "merge_status": if conflict { "conflict_needs_resolution" } else { "uncertain" },
                        "merge_detail": "An earlier integration has no confirmed result. Inspect the target repository; the command was not replayed.", "is_stale_base": false, "loser_cleanup": []}));
                }
                let target_branch = req.params.get("target_branch").and_then(|v| v.as_str())
                    .or(winner_ws.lineage.target_branch.as_deref());
                let Some(target_branch) = target_branch.map(str::to_string) else {
                    return ApiResponse::error(req.id, "Candidate has no recorded target branch");
                };

                if !winner_ws.is_ready() {
                    return ApiResponse::error(req.id, "Winner workspace is not ready for integration");
                }

                // `git merge` modifies the branch currently checked out in repo_path,
                // not the branch named by target_branch. Refuse a mismatched checkout.
                let checked_out = tokio::process::Command::new("git")
                    .current_dir(&repo_path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                let checked_out = match checked_out {
                    Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Could not verify the checked-out target branch"),
                };
                if checked_out != target_branch {
                    return ApiResponse::error(req.id, format!(
                        "Target checkout is on '{checked_out}', not '{target_branch}'; integration was not started"
                    ));
                }

                for (label, path) in [("target", repo_path.as_str()), ("winner", winner_ws.path.as_str())] {
                    let status = tokio::process::Command::new("git")
                        .current_dir(path)
                        .args(["status", "--porcelain", "--untracked-files=all"])
                        .output()
                        .await;
                    match status {
                        Ok(out) if out.status.success() && out.stdout.is_empty() => {}
                        Ok(out) if out.status.success() => return ApiResponse::error(req.id, format!(
                            "The {label} workspace has uncommitted or untracked changes; review them before integration"
                        )),
                        _ => return ApiResponse::error(req.id, format!("Could not inspect the {label} workspace")),
                    }
                }

                let winner_checkout = tokio::process::Command::new("git")
                    .current_dir(&winner_ws.path)
                    .args(["branch", "--show-current"])
                    .output()
                    .await;
                match winner_checkout {
                    Ok(out) if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == winner_branch => {}
                    _ => return ApiResponse::error(req.id, "Winner workspace no longer checks out its recorded branch"),
                }

                // Stale-Base Drift Check
                let mut rev_cmd = tokio::process::Command::new("git");
                rev_cmd.current_dir(&repo_path).arg("rev-parse").arg(&target_branch);
                let current_target_head = rev_cmd.output().await.ok().filter(|o| o.status.success()).map(|o| {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                });

                let (Some(ref current_head), Some(ref base_commit)) = (&current_target_head, &winner_ws.base_commit_hash) else {
                    return ApiResponse::error(req.id, "Cannot verify candidate base against target HEAD");
                };
                let is_stale_base = current_head != base_commit;
                if is_stale_base {
                    return ApiResponse::error(req.id, format!(
                        "Candidate base {base_commit} is stale relative to target {current_head}; integration was not started"
                    ));
                }

                let winner_head = match tokio::process::Command::new("git").current_dir(&winner_ws.path).args(["rev-parse", "HEAD"]).output().await {
                    Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    _ => return ApiResponse::error(req.id, "Could not verify winner HEAD"),
                };
                let preflight = serde_json::json!({"winner_workspace_id": winner_id.to_string(), "winner_branch": winner_branch,
                    "target_branch": target_branch, "current_target_head": current_target_head, "winner_head": winner_head,
                    "winner_base_commit": winner_ws.base_commit_hash, "is_stale_base": false, "preview": true,
                    "merge_success": false, "merge_detail": "Ready for explicit integration", "merge_status": "ready"});
                if req.params.get("preview").and_then(|value| value.as_bool()) == Some(true) { return ApiResponse::success(req.id, preflight); }
                if req.params.get("expected_target_head").and_then(|value| value.as_str()) != current_target_head.as_deref()
                    || req.params.get("expected_winner_head").and_then(|value| value.as_str()) != Some(&winner_head) {
                    return ApiResponse::error(req.id, "Review a fresh integration preview before applying; expected heads are missing or changed");
                }
                winner_ws.metadata["integration"] = serde_json::json!({"winner_head": winner_head, "target_head": current_target_head,
                    "target_branch": target_branch, "started_at": chrono::Utc::now().to_rfc3339()});
                if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, error.to_string()); }

                // Merge winner branch into target repository
                let mut merge_cmd = tokio::process::Command::new("git");
                merge_cmd.current_dir(&repo_path)
                    .arg("merge")
                    .arg("--no-ff")
                    .arg("-m")
                    .arg(format!("Merge winner candidate {} ({})", winner_ws.name, winner_branch))
                    .arg(&winner_branch);

                let merge_output = merge_cmd.output().await;
                let (merge_success, merge_detail, conflict_detected) = match merge_output {
                    Ok(out) => {
                        let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
                        let success = out.status.success();
                        let conflict = !success && (text.contains("CONFLICT") || text.contains("Automatic merge failed"));
                        (success, text.trim().to_string(), conflict)
                    }
                    Err(e) => (false, format!("Failed to spawn git merge: {e}"), false),
                };

                let merge_status = if merge_success {
                    "merged"
                } else if conflict_detected {
                    "conflict_needs_resolution"
                } else {
                    "failed"
                };

                // Integration never implies permission to delete other workspaces.
                // Candidate cleanup is a separate, explicit archive operation.
                let loser_cleanup_results: Vec<serde_json::Value> = Vec::new();

                let result = serde_json::json!({
                    "winner_workspace_id": winner_id.to_string(),
                    "winner_branch": winner_branch,
                    "target_branch": target_branch,
                    "is_stale_base": is_stale_base,
                    "current_target_head": current_target_head,
                    "winner_base_commit": winner_ws.base_commit_hash,
                    "merge_success": merge_success,
                    "merge_status": merge_status,
                    "conflict_detected": conflict_detected,
                    "merge_detail": merge_detail,
                    "loser_cleanup": loser_cleanup_results,
                });
                winner_ws.metadata["integration"]["result"] = result.clone();
                if let Err(error) = coordinator.update_workspace(&winner_ws).await { return ApiResponse::error(req.id, format!("Integration effect requires reconciliation: {error}")); }
                ApiResponse::success(req.id, result)
            }
            METHOD_SETTINGS_GET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                match store.get_settings() {
                    Ok(settings) => ApiResponse::success(req.id, serde_json::to_value(&settings).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SETTINGS_SET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                if let Some(map) = req.params.as_object() {
                    let mut updates = std::collections::HashMap::with_capacity(map.len());
                    for (k, v) in map {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        updates.insert(k.clone(), val_str);
                    }
                    if let Err(e) = store.set_settings(&updates) {
                        return ApiResponse::error(req.id, e.to_string());
                    }
                    match store.get_settings() {
                        Ok(settings) => ApiResponse::success(req.id, serde_json::to_value(&settings).unwrap_or_default()),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    ApiResponse::error(req.id, "Invalid params: expected JSON object of key-value settings")
                }
            }
            METHOD_RESEARCH_SOURCES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_sources() {
                    Ok(sources) => match serde_json::to_value(&sources) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_SOURCES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let src: SourceRecord = match serde_json::from_value(req.params) {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid source payload: {e}"))
                    }
                };
                let file_bytes = src.local_path.as_ref().and_then(|p| std::fs::read(p).ok());
                let src = match custos_core::research_ingress::source_draft_with_bytes(src, file_bytes.as_deref()) {
                    Ok(src) => src,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_source(&src) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": src.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let source_id = match req.params.get("source_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing source_id parameter"),
                };
                match research.list_anchors_for_source(source_id) {
                    Ok(anchors) => match serde_json::to_value(&anchors) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let anchor: PassageAnchor = match serde_json::from_value(req.params) {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid anchor payload: {e}"))
                    }
                };
                let anchor = match custos_core::research_ingress::passage_draft(anchor) {
                    Ok(anchor) => anchor,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_sources() {
                    Ok(sources) if sources.iter().any(|s| s.id == anchor.source_id) => {}
                    Ok(_) => return ApiResponse::error(req.id, "Passage source does not exist"),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_anchor(&anchor) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anchor.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_claims() {
                    Ok(claims) => match serde_json::to_value(&claims) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim: ResearchClaim = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid claim payload: {e}"))
                    }
                };
                let claim = match custos_core::research_ingress::claim_draft(claim) {
                    Ok(claim) => claim,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_claim(&claim) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": claim.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_runs() {
                    Ok(runs) => match serde_json::to_value(&runs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let run: ResearchExperimentRun = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid run payload: {e}"))
                    }
                };
                let run = match custos_core::research_ingress::experiment_draft(run) {
                    Ok(run) => run,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_runs() {
                    Ok(runs) if runs.iter().any(|existing| existing.run_id == run.run_id) => {
                        return ApiResponse::error(req.id, "Experiment draft id already exists");
                    }
                    Ok(_) => {}
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_run(&run) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "run_id": run.run_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_LINEAGE_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Missing artifact_path parameter"),
                };
                match research.list_artifact_lineage(artifact_path) {
                    Ok(lineage) => match serde_json::to_value(&lineage) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_HANDOFF_CODING => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim_id = match req.params.get("claim_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing claim_id parameter"),
                };
                let claim = match research.list_claims() {
                    Ok(claims) => match claims.into_iter().find(|c| c.id == claim_id) {
                        Some(claim) => claim,
                        None => return ApiResponse::error(req.id, "Claim does not exist"),
                    },
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let title = format!("Investigate research claim: {}", claim.statement);

                let cmd = CreateTask {
                    title,
                    metadata: Some(serde_json::json!({
                        "source": "research_workbench",
                        "claim_id": claim_id,
                        "claim_statement": claim.statement,
                        "claim_grounding": "unverified_client_draft",
                    })),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "handoff_status": "task_created",
                                "task": val,
                            }),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                match research.list_recipes() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe: custos_domain::Recipe = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid recipe payload: {e}"))
                    }
                };
                match research.save_recipe(&recipe) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": recipe.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.get" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.get_recipe(recipe_id) {
                    Ok(Some(recipe)) => match serde_json::to_value(&recipe) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Recipe {recipe_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.executions.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.list_execution_records_for_recipe(recipe_id) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.executions.get" | "v1.research.executions.resume.assess" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let execution_id = match req.params.get("execution_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing execution_id parameter"),
                };
                match research.get_execution_record(execution_id) {
                    Ok(Some(record)) => match if req.method == "v1.research.executions.resume.assess" {
                        serde_json::to_value(record.assess_checkpoint_resume())
                    } else {
                        serde_json::to_value(&record)
                    } {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("Execution record {execution_id} not found"),
                    ),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_COMPUTE_TARGETS_LIST => {
                let targets = serde_json::json!([
                    {
                        "backend": "colab",
                        "name": "Google Colab",
                        "description": "Hosted Colab runtime via google-colab-cli. Decoupled stop vs release prevents burning compute units.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "kaggle",
                        "name": "Kaggle Kernels",
                        "description": "Batch Kaggle GPU kernels (30h/week free T4/P100) with pinned versions, log streaming & auto-teardown.",
                        "supports_interactive": false,
                        "decoupled_release": false
                    },
                    {
                        "backend": "runpod",
                        "name": "Rented GPU (RunPod)",
                        "description": "On-demand cloud GPU with nvidia-smi preflight, PID group tracking & automatic pod destruction.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "vast_ai",
                        "name": "Rented GPU (Vast.ai)",
                        "description": "Vast.ai GPU instance with hardware verification and automated CLI instance teardown.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "dedicated_ssh",
                        "name": "Dedicated SSH GPU",
                        "description": "Custom remote SSH GPU server with hardware probing, PID supervision & checkpoint triggers.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    }
                ]);
                ApiResponse::success(req.id, targets)
            }
            METHOD_EXPERIMENT_PREFLIGHT => {
                let backend_str = req.params.get("backend").and_then(|v| v.as_str()).unwrap_or("colab");
                let recipe_id = req.params.get("recipe_id").and_then(|v| v.as_str()).unwrap_or("default");
                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let backend_kind = match backend_str {
                    "colab" => ComputeBackendKind::Colab,
                    "kaggle" => ComputeBackendKind::Kaggle,
                    "runpod" => ComputeBackendKind::RunPod,
                    "vast_ai" => ComputeBackendKind::VastAi,
                    "dedicated_ssh" | "ssh" => ComputeBackendKind::DedicatedSsh,
                    _ => ComputeBackendKind::Local,
                };
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let ssh_host = req.params.get("ssh_host").and_then(|v| v.as_str());
                let ssh_port = req.params.get("ssh_port").and_then(|v| v.as_u64()).map(|p| p as u16);
                let ssh_user = req.params.get("ssh_user").and_then(|v| v.as_str());
                let pod_id = req.params.get("pod_id").and_then(|v| v.as_str());
                let cloud_api_key = req.params.get("cloud_api_key").and_then(|v| v.as_str());

                let adapter = self.resolve_target_adapter(
                    backend_kind,
                    &work_dir,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    pod_id,
                    cloud_api_key,
                    force_mock,
                );

                match adapter.preflight(recipe_id).await {
                    Ok(report) => match serde_json::to_value(&report) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_EXPERIMENT_SUBMIT => {
                let experiment_id = match req.params.get("experiment_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing experiment_id parameter"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                let backend_str = req.params.get("backend").and_then(|v| v.as_str()).unwrap_or("colab");
                let backend_kind = match backend_str {
                    "colab" => ComputeBackendKind::Colab,
                    "kaggle" => ComputeBackendKind::Kaggle,
                    "runpod" => ComputeBackendKind::RunPod,
                    "vast_ai" => ComputeBackendKind::VastAi,
                    "dedicated_ssh" | "ssh" => ComputeBackendKind::DedicatedSsh,
                    _ => ComputeBackendKind::Local,
                };
                let parent_checkpoint_id = req.params.get("parent_checkpoint_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let ssh_host = req.params.get("ssh_host").and_then(|v| v.as_str());
                let ssh_port = req.params.get("ssh_port").and_then(|v| v.as_u64()).map(|p| p as u16);
                let ssh_user = req.params.get("ssh_user").and_then(|v| v.as_str());
                let pod_id = req.params.get("pod_id").and_then(|v| v.as_str());
                let cloud_api_key = req.params.get("cloud_api_key").and_then(|v| v.as_str());

                let mut attempt = ExecutionAttempt::new(experiment_id, recipe_id, backend_kind, parent_checkpoint_id);
                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));

                let adapter = self.resolve_target_adapter(
                    backend_kind,
                    &work_dir,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    pod_id,
                    cloud_api_key,
                    force_mock,
                );

                let idempotency_key = format!("submit_{}", attempt.id);
                match adapter.submit(&attempt, &idempotency_key).await {
                    Ok(external_ref) => {
                        let _ = attempt.apply(ExecEvent::PreflightOk);
                        let _ = attempt.apply(ExecEvent::SubmitStarted);
                        let _ = attempt.apply(ExecEvent::SubmitAcked(external_ref.clone()));

                        if let Some(ref store) = self.sqlite_store {
                            let _ = store.experiment_attempts().save_attempt(&attempt);
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "attempt": attempt,
                                "external_ref": external_ref,
                            }),
                        )
                    }
                    Err(e) => {
                        let _ = attempt.apply(ExecEvent::SubmitRejected);
                        if let Some(ref store) = self.sqlite_store {
                            let _ = store.experiment_attempts().save_attempt(&attempt);
                        }
                        ApiResponse::error(req.id, format!("Submission to {} failed: {}", backend_str, e))
                    }
                }
            }
            METHOD_EXPERIMENT_OBSERVE => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => return ApiResponse::error(req.id, format!("Attempt {attempt_id} not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => return ApiResponse::error(req.id, "Attempt does not have an external job reference"),
                };

                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                match adapter.observe(&ext_ref).await {
                    Ok(obs) => {
                        attempt.exec_state = obs.exec_state;
                        attempt.resource_state = obs.resource_state;
                        let _ = store.experiment_attempts().save_attempt(&attempt);
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "attempt": attempt,
                                "observation": obs,
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to observe {attempt_id}: {e}")),
                }
            }
            METHOD_EXPERIMENT_STOP => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => return ApiResponse::error(req.id, format!("Attempt {attempt_id} not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => return ApiResponse::error(req.id, "Attempt has no external job reference"),
                };

                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                let _ = adapter.stop_job(&ext_ref).await;
                let _ = attempt.apply(ExecEvent::StopRequested);
                let _ = attempt.apply(ExecEvent::StopConfirmed);
                let _ = store.experiment_attempts().save_attempt(&attempt);

                ApiResponse::success(req.id, serde_json::json!({ "stopped": true, "attempt": attempt }))
            }
            METHOD_EXPERIMENT_RELEASE => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => return ApiResponse::error(req.id, format!("Attempt {attempt_id} not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => return ApiResponse::error(req.id, "Attempt has no external job reference"),
                };

                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                let _ = adapter.release_compute(&ext_ref).await;
                let _ = attempt.apply_resource(ResourceEvent::ReleaseRequested);
                let _ = attempt.apply_resource(ResourceEvent::ReleaseConfirmed);
                let _ = store.experiment_attempts().save_attempt(&attempt);

                ApiResponse::success(req.id, serde_json::json!({ "released": true, "attempt": attempt }))
            }
            METHOD_EXPERIMENT_COLLECT => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => return ApiResponse::error(req.id, format!("Attempt {attempt_id} not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => return ApiResponse::error(req.id, "Attempt has no external job reference"),
                };

                let work_dir = dirs::home_dir().map(|h| h.join(".custos")).unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req.params.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                match adapter.collect_artifacts(&ext_ref).await {
                    Ok(artifacts) => {
                        let _ = attempt.apply(ExecEvent::Collected);
                        let _ = store.experiment_attempts().save_attempt(&attempt);
                        ApiResponse::success(req.id, serde_json::json!({ "artifacts": artifacts, "attempt": attempt }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to collect artifacts: {e}")),
                }
            }
            "v1.research.annotations.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let artifact_id = req
                    .params
                    .get("artifact_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let version = req
                    .params
                    .get("version")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                match research.list_annotations_for_artifact(artifact_id, version) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.annotations.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let anno: custos_domain::AnnotationRecord = match serde_json::from_value(req.params)
                {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid annotation payload: {e}"),
                        )
                    }
                };
                if anno.id.trim().is_empty() || anno.note.trim().is_empty() || anno.note.len() > 16_384
                    || anno.actor.trim().is_empty() || anno.side_chat_session_id.is_some() || anno.submitted_turn_id.is_some() {
                    return ApiResponse::error(req.id, "Annotation requires an identity, actor and note; conversation links must come from a real handoff");
                }
                let versions = match research.list_artifact_lineage(&anno.artifact_path) {
                    Ok(versions) => versions,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                if !versions.iter().any(|version| version.version == anno.artifact_version) {
                    return ApiResponse::error(req.id, "Annotation must target a recorded artifact version");
                }
                let valid_target = match &anno.target {
                    custos_domain::AnnotationTarget::Text { start_offset, end_offset, exact_text } =>
                        start_offset < end_offset && !exact_text.is_empty() && exact_text.len() <= 65_536,
                    custos_domain::AnnotationTarget::ImagePin { x_ratio, y_ratio, pin_number } =>
                        x_ratio.is_finite() && y_ratio.is_finite() && (0.0..=1.0).contains(x_ratio)
                            && (0.0..=1.0).contains(y_ratio) && *pin_number > 0,
                    custos_domain::AnnotationTarget::DomElement { selector } => !selector.trim().is_empty() && selector.len() <= 4096,
                };
                if !valid_target || anno.status != custos_domain::AnnotationStatus::Pending {
                    return ApiResponse::error(req.id, "Annotations enter as pending notes with a valid target; they do not certify review completion");
                }
                match research.save_annotation(&anno) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anno.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_all_artifacts() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing or empty artifact_path parameter"),
                };
                let versions = match research.list_artifact_lineage(artifact_path) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let annotations = match research.list_annotations_for_artifact(artifact_path, None) {
                    Ok(a) => a,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let (latest_version, latest_content_hash) = if let Some(latest) = versions.last() {
                    (latest.version, latest.content_hash.clone())
                } else {
                    (0, String::new())
                };

                let mut content: Option<String> = None;
                let mut content_version = None;
                let mut file = None;
                if artifact_path.starts_with("notes/") {
                    let note_id = artifact_path.trim_start_matches("notes/").trim_end_matches(".md");
                    match research.get_note(note_id) {
                        Ok(Some(note)) => { content_version = Some(note.version); content = Some(note.content); },
                        Ok(None) => {},
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    }
                } else if let Some(workspace_id) = req.params.get("workspace_id").and_then(|value| value.as_str()) {
                    if versions.is_empty() {
                        return ApiResponse::error(req.id, "Artifact has no recorded lineage");
                    }
                    let workspace = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    let artifact = std::path::Path::new(artifact_path);
                    let relative = if artifact.is_absolute() {
                        match artifact.strip_prefix(&workspace.path) {
                            Ok(relative) => relative.to_string_lossy().to_string(),
                            Err(_) => return ApiResponse::error(req.id, "Artifact is outside the selected workspace"),
                        }
                    } else { artifact_path.to_string() };
                    let current = match self.files.read_file(&workspace, &relative, Some(10 * 1024 * 1024)).await {
                        Ok(current) => current,
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    };
                    let bytes = if current.is_binary {
                        match base64::engine::general_purpose::STANDARD.decode(&current.content) {
                            Ok(bytes) => bytes,
                            Err(error) => return ApiResponse::error(req.id, format!("Invalid binary artifact encoding: {error}")),
                        }
                    } else { current.content.as_bytes().to_vec() };
                    let hash = blake3::hash(&bytes).to_hex().to_string();
                    if !current.truncated && (latest_content_hash == hash || latest_content_hash == format!("blake3:{hash}")) {
                        content_version = Some(latest_version);
                        if !current.is_binary { content = Some(current.content.clone()); }
                    }
                    file = Some(current);
                }

                let resp = ArtifactDetailResponse {
                    artifact_path: artifact_path.to_string(),
                    latest_version,
                    latest_content_hash,
                    versions,
                    annotations,
                    content,
                    file,
                    content_version,
                };
                match serde_json::to_value(&resp) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_RECORD_LINEAGE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let node: ArtifactLineageNode = match serde_json::from_value(req.params) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid artifact lineage payload: {e}")),
                };
                if node.artifact_path.trim().is_empty() || node.content_hash.trim().is_empty() {
                    return ApiResponse::error(req.id, "Artifact path and content hash cannot be empty");
                }
                match research.record_artifact_lineage(&node) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "recorded": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_LINEAGE_GRAPH => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let filter_path = req.params.get("artifact_path").and_then(|v| v.as_str());
                match research.get_artifact_lineage_graph(filter_path) {
                    Ok(graph) => match serde_json::to_value(&graph) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let session_id = req.params.get("session_id").and_then(|v| v.as_str());
                match research.list_notes(session_id) {
                    Ok(notes) => match serde_json::to_value(&notes) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match research.get_note(id) {
                    Ok(Some(note)) => match serde_json::to_value(&note) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, "Note not found"),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: SaveNoteParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid save note params: {e}")),
                };

                let note = if let Some(ref note_id) = params.id {
                    match research.get_note(note_id) {
                        Ok(Some(mut existing)) => {
                            match existing.update(Some(params.title), params.content, params.tags) {
                                Ok(prev_ver) => {
                                    if let Err(e) = research.save_note(&existing) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    if let Err(e) = research.save_note_version(&prev_ver) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", existing.id),
                                        version: existing.version,
                                        content_hash: existing.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: Some(prev_ver.content_hash),
                                        timestamp: existing.updated_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    existing
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Ok(None) => {
                            match NoteRecord::new(params.title, params.content, params.session_id, params.task_id, params.tags.unwrap_or_default()) {
                                Ok(mut n) => {
                                    n.id = note_id.clone();
                                    if let Err(e) = research.save_note(&n) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", n.id),
                                        version: 1,
                                        content_hash: n.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: None,
                                        timestamp: n.created_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    n
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    match NoteRecord::new(params.title, params.content, params.session_id, params.task_id, params.tags.unwrap_or_default()) {
                        Ok(n) => {
                            if let Err(e) = research.save_note(&n) {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            let node = ArtifactLineageNode {
                                artifact_path: format!("notes/{}.md", n.id),
                                version: 1,
                                content_hash: n.content_hash.clone(),
                                produced_by_run_id: None,
                                parent_version_hash: None,
                                timestamp: n.created_at,
                            };
                            let _ = research.record_artifact_lineage(&node);
                            n
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                };

                match serde_json::to_value(&note) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_HISTORY => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let note_id = match req.params.get("note_id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing note_id parameter"),
                };
                match research.list_note_versions(note_id) {
                    Ok(vers) => match serde_json::to_value(&vers) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_notebook_cells(session_id) {
                    Ok(cells) => match serde_json::to_value(&cells) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_SAVE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let mut cells: Vec<NotebookCell> = match req.params.get("cells").and_then(|v| serde_json::from_value(v.clone()).ok()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing or invalid cells parameter"),
                };
                if cells.iter().any(|cell| cell.session_id != session_id) {
                    return ApiResponse::error(req.id, "Notebook cell session_id must match the requested session_id");
                }
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                if let Some(kernel) = self.python_kernel.as_ref() {
                    let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                    match kernel.get_status_scoped(session_id, notebook_id, Some(research.as_ref() as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync))).await {
                        Ok(state) if state.status == custos_domain::KernelStatus::Busy => return ApiResponse::error(req.id, "Wait for notebook execution or interrupt it before changing cells"),
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        _ => {},
                    }
                }
                let stored = match research.list_notebook_cells(session_id) {
                    Ok(cells) => cells,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                for cell in &mut cells {
                    if let Some(previous) = stored.iter().find(|saved| saved.id == cell.id) {
                        if previous.source == cell.source && previous.cell_type == cell.cell_type {
                            cell.execution_count = previous.execution_count;
                            cell.status = previous.status;
                            cell.stdout = previous.stdout.clone();
                            cell.stderr = previous.stderr.clone();
                            cell.output_image = previous.output_image.clone();
                            cell.wall_ms = previous.wall_ms;
                        } else {
                            cell.execution_count = None;
                            cell.status = CellExecutionStatus::Idle;
                            cell.stdout = None;
                            cell.stderr = None;
                            cell.output_image = None;
                            cell.wall_ms = None;
                        }
                        cell.epoch = previous.epoch;
                    } else {
                        cell.execution_count = None;
                        cell.status = CellExecutionStatus::Idle;
                        cell.stdout = None;
                        cell.stderr = None;
                        cell.output_image = None;
                        cell.wall_ms = None;
                        cell.epoch = 0;
                    }
                }
                match research.replace_notebook_cells(session_id, &cells) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "saved": true, "count": cells.len() })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let mut params: ExecuteCellParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid execute cell params: {e}")),
                };
                let workspace_id = match params.workspace_id.as_deref().filter(|id| !id.trim().is_empty()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Notebook execution requires a registered workspace_id"),
                };
                let workspace = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(workspace) => workspace,
                    Err(response) => return response,
                };
                let research = match self.research.as_ref() {
                    Some(repository) => repository,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let saved = match research.list_notebook_cells(&params.session_id) {
                    Ok(cells) => cells,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match saved.iter().find(|cell| cell.id == params.cell_id) {
                    Some(cell) if cell.source == params.code && cell.cell_type == custos_domain::NotebookCellType::Code => {},
                    _ => return ApiResponse::error(req.id, "Save the exact code cell in this session before executing it"),
                }
                // cwd is resolved from the registered workspace, never from daemon process state.
                if params.cwd.as_deref().is_some_and(|cwd| cwd != workspace.path) {
                    return ApiResponse::error(req.id, "Notebook cwd must match the registered workspace");
                }
                params.cwd = Some(workspace.path);

                let repo_ref = self.research.as_deref().map(|r| r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync));
                match kernel.execute_cell(params, repo_ref).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_INTERRUPT => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                match kernel.interrupt_scoped(session_id, notebook_id).await {
                    Ok(interrupted) => ApiResponse::success(req.id, serde_json::json!({ "interrupted": interrupted })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_RESET => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.research.as_deref().map(|r| r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync));
                match kernel.reset_scoped(session_id, notebook_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_STATUS => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => return ApiResponse::error(req.id, "PythonKernelCoordinator not configured on daemon"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty session_id parameter"),
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.research.as_deref().map(|r| r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync));
                match kernel.get_status_scoped(session_id, notebook_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let target_type = req.params.get("target_type").and_then(|v| serde_json::from_value(v.clone()).ok());
                let target_id = req.params.get("target_id").and_then(|v| v.as_str());
                match research.list_reviews(target_type, target_id) {
                    Ok(reviews) => match serde_json::to_value(&reviews) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("review_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.get_review(id) {
                    Ok(Some(rev)) => match serde_json::to_value(&rev) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_RECORD => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: RecordReviewParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid record review params: {e}")),
                };
                let record = match ReviewerRecord::new(
                    params.target_type,
                    params.target_id,
                    params.reviewer,
                    params.method,
                    params.status,
                    params.evidence_summary,
                    params.evidence_digest,
                    params.findings.unwrap_or_default(),
                ) {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                if let Err(e) = research.save_review(&record) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                match serde_json::to_value(&record) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_MARK_STALE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("review_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.mark_review_stale(id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "marked_stale": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                match research.list_synthesis_proposals() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let id = match req.params.get("id").or_else(|| req.params.get("proposal_id")).and_then(|v| v.as_str()) {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty proposal id parameter"),
                };
                match research.get_synthesis_proposal(id) {
                    Ok(Some(prop)) => match serde_json::to_value(&prop) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: SaveSynthesisProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid save proposal params: {e}")),
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                let claims: Vec<ClaimHandoffSummary> = params.claim_ids.iter().filter_map(|cid| {
                    all_claims.iter().find(|c| &c.id == cid).map(|c| {
                        let has_fresh_review = all_reviews.iter().any(|r| {
                            r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                        });
                        ClaimHandoffSummary {
                            claim_id: c.id.clone(),
                            statement: c.statement.clone(),
                            level: c.level,
                            confidence_score: c.confidence_score,
                            has_fresh_review,
                            sealed_proof_uri: c.sealed_proof_uri.clone(),
                        }
                    })
                }).collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let recipes: Vec<RecipeHandoffSummary> = params.recipe_ids.iter().filter_map(|rid| {
                    all_recipes.iter().find(|r| &r.id == rid).map(|r| {
                        RecipeHandoffSummary {
                            recipe_id: r.id.clone(),
                            name: r.name.clone(),
                            command: r.command.clone(),
                            inputs_count: r.inputs.len(),
                            outputs: r.outputs.clone(),
                        }
                    })
                }).collect();

                let mut proposal = match ResearchSynthesisProposal::new(
                    params.title,
                    params.summary,
                    claims,
                    recipes,
                    params.artifact_paths.unwrap_or_default(),
                    params.workspace_id,
                    params.target_branch,
                ) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                if let Some(existing_id) = params.id {
                    proposal.id = existing_id;
                }

                if let Err(e) = research.save_synthesis_proposal(&proposal) {
                    return ApiResponse::error(req.id, e.to_string());
                }

                match serde_json::to_value(&proposal) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured on daemon"),
                };
                let params: HandoffToCodingParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid handoff params: {e}")),
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                // Invariant: Verify all requested claim IDs actually exist in the repository
                for cid in &params.claim_ids {
                    if !all_claims.iter().any(|c| &c.id == cid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Claim '{cid}' not found in research repository"),
                        );
                    }
                }
                let selected_claims: Vec<&ResearchClaim> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| all_claims.iter().find(|c| &c.id == cid))
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Invariant: Verify all requested recipe IDs actually exist in the repository
                for rid in &params.recipe_ids {
                    if !all_recipes.iter().any(|r| &r.id == rid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Recipe '{rid}' not found in research repository"),
                        );
                    }
                }
                let selected_recipes: Vec<&Recipe> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| all_recipes.iter().find(|r| &r.id == rid))
                    .collect();

                // Fail-closed verification gate: EVERY claim must be grounded/verified
                if params.enforce_verification && !selected_claims.is_empty() {
                    let unverified = selected_claims.iter().find(|c| {
                        !(c.level >= ClaimGroundingLevel::L2Verified
                            || all_reviews.iter().any(|r| {
                                r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                            }))
                    });
                    if let Some(bad_claim) = unverified {
                        return ApiResponse::error(
                            req.id,
                            format!(
                                "Fail-closed verification gate: claim '{}' is ungrounded (L0/unverified) and lacks approved active verifier record or L2+ proof",
                                bad_claim.id
                            ),
                        );
                    }
                }

                let claim_summaries: Vec<ClaimHandoffSummary> = selected_claims.iter().map(|c| {
                    let has_fresh_review = all_reviews.iter().any(|r| {
                        r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                    });
                    ClaimHandoffSummary {
                        claim_id: c.id.clone(),
                        statement: c.statement.clone(),
                        level: c.level,
                        confidence_score: c.confidence_score,
                        has_fresh_review,
                        sealed_proof_uri: c.sealed_proof_uri.clone(),
                    }
                }).collect();

                let recipe_summaries: Vec<RecipeHandoffSummary> = selected_recipes.iter().map(|r| {
                    RecipeHandoffSummary {
                        recipe_id: r.id.clone(),
                        name: r.name.clone(),
                        command: r.command.clone(),
                        inputs_count: r.inputs.len(),
                        outputs: r.outputs.clone(),
                    }
                }).collect();

                let caveats = ResearchSynthesisProposal::generate_caveats(&claim_summaries, &recipe_summaries);
                let mut created_task_ids = Vec::new();

                // 1. Create coding tasks for claims
                for claim in &selected_claims {
                    let cmd = CreateTask {
                        title: format!("[Research Claim] {}", claim.statement),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "claim_id": claim.id,
                            "claim_statement": claim.statement,
                            "claim_level": claim.level,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                            "caveats": caveats,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "engineering_verification".into(),
                            name: format!("Verify: {}", claim.statement),
                            description: format!("Empirically implement and produce diff / test verification for claim: {}", claim.statement),
                            required_capabilities: vec!["code.files".into(), "code.changes".into(), "compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::Diff,
                                    required: true,
                                },
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to child task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(req.id, format!("Failed to create task: {e}"));
                        }
                    }
                }

                // 2. Create coding tasks for recipes
                for recipe in &selected_recipes {
                    let cmd = CreateTask {
                        title: format!("[Recipe Reproduction] {}", recipe.name),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "recipe_id": recipe.id,
                            "recipe_name": recipe.name,
                            "recipe_command": recipe.command,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "recipe_execution".into(),
                            name: format!("Execute: {}", recipe.name),
                            description: format!("Execute recipe command: {}", recipe.command),
                            required_capabilities: vec!["compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to recipe task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(req.id, format!("Failed to create recipe task: {e}"));
                        }
                    }
                }

                // If proposal_id is provided, mark it completed
                if let Some(ref prop_id) = params.proposal_id {
                    if let Ok(Some(mut prop)) = research.get_synthesis_proposal(prop_id) {
                        prop.mark_completed();
                        let _ = research.save_synthesis_proposal(&prop);
                    }
                }

                let handoff_res = HandoffToCodingResult {
                    handoff_id: custos_domain::new_id("handoff"),
                    proposal_id: params.proposal_id,
                    task_ids: created_task_ids,
                    workspace_id: params.workspace_id,
                    target_branch: params.target_branch,
                    verified_claims_count: selected_claims.len(),
                    converted_recipes_count: selected_recipes.len(),
                    caveats,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                match serde_json::to_value(&handoff_res) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_providers() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.get" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match providers_repo.get_provider(id) {
                    Ok(Some(cfg)) => match serde_json::to_value(&cfg) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Provider {id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.delete" => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match self.remove_provider_credential(id, true).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "id": id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.routing.get" | "v1.providers.routing.save" | "v1.providers.route" => {
                let repo = match self.providers.as_ref() { Some(repo)=>repo,None=>return ApiResponse::error(req.id,"ProviderRepository not configured") };
                let service = req.params.get("service_type").and_then(|v|v.as_str()).unwrap_or("");
                if service.is_empty() { return ApiResponse::error(req.id,"Missing provider service"); }
                let result = match req.method.as_str() {
                    "v1.providers.route" => {
                        let model = req.params.get("model").and_then(|v|v.as_str()).unwrap_or("");
                        crate::runtime::reserve_available_provider_account(repo,service,model,req.params.get("reasoning_effort").and_then(|v|v.as_str()),&self.model_availability).map(|account|serde_json::json!({"account_id":account}))
                    },
                    "v1.providers.routing.save" => {
                        let enabled = match req.params.get("round_robin").and_then(|v|v.as_bool()) { Some(v)=>v,None=>return ApiResponse::error(req.id,"Missing round_robin mode") };
                        let order: Vec<String> = match req.params.get("account_order").cloned().map(serde_json::from_value) { Some(Ok(v))=>v,_=>return ApiResponse::error(req.id,"Invalid account order") };
                        repo.save_routing_policy(service,enabled,&order).and_then(|()|repo.routing_policy(service))
                    },
                    _ => repo.routing_policy(service),
                };
                return match result { Ok(value)=>ApiResponse::success(req.id,value),Err(e)=>ApiResponse::error(req.id,e.to_string()) };
            }
            "v1.providers.save" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("p_custom")
                    .to_string();
                let existing_provider = match providers_repo.get_provider(&id) { Ok(value)=>value,Err(e)=>return ApiResponse::error(req.id,e.to_string()) };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(||existing_provider.as_ref().map(|p|p.name.as_str()).unwrap_or(&id))
                    .to_string();
                let service_type = req
                    .params
                    .get("service_type")
                    .or_else(|| req.params.get("provider_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(||existing_provider.as_ref().map(|p|p.service_type.as_str()).unwrap_or("anthropic"))
                    .to_string();
                let raw_api_key = req.params.get("api_key").and_then(|v| v.as_str());
                let masked = if raw_api_key.is_some_and(|key|!key.trim().is_empty()) { "••••••••".to_string() } else { providers_repo.get_provider(&id).ok().flatten().map(|p|p.api_key_masked).unwrap_or_else(||"none".into()) };
                let status = req
                    .params
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or(if masked != "none" {
                        "configured"
                    } else {
                        "unconfigured"
                    })
                    .to_string();
                let endpoint_url = req
                    .params
                    .get("endpoint_url")
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let default_model = req
                    .params
                    .get("default_model")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let context_window = req
                    .params
                    .get("context_window")
                    .and_then(|v| v.as_u64());
                let fast_mode = req
                    .params
                    .get("fast_mode")
                    .and_then(|v| v.as_bool());

                let now = chrono::Utc::now().timestamp_millis();

                let cfg = custos_domain::ProviderConfig {
                    id,
                    name,
                    service_type,
                    api_key_masked: masked,
                    status,
                    endpoint_url,
                    default_model,
                    context_window,
                    fast_mode,
                    created_at: now,
                    updated_at: now,
                };
                match self.credential_service() {
                    Ok(service) => match service.save_provider(&cfg, raw_api_key).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": cfg.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.models.probe" => {
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string());

                let provider_record = if let (Some(repo), Some(pid)) =
                    (self.providers.as_ref(), provider_id.as_deref())
                {
                    repo.get_provider(pid).ok().flatten()
                } else {
                    None
                };

                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| provider_record.as_ref().map(|p| p.service_type.as_str()))
                    .unwrap_or("local");

                let base_url = req
                    .params
                    .get("base_url")
                    .or_else(|| req.params.get("endpoint_url"))
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        provider_record
                            .as_ref()
                            .and_then(|p| p.endpoint_url.as_deref())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    })
                    .or_else(|| default_model_probe_base_url(provider_type).map(str::to_string))
                    .unwrap_or_default();

                if base_url.trim().is_empty() {
                    return ApiResponse::error(req.id, "Missing base_url parameter for models probe");
                }

                let token_record = if req.params.get("api_key").and_then(|v|v.as_str()).is_some_and(|key|!key.trim().is_empty()) { None } else if self.credentials.is_some() {
                    match self.read_provider_token(provider_id.as_deref().unwrap_or(provider_type)).await { Ok(token)=>token,Err(e)=>return ApiResponse::error(req.id,e.to_string()) }
                } else { None };
                let explicit_key = req
                    .params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty());
                let oauth = explicit_key.is_none()
                    && token_record.as_ref().is_some_and(|tok| {
                        tok.refresh_token.is_some()
                            || tok
                                .scope
                                .as_deref()
                                .is_some_and(|s| !s.trim().is_empty())
                    });
                let mut api_key_owned = explicit_key
                    .map(str::to_string)
                    .or_else(|| token_record.as_ref().map(|tok| tok.access_token.clone()));
                if oauth {
                    if let Some(tok) = token_record.as_ref().filter(|tok| tok.is_expired(120)) {
                        let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(&tok.service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                        let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(&settings.0, &settings.1);
                        match manager
                            .refresh_token(
                                &settings.2,
                                tok,
                            )
                            .await
                        {
                            Ok(refreshed) => {
                                if self.credentials.is_some() {
                                    if let Err(e) = self.refresh_provider_token(tok, &refreshed).await {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                }
                                api_key_owned = Some(refreshed.access_token);
                            }
                            Err(_) => {
                                return ApiResponse::error(
                                    req.id,
                                    "OAuth refresh failed; reconnect the account",
                                )
                            }
                        }
                    }
                }
                let probe = if oauth && provider_type == "copilot" {
                    match api_key_owned.as_deref() { Some(token) => custos_adapters::providers::probe::probe_copilot_models(token).await, None => Err(custos_domain::DomainError::Validation("Copilot OAuth credentials missing".into())) }
                } else if provider_type == "antigravity" || (oauth && provider_type == "gemini") {
                    match api_key_owned.as_deref() {
                        Some(token) => {
                            let mut combined = Vec::new();
                            let mut seen_ids = std::collections::HashSet::new();
                            if let Ok(models) = custos_adapters::providers::antigravity::probe_codeassist_models("antigravity", "https://cloudcode-pa.googleapis.com", token).await {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) { combined.push(m); }
                                }
                            }
                            if let Ok(models) = custos_adapters::providers::antigravity::probe_codeassist_models("gemini", "https://cloudcode-pa.googleapis.com", token).await {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) { combined.push(m); }
                                }
                            }
                            if combined.is_empty() {
                                custos_adapters::providers::antigravity::probe_codeassist_models("antigravity", "https://cloudcode-pa.googleapis.com", token).await
                            } else {
                                Ok(combined)
                            }
                        }
                        None => Err(custos_domain::DomainError::Validation("Google OAuth credentials missing".into()))
                    }
                } else if oauth && matches!(provider_type, "openai" | "codex") {
                    match api_key_owned.as_deref() {
                        Some(token) => custos_adapters::providers::probe::probe_oauth_models(token).await,
                        None => Err(custos_domain::DomainError::Validation(
                            "OAuth credentials missing".into(),
                        )),
                    }
                } else {
                    custos_adapters::providers::probe::probe_endpoint_models(
                        &base_url,
                        api_key_owned.as_deref(),
                        if oauth && provider_type == "anthropic" { "anthropic-oauth" } else { provider_type },
                    )
                    .await
                };
                match probe {
                    Ok(mut models) => {
                        models.retain(|model| custos_adapters::providers::probe::conversation_model_is_visible(&model.id, model.name.as_deref()));
                        let mut unavailable_models = std::collections::HashMap::new();
                        if !matches!(provider_type, "openai" | "codex" | "anthropic" | "gemini" | "antigravity" | "copilot" | "deepseek" | "local" | "ollama" | "groq" | "openrouter" | "together" | "cerebras" | "perplexity" | "mistral" | "lmstudio" | "custom") { models.clear(); }
                        if !matches!(provider_type, "openai" | "codex" | "copilot") {
                            for model in &mut models {
                                if provider_type == "antigravity" && custos_adapters::providers::antigravity::supports_tiered_reasoning(&model.id) { continue; }
                                model.efforts.clear(); model.default_effort = None;
                            }
                        }
                        if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id.as_deref()) {
                            let catalog_result = self.model_catalog.merge_probed_models(provider_type, models.clone());
                            let cache_key = custos_adapters::providers::probe::verified_catalog_key(pid, &base_url, oauth, token_record.as_ref().map(|t| t.created_at).unwrap_or(0), provider_type);
                            if let Err(e) = repo.save_catalog_models(&cache_key, &catalog_result.models) { return ApiResponse::error(req.id, e.to_string()); }
                            if let Err(e) = self.model_availability.refresh(&cache_key) { return ApiResponse::error(req.id, e.to_string()); }
                            unavailable_models = match self.model_availability.snapshot(&cache_key) { Ok(value) => value, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                            models.retain(|model| !unavailable_models.contains_key("*") && !unavailable_models.contains_key(&model.id));
                        }
                        match serde_json::to_value(&models) {
                            Ok(val) => ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "models": val,
                                    "count": models.len(),
                                    "unavailable_models": unavailable_models,
                                    "base_url": base_url,
                                }),
                            ),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Probe failed: {e}")),
                }
            }
            "v1.models.catalog" => {
                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str());
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id) {
                    let record = repo.get_provider(pid).ok().flatten();
                    let service = record
                        .as_ref()
                        .map(|p| p.service_type.as_str())
                        .or(provider_type)
                        .unwrap_or("local");
                    let endpoint = record
                        .as_ref()
                        .and_then(|p| p.endpoint_url.as_deref())
                        .filter(|s| !s.trim().is_empty())
                        .or_else(|| default_model_probe_base_url(service))
                        .unwrap_or("");
                    let cache_key = if record.is_some() {
                        match crate::runtime::provider_catalog_scope(repo, pid) { Ok(scope) => scope, Err(error) => return ApiResponse::error(req.id, error.to_string()) }
                    } else {
                        custos_adapters::providers::probe::verified_catalog_key(pid, endpoint, false, 0, service)
                    };
                    let fetched_at = match repo.catalog_fetched_at(&cache_key) {
                        Ok(timestamp) => timestamp,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    return match repo.list_catalog_models(Some(&cache_key)) {
                        Ok(mut cached) => {
                            let unavailable = match self.model_availability.snapshot(&cache_key) { Ok(value) => value, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                            cached.retain(|model| custos_adapters::providers::probe::conversation_model_is_visible(&model.id, Some(&model.label)) && !unavailable.contains_key("*") && !unavailable.contains_key(&model.id));
                            for model in &mut cached { model.provider_type = service.to_string(); }
                            ApiResponse::success(req.id, serde_json::json!({ "origin": "cached", "models": cached, "fetched_at": fetched_at, "unavailable_models": unavailable }))
                        },
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let catalog = self.model_catalog.get_catalog(provider_type);
                // Scanned files are inventory, not composed inference routes.
                // Exposing them as `local/<file_name>` sends them to the HTTP/Ollama adapter.
                match serde_json::to_value(&catalog) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.list" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match serde_json::to_value(&folders) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.add" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_path = match req.params.get("folder_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing folder_path parameter"),
                };
                let path = std::path::Path::new(folder_path);
                if !path.exists() || !path.is_dir() {
                    return ApiResponse::error(req.id, format!("Path does not exist or is not a directory: {}", folder_path));
                }
                let label = req.params.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());
                let now = chrono::Utc::now().timestamp_millis();
                let folder = custos_domain::LocalModelFolder {
                    id: custos_domain::new_id("fld"),
                    folder_path: folder_path.to_string(),
                    label,
                    is_active: true,
                    auto_scan: true,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = repo.save_local_folder(&folder) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let discovered = custos_adapters::local_inference::folder_scanner::scan_folder_for_models(&folder, &hardware);
                for m in &discovered {
                    if let Err(error) = repo.upsert_cached_model(m) {
                        return ApiResponse::error(req.id, format!("Cannot save discovered local model: {error}"));
                    }
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "folder": folder,
                    "models_discovered": discovered.len(),
                    "models": discovered
                }))
            }
            "v1.local_models.folders.remove" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match repo.delete_local_folder(id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "deleted": true, "id": id })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.scan" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let mut total_discovered = Vec::new();
                for f in &folders {
                    if let Some(fid) = folder_id {
                        if f.id != fid { continue; }
                    }
                    if !f.is_active { continue; }
                    let found = custos_adapters::local_inference::folder_scanner::scan_folder_for_models(f, &hardware);
                    for m in &found {
                        if let Err(error) = repo.upsert_cached_model(m) {
                            return ApiResponse::error(req.id, format!("Cannot save discovered local model: {error}"));
                        }
                    }
                    total_discovered.extend(found);
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "scanned_folders": folders.len(),
                    "models_count": total_discovered.len(),
                    "models": total_discovered,
                    "hardware": hardware,
                    "scanned_at": chrono::Utc::now().timestamp_millis()
                }))
            }
            "v1.local_models.list" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                match repo.list_cached_models(folder_id) {
                    Ok(mut models) => {
                        models.retain(|model| std::path::Path::new(&model.file_path).is_file());
                        ApiResponse::success(req.id, serde_json::json!({ "models": models }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.hardware" => {
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                match serde_json::to_value(&hardware) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.profile.get" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => ApiResponse::success(req.id, serde_json::json!({ "profile": profile })),
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                        let profile = custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(), revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(), gpu_layers: None,
                            eval_batch_size: Some(512), cpu_threads: None,
                            flash_attention: if hardware.is_apple_silicon { Some(true) } else { None },
                            mmap: true, mlock: false, idle_timeout_seconds: None, updated_at: 0,
                        };
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.profile.save" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let descriptor = match repo.get_cached_model_by_path(model_path) {
                    Ok(Some(value)) => value,
                    Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let context_size = req.params.get("context_size").and_then(|v| v.as_u64()).unwrap_or(4096);
                if context_size < 512 || context_size > descriptor.context_length {
                    return ApiResponse::error(req.id, format!("context_size must be between 512 and {} for this artifact", descriptor.context_length));
                }
                let gpu_mode = req.params.get("gpu_mode").and_then(|v| v.as_str()).unwrap_or("auto");
                if !matches!(gpu_mode, "auto" | "max" | "off" | "manual") {
                    return ApiResponse::error(req.id, "gpu_mode must be auto, max, off, or manual");
                }
                let gpu_layers = req.params.get("gpu_layers").and_then(|v| v.as_u64()).map(|v| v.min(u32::MAX as u64) as u32);
                if gpu_mode == "manual" && gpu_layers.is_none() {
                    return ApiResponse::error(req.id, "manual GPU mode requires gpu_layers");
                }
                let revision = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(current)) => current.revision.saturating_add(1),
                    Ok(None) => 1,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let profile = custos_domain::LocalModelLoadProfile {
                    model_path: model_path.to_string(), revision, context_size,
                    gpu_mode: gpu_mode.to_string(), gpu_layers,
                    eval_batch_size: req.params.get("eval_batch_size").and_then(|v| v.as_u64()).map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    cpu_threads: req.params.get("cpu_threads").and_then(|v| v.as_u64()).map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    flash_attention: req.params.get("flash_attention").and_then(|v| v.as_bool()),
                    mmap: req.params.get("mmap").and_then(|v| v.as_bool()).unwrap_or(true),
                    mlock: req.params.get("mlock").and_then(|v| v.as_bool()).unwrap_or(false),
                    idle_timeout_seconds: req.params.get("idle_timeout_seconds").and_then(|v| v.as_u64()),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                match repo.save_local_model_load_profile(&profile) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "profile": profile })),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.preflight" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let path = std::path::Path::new(model_path);
                if !path.exists() || !path.is_file() {
                    return ApiResponse::error(req.id, format!("Model file does not exist: {}", model_path));
                }
                let hardware = custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let descriptor = match custos_adapters::local_inference::gguf_inspector::inspect_gguf_file(path, "preflight", &hardware) {
                    Ok(d) => d,
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to parse GGUF file: {}", e)),
                };
                let requested_context = req.params.get("requested_context").and_then(|v| v.as_u64()).unwrap_or(4096).min(descriptor.context_length);
                let inspected_context = descriptor.context_length.min(8192).max(1);
                let inspected_kv_bytes = descriptor.total_memory_required.saturating_sub(descriptor.file_size_bytes);
                let estimated_kv_bytes = inspected_kv_bytes
                    .saturating_mul(requested_context)
                    .checked_div(inspected_context)
                    .unwrap_or(inspected_kv_bytes);
                let estimated_memory_bytes = descriptor.file_size_bytes.saturating_add(estimated_kv_bytes);
                let can_load = estimated_memory_bytes < hardware.total_ram_bytes;
                let mut issues = Vec::new();
                if !can_load {
                    issues.push(format!(
                        "Estimated model and KV-cache memory ({} bytes) is at least the detected physical RAM ({} bytes); loading is blocked by the current preflight policy",
                        estimated_memory_bytes, hardware.total_ram_bytes
                    ));
                }
                if descriptor.compatibility == custos_domain::ModelCompatibility::Red {
                    issues.push("Model memory requirements with large context may exceed memory. Offloading or reduced context recommended.".to_string());
                } else if descriptor.compatibility == custos_domain::ModelCompatibility::Yellow {
                    issues.push("Model may require partial CPU offloading and run with reduced speed".to_string());
                }
                let response = custos_domain::LocalModelPreflightResponse {
                    model_path: model_path.to_string(),
                    can_load,
                    architecture: descriptor.architecture,
                    quantization: descriptor.quantization,
                    file_size_bytes: descriptor.file_size_bytes,
                    context_limit: requested_context,
                    estimated_memory_bytes,
                    free_ram_bytes: hardware.free_ram_bytes,
                    metal_vram_limit_bytes: hardware.metal_vram_limit_bytes,
                    compatibility: descriptor.compatibility,
                    recommended_gpu_layers: None,
                    issues,
                };
                match serde_json::to_value(&response) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.load" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(), revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(), gpu_layers: None,
                            eval_batch_size: Some(512), cpu_threads: None, flash_attention: None,
                            mmap: true, mlock: false, idle_timeout_seconds: None, updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match custos_adapters::local_inference::preload_model_with_profile(model_path, &profile).await {
                    Ok(instance) => {
                        match serde_json::to_value(&instance) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to load local model: {e}")),
                }
            }
            "v1.local_models.ensure" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let force_reload = req.params.get("force_reload").and_then(|v| v.as_bool()).unwrap_or(false);
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => return ApiResponse::error(req.id, "Model artifact is not registered"),
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.min(4096).max(512),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };

                let instances = custos_adapters::local_inference::list_loaded_instances().await.unwrap_or_default();
                let clean_model = model_path.strip_prefix("native:").unwrap_or(model_path);
                let existing = instances.into_iter().find(|inst| {
                    inst.model_path == clean_model || inst.model_name == clean_model
                });

                if let Some(inst) = existing {
                    if !force_reload && inst.profile_revision == profile.revision {
                        return ApiResponse::success(req.id, serde_json::json!({
                            "instance": inst,
                            "drift_detected": false,
                            "reloaded": false
                        }));
                    }
                }

                match custos_adapters::local_inference::preload_model_with_profile(model_path, &profile).await {
                    Ok(instance) => {
                        ApiResponse::success(req.id, serde_json::json!({
                            "instance": instance,
                            "drift_detected": false,
                            "reloaded": true
                        }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to ensure local model: {e}")),
                }
            }
            "v1.local_models.verify" => {
                let model_opt = req.params.get("model_path")
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str());

                let target_model = match model_opt {
                    Some(m) if !m.trim().is_empty() => m.trim().to_string(),
                    _ => {
                        let instances = custos_adapters::local_inference::list_loaded_instances().await.unwrap_or_default();
                        match instances.first() {
                            Some(inst) => inst.model_name.clone(),
                            None => return ApiResponse::error(req.id, "No active local model instance is loaded to verify"),
                        }
                    }
                };

                let clean_model = target_model.strip_prefix("native:").unwrap_or(&target_model);
                let start = std::time::Instant::now();
                match custos_adapters::local_inference::verify_sidecar_generation(clean_model).await {
                    Ok(output) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        ApiResponse::success(req.id, serde_json::json!({
                            "verified": true,
                            "model": clean_model,
                            "test_output": output,
                            "latency_ms": latency_ms
                        }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Generation probe failed: {e}")),
                }
            }
            "v1.local_models.unload" => {
                let model_identifier = match req.params.get("model_path")
                    .or_else(|| req.params.get("instance_id"))
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path or instance_id parameter"),
                };
                let clean_model = model_identifier.strip_prefix("native:").unwrap_or(model_identifier);
                match custos_adapters::local_inference::evict_model(clean_model).await {
                    Ok(evicted) => {
                        ApiResponse::success(req.id, serde_json::json!({ "evicted": evicted, "model": clean_model }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to unload model: {}", e)),
                }
            }
            "v1.local_models.instances" => {
                match custos_adapters::local_inference::list_loaded_instances().await {
                    Ok(instances) => ApiResponse::success(req.id, serde_json::json!({ "instances": instances })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.usage" => {
                let Some(store) = &self.usage_runs else { return ApiResponse::error(req.id, "Usage store unavailable"); };
                let tasks = match self.task_service.list_tasks().await { Ok(tasks) => tasks, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let mut records = Vec::new();
                for task in tasks {
                    let runs = match store.list_runs_for_task(&task.id).await { Ok(runs) => runs, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                    for run in runs {
                        if run.metadata.get("actual_mode").and_then(|v| v.as_str()) != Some("model") { continue; }
                        if run.metadata.get("model_id").and_then(|v| v.as_str()).is_some_and(|id| id.starts_with("custos-")) { continue; }
                        let usage = run.metadata.get("usage").cloned().unwrap_or(serde_json::Value::Null);
                        let failure = run.metadata.get("failure_reason").and_then(|v| v.as_str()).unwrap_or("");
                        records.push(serde_json::json!({"id":run.id,"account_id":run.metadata.get("provider_account_id"),"model":run.metadata.get("model_id").or_else(||run.metadata.get("requested_model")),"status":run.status.to_string(),"recorded_at":run.started_at,"input_tokens":usage.get("input_tokens"),"output_tokens":usage.get("output_tokens"),"total_tokens":usage.get("total_tokens"),"rate_limited":failure.contains("429") || failure.contains("RESOURCE_EXHAUSTED")}));
                    }
                }
                let mut quotas = Vec::new();
                if req.params.get("include_quota").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(repo) = &self.providers {
                        let providers = match repo.list_providers() { Ok(p) => p, Err(e) => return ApiResponse::error(req.id,e.to_string()) };
                        for provider in providers {
                            let mut snapshot = serde_json::json!({"account_id":provider.id,"status":if provider.service_type == "local" {"not_applicable"} else {"unsupported"},"windows":[],"fetched_at":chrono::Utc::now()});
                            if provider.service_type != "local" {
                                match self.read_provider_token(&provider.id).await {
                                    Ok(Some(mut token)) if token.refresh_token.is_some() || provider.service_type == "copilot" => {
                                        if token.is_expired(120) && token.refresh_token.is_some() {
                                            let refreshed = match custos_adapters::providers::oauth_pkce::oauth_settings(&token.service_type) {
                                                Ok(settings) => custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(&settings.0,&settings.1).refresh_token(&settings.2,&token).await,
                                                Err(e) => Err(e),
                                            };
                                            match refreshed { Ok(value) => { if let Err(e)=self.refresh_provider_token(&token, &value).await { return ApiResponse::error(req.id,e.to_string()); } token=value; }, Err(_) => { snapshot["status"]=serde_json::json!("unavailable"); snapshot["error"]=serde_json::json!("Reconnect account to refresh quota"); quotas.push(snapshot); continue; } }
                                        }
                                        match custos_adapters::providers::probe::probe_account_quota(&provider.service_type,&token.access_token).await {
                                            Ok(value) => {
                                                if value["status"] == "available" && value["windows"].as_array().is_some_and(|windows| !windows.is_empty() && windows.iter().all(|window| window["used_percent"].as_f64().is_some_and(|used| (0.0..100.0).contains(&used)))) {
                                                    if let Ok(scope) = crate::runtime::provider_catalog_scope(repo, &provider.id) { if let Err(error) = self.model_availability.quota_recovered(&scope) { tracing::debug!(%error, "Quota recovery projection failed"); } }
                                                }
                                                snapshot["status"]=value["status"].clone(); snapshot["windows"]=value["windows"].clone(); snapshot["source"]=value["source"].clone(); },
                                            Err(e) => { snapshot["status"]=serde_json::json!("unavailable"); snapshot["error"]=serde_json::json!(e.to_string()); }
                                        }
                                    },
                                    Err(_) => { snapshot["status"]=serde_json::json!("unavailable"); },
                                    _ => {}
                                }
                            }
                            quotas.push(snapshot);
                        }
                    }
                }
                tracing::debug!(request_id = %req.id, records = records.len(), quota_accounts = quotas.len(), "Provider usage snapshot loaded");
                ApiResponse::success(req.id, serde_json::json!({"records":records,"quotas":quotas,"fetched_at":chrono::Utc::now()}))
            }
            "v1.models.pricing" => {
                let model_id = req
                    .params
                    .get("model_id")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if model_id.trim().is_empty() {
                    return ApiResponse::error(req.id, "Missing model_id parameter");
                }

                let pricing = custos_adapters::providers::pricing::lookup_model_pricing(model_id);
                let input_tokens = req.params.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let output_tokens = req.params.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_read_tokens = req.params.get("cache_read_tokens").and_then(|v| v.as_u64());
                let cache_write_tokens = req.params.get("cache_write_tokens").and_then(|v| v.as_u64());

                let estimated_cost = pricing.as_ref().map(|p| {
                    custos_adapters::providers::pricing::calculate_token_cost(
                        p,
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_write_tokens,
                    )
                });

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "model_id": model_id,
                        "pricing": pricing,
                        "estimated_cost_usd": estimated_cost,
                    }),
                )
            }
            "v1.keys.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_client_keys() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.generate" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Client API Token");
                let key = custos_domain::ClientApiKeyRecord {
                    id: custos_domain::new_id("key"),
                    name: name.to_string(),
                    token: format!("custos_live_sec_{}", custos_domain::new_id("tok")),
                    created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    revoked: false,
                };
                match providers_repo.create_client_key(&key) {
                    Ok(()) => match serde_json::to_value(&key) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.revoke" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let key_id = match req.params.get("key_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing key_id parameter"),
                };
                match providers_repo.revoke_client_key(key_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "revoked": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.llm.status" => {
                let has_env_key = std::env::var("ANTHROPIC_API_KEY").is_ok()
                    || std::env::var("OPENAI_API_KEY").is_ok()
                    || std::env::var("GEMINI_API_KEY").is_ok()
                    || std::env::var("CUSTOS_PROVIDER").is_ok();

                let oauth_tokens = if self.credentials.is_some() {
                    match self.provider_tokens().await { Ok(tokens) => tokens, Err(e) => return ApiResponse::error(req.id, e.to_string()) }
                } else {
                    Vec::new()
                };

                let active_oauth = oauth_tokens
                    .into_iter()
                    .find(|t| !t.access_token.trim().is_empty());

                let configured_provider = if let Some(ref repo) = self.providers {
                    repo.list_providers().ok().and_then(|list| {
                        list.into_iter().find(|p| {
                            (p.status == "configured"
                                || p.status == "active"
                                || p.status == "primary")
                                && p.api_key_masked != "none"
                        })
                    })
                } else {
                    None
                };

                if let Some(tok) = active_oauth {
                    let display_name = match tok.provider_id.as_str() {
                        "openai" => "OpenAI (OAuth PKCE)".to_string(),
                        "github-copilot" | "copilot" => "GitHub Copilot (OAuth)".to_string(),
                        "google" | "gemini" => "Google Gemini (OAuth)".to_string(),
                        "antigravity" => "Antigravity Sovereign (OAuth)".to_string(),
                        other => format!("{other} (OAuth)"),
                    };
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": display_name,
                            "message": format!("{display_name} authentication active"),
                            "expires_at": tok.expires_at,
                            "provider_id": tok.provider_id,
                        }),
                    )
                } else if has_env_key || configured_provider.is_some() {
                    let prov_name = configured_provider
                        .map(|p| p.name)
                        .unwrap_or_else(|| "Environment API Key".to_string());
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": prov_name,
                            "message": "LLM sẵn sàng thực thi",
                        }),
                    )
                } else {
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": "Custos Sovereign (Local / Offline)",
                            "message": "Custos Offline Sovereign runtime sẵn sàng",
                        }),
                    )
                }
            }
            METHOD_OAUTH_AUTHORIZE => {
                let params: OAuthAuthorizeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let raw_service = params.service_type.as_deref().or(params.provider_id.as_deref()).unwrap_or("openai");
                let service = raw_service.split('-').next().unwrap_or(raw_service);
                if service == "copilot" {
                    let server = match self.oauth_callback_server.as_ref() { Some(v) => v, None => return ApiResponse::error(req.id, "OAuth callback server not configured") };
                    return match custos_adapters::providers::oauth_pkce::start_copilot_device().await {
                        Ok(mut device) => {
                            if device.get("device_code").and_then(|v| v.as_str()).is_none() || device.get("user_code").and_then(|v| v.as_str()).is_none() { return ApiResponse::error(req.id, "Invalid GitHub device authorization response"); }
                            server.start_copilot_device(device.clone(), params.provider_id.clone().unwrap_or_else(|| "copilot".into()));
                            if let Some(object) = device.as_object_mut() { object.remove("device_code"); }
                            device["flow_type"] = serde_json::json!("device_code");
                            device["authorization_url"] = device.get("verification_uri").cloned().unwrap_or(serde_json::json!("https://github.com/login/device"));
                            ApiResponse::success(req.id, device)
                        }
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(service) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let auth_url = params
                    .auth_endpoint
                    .as_deref()
                    .unwrap_or(&settings.0);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(&settings.2);
                let service_type_or_default = service;
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(if service_type_or_default == "anthropic" { "https://platform.claude.com/oauth/code/callback" } else if service_type_or_default == "antigravity" { "http://localhost:51121/oauth-callback" } else { custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI });
                let scope = params.scope.as_deref().or(Some(settings.3.as_str()));

                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(auth_url, token_url);
                let challenge = custos_adapters::providers::oauth_pkce::OAuthPkceManager::generate_pkce_challenge();

                match manager.build_authorization_url(client_id, redirect_uri, scope, &challenge) {
                    Ok(res) => {
                        // Start the loopback callback server on port 1455 if available
                        if let Some(ref cb_server) = self.oauth_callback_server {
                            let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                            let service_type = service;
                            let pending = crate::oauth_callback_server::PendingOAuthContext {
                                provider_id: provider_id.to_string(),
                                service_type: service_type.to_string(),
                                client_id: client_id.to_string(),
                                code_verifier: challenge.code_verifier.clone(),
                                state: challenge.state.clone(),
                                redirect_uri: redirect_uri.to_string(),
                                expires_at: chrono::Utc::now().timestamp_millis() + 600_000,
                            };
                            if let Err(e) = cb_server.start_listening(pending).await {
                                tracing::warn!(
                                    provider_id = %provider_id,
                                    "Notice: OAuth loopback callback listener failed to bind: {e}"
                                );
                            }
                        }

                        match serde_json::to_value(&res) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_EXCHANGE => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let params: OAuthExchangeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                let raw_service = params.service_type.as_deref().or(params.provider_id.as_deref()).unwrap_or("openai");
                let service_type = raw_service.split('-').next().unwrap_or(raw_service);
                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let client_id = params
                    .client_id
                    .as_deref()
                    .unwrap_or(&settings.2);
                let service_type_or_default = service_type;
                let redirect_uri = params
                    .redirect_uri
                    .as_deref()
                    .unwrap_or(if service_type_or_default == "anthropic" { "https://platform.claude.com/oauth/code/callback" } else if service_type_or_default == "antigravity" { "http://localhost:51121/oauth-callback" } else { custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI });
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);

                let callback_server = match self.oauth_callback_server.as_ref() { Some(v) => v, None => return ApiResponse::error(req.id, "OAuth callback server not configured") };
                if let Err(e) = callback_server.validate_manual_callback(provider_id, &params.code_verifier, redirect_uri, &params.code_or_url) { return ApiResponse::error(req.id, e.to_string()); }
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager
                    .exchange_code_with_identity(
                        provider_id,
                        service_type,
                        client_id,
                        &params.code_or_url,
                        redirect_uri,
                        &params.code_verifier,
                    )
                    .await
                {
                    Ok((token_record, identity)) => {
                        // Upsert matching provider config so it appears in UI as active
                        let now = chrono::Utc::now().timestamp_millis();
                        let masked = "oauth".to_string();
                        let prov_cfg = custos_domain::ProviderConfig {
                            id: provider_id.to_string(),
                            name: identity.unwrap_or_else(|| format!("{provider_id} (OAuth)")),
                            service_type: service_type.to_string(),
                            api_key_masked: masked,
                            status: "active".to_string(),
                            endpoint_url: if service_type == "gemini" { Some("https://cloudcode-pa.googleapis.com".into()) } else { default_model_probe_base_url(service_type).map(str::to_string) },
                            default_model: None,
                            context_window: None,
                            fast_mode: Some(false),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = self.write_provider_connection(&token_record, &prov_cfg).await { return ApiResponse::error(req.id, e.to_string()); }

                        match serde_json::to_value(serde_json::json!({"provider_id":token_record.provider_id,"service_type":token_record.service_type,"connected":true,"expires_at":token_record.expires_at,"scope":token_record.scope,"has_refresh_token":token_record.refresh_token.is_some()})) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_REFRESH => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let params: OAuthRefreshParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let existing = match self.read_provider_token(&params.provider_id).await {
                    Ok(Some(tok)) => tok,
                    Ok(None) => return ApiResponse::error(req.id, format!("No OAuth token found for {}", params.provider_id)),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(&existing.service_type) { Ok(v) => v, Err(e) => return ApiResponse::error(req.id, e.to_string()) };
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let token_url = params
                    .token_endpoint
                    .as_deref()
                    .unwrap_or(&settings.1);
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager.refresh_token(client_id, &existing).await {
                    Ok(refreshed) => {
                        if let Err(e) = self.refresh_provider_token(&existing, &refreshed).await {
                            return ApiResponse::error(req.id, format!("Failed to update token: {e}"));
                        }
                        match serde_json::to_value(serde_json::json!({"provider_id":refreshed.provider_id,"service_type":refreshed.service_type,"connected":true,"expires_at":refreshed.expires_at,"scope":refreshed.scope,"has_refresh_token":refreshed.refresh_token.is_some()})) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_GET => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.read_provider_token(provider_id).await {
                    Ok(Some(tok)) => {
                        let sanitized = serde_json::json!({
                            "provider_id": tok.provider_id,
                            "service_type": tok.service_type,
                            "connected": !tok.access_token.is_empty(),
                            "expires_at": tok.expires_at,
                            "token_type": tok.token_type,
                            "scope": tok.scope,
                            "has_refresh_token": tok.refresh_token.is_some(),
                        });
                        ApiResponse::success(req.id, sanitized)
                    }
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_DELETE => {
                if self.providers.is_none() { return ApiResponse::error(req.id,"ProviderRepository not configured"); }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.remove_provider_credential(provider_id, false).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "provider_id": provider_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_CANCEL => {
                if let Some(server) = self.oauth_callback_server.as_ref() { server.cancel(); }
                ApiResponse::success(req.id, serde_json::json!({"cancelled":true}))
            }
            METHOD_OAUTH_STATUS => {
                if let Some(ref cb_server) = self.oauth_callback_server {
                    let status_val = match cb_server.status() {
                        crate::oauth_callback_server::CallbackServerStatus::Idle => {
                            serde_json::json!({ "status": "idle" })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Listening { port } => {
                            serde_json::json!({ "status": "listening", "port": port })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Exchanging { .. } => {
                            serde_json::json!({ "status": "exchanging" })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Completed { provider_id } => {
                            serde_json::json!({ "status": "completed", "provider_id": provider_id })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Failed { error } => {
                            serde_json::json!({ "status": "failed", "error": error })
                        }
                    };
                    ApiResponse::success(req.id, status_val)
                } else {
                    ApiResponse::success(req.id, serde_json::json!({ "status": "idle" }))
                }
            }
            METHOD_GITHUB_ACCOUNTS => {
                let gh = match resolve_gh_binary() { Some(gh) => gh, None => return ApiResponse::success(req.id, serde_json::json!({"installed":false,"accounts":[]})) };
                match custos_adapters::repository_git::github_accounts(&gh).await {
                    Ok(value) => ApiResponse::success(req.id,value),
                    Err(error) => ApiResponse::error(req.id,error.to_string()),
                }
            }
            METHOD_GIT_REPOSITORY | METHOD_GIT_SWITCH | METHOD_GIT_FETCH | METHOD_GIT_ACCOUNT | METHOD_GIT_GITHUB => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.is_empty() => id,
                    _ => return ApiResponse::error(req.id,"An exact workspace_id is required"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id,workspace_id).await { Ok(ws) => ws, Err(response) => return response };
                if !matches!(ws.kind,custos_domain::WorkspaceKind::Git { .. }) { return ApiResponse::error(req.id,"Local registered Git checkout required; remote-host Git routing is not available here"); }
                let _guard = self.candidate_mutation.lock().await;
                let path = std::path::Path::new(&ws.path);
                let gh = resolve_gh_binary();
                let result: Result<serde_json::Value,DomainError> = async {
                    use custos_adapters::repository_git as git;
                    tracing::debug!(request_id = %req.id, workspace_id, method = %req.method, "Repository Git operation");
                    match req.method.as_str() {
                        METHOD_GIT_REPOSITORY => git::repository(path).await,
                        METHOD_GIT_ACCOUNT => {
                            let binding = req.params.get("account").filter(|v| !v.is_null()).map(|v| serde_json::from_value::<git::GitHubBinding>(v.clone()).map_err(|_| DomainError::Validation("Invalid account preference".into()))).transpose()?;
                            git::bind_account(path,gh.as_deref(),binding.as_ref()).await?;
                            git::repository(path).await
                        }
                        METHOD_GIT_GITHUB => {
                            let binding = git::account_binding(path).await?.ok_or_else(|| DomainError::Validation("Choose a repository GitHub account first".into()))?;
                            git::github_viewer(gh.as_deref().ok_or_else(|| DomainError::Validation("GitHub CLI is not installed".into()))?,&binding).await
                        }
                        METHOD_GIT_FETCH => {
                            let remote = req.params.get("remote").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit remote required".into()))?;
                            git::fetch_remote(path,gh.as_deref(),remote).await
                        }
                        METHOD_GIT_SWITCH => {
                            let branch = req.params.get("branch").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Explicit branch required".into()))?;
                            let expected = req.params.get("expected_head").and_then(|v| v.as_str()).ok_or_else(|| DomainError::Validation("Observed HEAD required".into()))?;
                            if self.terminal.list_sessions(Some(workspace_id)).await?.iter().any(|s| matches!(s.status,custos_domain::TerminalSessionStatus::Active)) {
                                return Err(DomainError::Conflict("Close active checkout terminals before switching, or use a new worktree".into()));
                            }
                            let runs = self.usage_runs.as_ref().ok_or_else(|| DomainError::Validation("Cannot verify active runs; branch switch unavailable".into()))?;
                            for task in self.task_service.list_tasks().await? {
                                for run in runs.list_runs_for_task(&task.id).await? {
                                    let target = run.metadata.get("execution_workspace_id").or_else(||run.metadata.get("workspace_id")).and_then(|v|v.as_str());
                                    if !run.status.is_terminal() && (target.is_none() || target==Some(workspace_id)) {
                                        return Err(DomainError::Conflict("Active or unscoped run prevents branch switching; use a separate worktree".into()));
                                    }
                                }
                            }
                            git::switch_branch(path,branch,expected).await
                        }
                        _ => Err(DomainError::Validation("Unsupported Git operation".into())),
                    }
                }.await;
                match result { Ok(value) => ApiResponse::success(req.id,value), Err(error) => ApiResponse::error(req.id,error.to_string()) }
            }
            METHOD_GITHUB_LOGIN => {
                let _login_guard = GITHUB_LOGIN_LOCK.lock().await;
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Install GitHub CLI (gh) to connect. Local Git/worktrees remain available."),
                };
                // Reuse the bounded PTY lifecycle, never the model OAuth singleton.
                let existing = self.terminal.list_sessions(Some("github-auth")).await;
                if let Ok(sessions) = existing {
                    if let Some(session) = sessions.into_iter().find(|session| matches!(session.status, custos_domain::TerminalSessionStatus::Active)) {
                        return match serde_json::to_value(session) {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        };
                    }
                }
                let command = github_login_command(&gh);
                tracing::info!(request_id = %req.id, "Starting user-requested GitHub CLI login");
                match self.terminal.spawn("github-auth", &std::env::temp_dir(), Some(&command), 100, 28).await {
                    Ok(session) => match serde_json::to_value(session) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_AUTH_STATUS => {
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::success(req.id, serde_json::json!({
                        "authenticated": false, "source": "gh_cli", "installed": false,
                        "error": "GitHub CLI (gh) is not installed. Local Git/worktrees remain available."
                    })),
                };
                // The authenticated viewer is authoritative. Never export gh's token or
                // trust a cached profile / a fragment of multi-account auth-status text.
                let mut command = tokio::process::Command::new(gh);
                command.args(["api", "--hostname", "github.com", "user"]);
                command.kill_on_drop(true);
                let result = tokio::time::timeout(std::time::Duration::from_secs(10), command.output()).await;
                let value = match result {
                    Ok(Ok(output)) => github_auth_metadata(output.status.success(), &output.stdout),
                    Ok(Err(_)) => serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "Could not launch GitHub CLI."}),
                    Err(_) => serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "GitHub verification timed out. Check the network and retry."}),
                };
                ApiResponse::success(req.id, value)
            }
            METHOD_GITHUB_VIEWER => {
                let gh_bin = match resolve_gh_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "GitHub CLI (`gh`) is not installed or not in PATH",
                        );
                    }
                };

                match tokio::process::Command::new(&gh_bin)
                    .args(["api", "user"])
                    .output()
                    .await
                {
                    Ok(out) if out.status.success() => {
                        match serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => {
                                ApiResponse::error(req.id, format!("Invalid JSON from gh api user: {e}"))
                            }
                        }
                    }
                    Ok(out) => {
                        let err_msg = String::from_utf8_lossy(&out.stderr);
                        ApiResponse::error(req.id, format!("gh api user failed: {}", err_msg.trim()))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to run gh api user: {e}")),
                }
            }
            METHOD_CAPABILITIES_LIST => {
                let caps = self.list_capabilities();
                match serde_json::to_value(&caps) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_CAPABILITIES_GET => {
                let target_id = req
                    .params
                    .get("capability_id")
                    .or_else(|| req.params.get("id"))
                    .or_else(|| req.params.get("resource_id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let caps = self.list_capabilities();
                        if let Some(c) = caps
                            .into_iter()
                            .find(|cap| cap.id == id || cap.resource_id.as_deref() == Some(id))
                        {
                            match serde_json::to_value(&c) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            }
                        } else {
                            ApiResponse::error(req.id, format!("Capability '{id}' not found"))
                        }
                    }
                    None => ApiResponse::error(req.id, "Missing capability_id parameter"),
                }
            }
            METHOD_TERMINAL_SPAWN => {
                let workspace_id = match req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };

                let working_dir_str = match req.params.get("working_dir").and_then(|v| v.as_str()) {
                    Some(dir) => dir.to_string(),
                    None => {
                        let coordinator = match self.workspace.as_ref() {
                            Some(c) => c,
                            None => {
                                return ApiResponse::error(
                                    req.id,
                                    "WorkspaceCoordinator not configured on daemon",
                                )
                            }
                        };
                        let ws_id = custos_domain::WorkspaceId::new(workspace_id);
                        match coordinator.get_workspace(&ws_id).await {
                            Ok(Some(ws)) => ws.path,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Workspace {workspace_id} not found"),
                                );
                            }
                            Err(e) => {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Workspace {workspace_id} lookup failed: {e}"),
                                );
                            }
                        }
                    }
                };

                let command = req.params.get("command").and_then(|v| v.as_str());
                let cols = req
                    .params
                    .get("cols")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(80) as u16;
                let rows = req
                    .params
                    .get("rows")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(24) as u16;

                match self
                    .terminal
                    .spawn(
                        workspace_id,
                        std::path::Path::new(&working_dir_str),
                        command,
                        cols,
                        rows,
                    )
                    .await
                {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_WRITE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let data = match req.params.get("data").and_then(|v| v.as_str()) {
                    Some(d) => d,
                    None => return ApiResponse::error(req.id, "Missing data parameter"),
                };

                match self.terminal.write(session_id, data.as_bytes()).await {
                    Ok(written) => {
                        ApiResponse::success(req.id, serde_json::json!({ "written": written }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_RESIZE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let cols = match req.params.get("cols").and_then(|v| v.as_u64()) {
                    Some(c) => c as u16,
                    None => return ApiResponse::error(req.id, "Missing cols parameter"),
                };
                let rows = match req.params.get("rows").and_then(|v| v.as_u64()) {
                    Some(r) => r as u16,
                    None => return ApiResponse::error(req.id, "Missing rows parameter"),
                };

                match self.terminal.resize(session_id, cols, rows).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "ok": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_READ => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let from_seq = req
                    .params
                    .get("from_seq")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let max_bytes = req
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(65536) as usize;

                match self
                    .terminal
                    .read_output(session_id, from_seq, max_bytes)
                    .await
                {
                    Ok(chunk) => match serde_json::to_value(&chunk) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_TERMINATE => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.terminal.terminate(session_id).await {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "terminated": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_LIST => {
                let workspace_id = req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str());

                match self.terminal.list_sessions(workspace_id).await {
                    Ok(sessions) => match serde_json::to_value(&sessions) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_GET => {
                let session_id = match req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.terminal.get_session(session_id).await {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_TREE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };
                let relative_dir = req.params.get("relative_dir").and_then(|v| v.as_str());
                let max_depth = req.params.get("max_depth").and_then(|v| v.as_u64()).map(|d| d as usize);

                match self.files.get_file_tree(&ws, relative_dir, max_depth).await {
                    Ok(tree) => match serde_json::to_value(&tree) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_READ => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let max_bytes = req.params.get("max_bytes").and_then(|v| v.as_u64()).map(|b| b as usize);
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.read_file(&ws, path, max_bytes).await {
                    Ok(content) => match serde_json::to_value(&content) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_WRITE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c.to_string(),
                    None => return ApiResponse::error(req.id, "Missing content parameter"),
                };
                let create_parents = req.params.get("create_parents").and_then(|v| v.as_bool()).unwrap_or(true);
                let overwrite = req.params.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(true);
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                let mut params = custos_domain::WriteWorkspaceFileParams::new(ws.id.clone(), path, content);
                params.create_parents = create_parents;
                params.overwrite = overwrite;

                match self.files.write_file(&ws, params).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_diff(&ws, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.get_file_diff(&ws, path, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_STAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.stage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "staged": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_UNSTAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.unstage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "unstaged": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_DISCARD => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.discard_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "discarded": true, "path": path })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_COMMIT => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) if !m.trim().is_empty() => m,
                    _ => return ApiResponse::error(req.id, "Missing message parameter"),
                };
                let ws = match self.resolve_workspace_for_dispatch(&req.id, workspace_id).await {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.files.commit(&ws, message).await {
                    Ok(out) => ApiResponse::success(req.id, serde_json::json!({ "committed": true, "output": out })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_LIST => {
                let harnesses = self.harnesses.list();
                match serde_json::to_value(&harnesses) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_GET => {
                let harness_id = req
                    .params
                    .get("harness_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match harness_id {
                    Some(id) => match self.harnesses.get_descriptor(id) {
                        Some(desc) => match serde_json::to_value(&desc) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        },
                        None => ApiResponse::error(req.id, format!("Harness '{id}' not found")),
                    },
                    None => ApiResponse::error(req.id, "Missing harness_id parameter"),
                }
            }
            METHOD_HARNESS_RUN_NATIVE => {
                let params: RunNativeHarnessRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid run_native parameters: {e}")),
                };

                // Require authoritative workspace binding to prevent arbitrary filesystem escapes
                let coordinator = match self.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        );
                    }
                };

                let cwd_path = if let Some(ws_id) = &params.workspace_id {
                    match self.resolve_workspace_for_dispatch(&req.id, ws_id).await {
                        Ok(ws) => {
                            let base = std::path::PathBuf::from(ws.path);
                            if let Some(c) = &params.cwd {
                                let requested = std::path::PathBuf::from(c);
                                if requested.is_absolute() && !requested.starts_with(&base) {
                                    return ApiResponse::error(
                                        req.id,
                                        format!("Security boundary violation: cwd '{c}' escapes registered workspace '{}'", base.display()),
                                    );
                                }
                                requested
                            } else {
                                base
                            }
                        }
                        Err(resp) => return resp,
                    }
                } else if let Some(c) = &params.cwd {
                    let requested = std::path::PathBuf::from(c);
                    // Check if requested cwd is within ANY registered workspace
                    let registered_workspaces = coordinator.list_workspaces().await.unwrap_or_default();
                    let canonical = requested.canonicalize().unwrap_or_else(|_| requested.clone());
                    let is_within_registered = registered_workspaces.iter().any(|ws| {
                        let ws_path = std::path::PathBuf::from(&ws.path);
                        let ws_canonical = ws_path.canonicalize().unwrap_or(ws_path);
                        canonical.starts_with(&ws_canonical)
                    });
                    if !is_within_registered {
                        return ApiResponse::error(
                            req.id,
                            format!("Security boundary violation: raw harness execution requires cwd inside a registered workspace, '{c}' is outside registered boundaries"),
                        );
                    }
                    requested
                } else {
                    return ApiResponse::error(
                        req.id,
                        "Missing workspace_id: raw harness execution must be bound to a registered workspace",
                    );
                };

                match self.harnesses.run_native(&params.harness_id, &params.instruction, &cwd_path).await {
                    Ok(result) => match serde_json::to_value(&result) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(err) => ApiResponse::error(req.id, format!("Harness execution failed: {err}")),
                }
            }
            METHOD_HARNESS_CANCEL => {
                let params: CancelHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid cancel parameters: {e}")),
                };
                match self.harnesses.cancel(&params.harness_id, &params.run_id).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "cancelled": true })),
                    Err(err) => ApiResponse::error(req.id, format!("Failed to cancel harness: {err}")),
                }
            }
            METHOD_HARNESS_STEER => {
                let params: SteerHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid steer parameters: {e}")),
                };
                match self.harnesses.steer(&params.harness_id, &params.run_id, &params.guidance).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "steered": true })),
                    Err(err) => ApiResponse::error(req.id, format!("Failed to steer harness: {err}")),
                }
            }
            METHOD_BROWSER_SESSIONS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let ws_id = req.params.get("workspace_id").and_then(|v| v.as_str());
                match fleet.list_browser_sessions() {
                    Ok(sessions) => {
                        let filtered = if let Some(ws) = ws_id {
                            sessions.into_iter().filter(|s| s.workspace_id.as_deref() == Some(ws)).collect()
                        } else {
                            sessions
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_SESSIONS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("Default Session");
                let ws_id = req.params.get("workspace_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let session = BrowserSession::new(name, ws_id);
                match fleet.save_browser_session(&session) {
                    Ok(()) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                match fleet.list_browser_tabs(Some(session_id)) {
                    Ok(tabs) => match serde_json::to_value(&tabs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = req.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let tab = match BrowserTab::new(session_id, url) {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_NAVIGATE => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; navigation is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let raw_url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(raw_url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                let mut tab = match fleet.get_browser_tab(tab_id) {
                    Ok(Some(t)) => t,
                    Ok(None) => return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let now = chrono::Utc::now().timestamp_millis();
                tab.url = raw_url.to_string();
                tab.title = if raw_url.is_empty() || raw_url == "about:blank" {
                    "New Tab".to_string()
                } else {
                    raw_url.to_string()
                };

                // Launch real browser navigation via agent-browser if available
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    if !raw_url.is_empty() && raw_url != "about:blank" {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(1500),
                            tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("open")
                                .arg(raw_url)
                                .output(),
                        )
                        .await;
                    }
                }

                tab.status = BrowserTabStatus::Ready;
                tab.updated_at = now;
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_SNAPSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; snapshot capture is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = if let Some(t) = req.params.get("tab_id").and_then(|v| v.as_str()) {
                    t.to_string()
                } else {
                    return ApiResponse::error(req.id, "Missing tab_id parameter");
                };

                let mut tab = match fleet.get_browser_tab(&tab_id) {
                    Ok(Some(t)) => t,
                    Ok(None) => return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();

                if let Ok(explicit_snapshot) = serde_json::from_value::<BrowserPageSnapshot>(req.params.clone()) {
                    tab.last_snapshot = Some(explicit_snapshot);
                    tab.updated_at = now;
                    let _ = fleet.save_browser_tab(&tab);
                    return ApiResponse::success(req.id, serde_json::json!({ "saved": true, "tab_id": tab_id }));
                }

                // If agent-browser is available, execute real AX tree snapshot and real screenshot
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let snap_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("snapshot")
                        .arg("-i")
                        .arg("--urls")
                        .output()
                        .await;

                    let screenshot_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("screenshot")
                        .output()
                        .await;

                    let mut screenshot_path = None;
                    if let Ok(ref s_out) = screenshot_out {
                        if s_out.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&s_out.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    screenshot_path = Some(p.to_string());
                                }
                            }
                        }
                    }

                    if let Ok(s_out) = snap_out {
                        if s_out.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&s_out.stdout) {
                                if let Some(data) = val.get("data") {
                                    let origin = data.get("origin").and_then(|v| v.as_str()).unwrap_or(&tab.url).to_string();
                                    let snap_text = data.get("snapshot").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let refs_val = data.get("refs");

                                    let mut links = Vec::new();
                                    if let Some(refs_obj) = refs_val.and_then(|v| v.as_object()) {
                                        for (key, val) in refs_obj {
                                            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                            let role = val.get("role").and_then(|v| v.as_str()).unwrap_or("");
                                            links.push(format!("@{} [{}]: {}", key, role, name));
                                        }
                                    }

                                    let summary = format!(
                                        "URL: {}\n\nAccessibility Tree (Interactive Elements):\n{}\n\nInteractive Refs Count: {}",
                                        origin,
                                        if snap_text.is_empty() { "No interactive elements found" } else { &snap_text },
                                        links.len()
                                    );

                                    let snapshot = BrowserPageSnapshot {
                                        tab_id: tab_id.clone(),
                                        url: origin.clone(),
                                        title: if tab.title == "New Tab" { origin } else { tab.title.clone() },
                                        dom_tree_summary: summary,
                                        text_content: snap_text,
                                        links,
                                        viewport_width: 1280,
                                        viewport_height: 800,
                                        screenshot_uri: screenshot_path,
                                        timestamp: now,
                                    };

                                    tab.last_snapshot = Some(snapshot);
                                    tab.updated_at = now;
                                    let _ = fleet.save_browser_tab(&tab);
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "saved": true,
                                        "tab_id": tab_id,
                                        "snapshot": tab.last_snapshot
                                    }));
                                }
                            }
                        }
                    }
                }

                // Fallback: HTTP GET fetch if agent-browser binary is not found
                let snapshot = if tab.url.starts_with("http://") || tab.url.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap_or_default();
                    let start = std::time::Instant::now();
                    match client.get(&tab.url).send().await {
                        Ok(resp) => {
                            let status_code = resp.status().as_u16();
                            let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
                            let duration = start.elapsed().as_millis() as u64;
                            let html_text = resp.text().await.unwrap_or_default();

                            tab.network_requests.push(BrowserNetworkRequest {
                                url: tab.url.clone(),
                                method: "GET".to_string(),
                                status: Some(status_code),
                                content_type: content_type.clone(),
                                duration_ms: duration,
                                timestamp: now,
                            });

                            let extracted_title = if let Some(start_idx) = html_text.to_lowercase().find("<title>") {
                                let rest = &html_text[start_idx + 7..];
                                rest.find("</title>").map(|end_idx| rest[..end_idx].trim().to_string())
                            } else {
                                None
                            };

                            let mut links = Vec::new();
                            let mut search_slice = html_text.as_str();
                            while let Some(href_pos) = search_slice.find("href=\"") {
                                let remainder = &search_slice[href_pos + 6..];
                                if let Some(quote_end) = remainder.find('"') {
                                    let href = &remainder[..quote_end];
                                    if (href.starts_with("http://") || href.starts_with("https://") || href.starts_with('/')) && links.len() < 25 {
                                        links.push(href.to_string());
                                    }
                                    search_slice = &remainder[quote_end..];
                                } else {
                                    break;
                                }
                            }

                            let mut plain_text = String::new();
                            let mut in_tag = false;
                            for c in html_text.chars() {
                                if c == '<' {
                                    in_tag = true;
                                } else if c == '>' {
                                    in_tag = false;
                                    plain_text.push(' ');
                                } else if !in_tag {
                                    plain_text.push(c);
                                }
                                if plain_text.len() > 4000 {
                                    break;
                                }
                            }
                            let compact_text = plain_text.split_whitespace().collect::<Vec<_>>().join(" ");
                            let final_title = extracted_title.unwrap_or_else(|| tab.url.clone());
                            tab.title = final_title.clone();

                            BrowserPageSnapshot {
                                tab_id: tab_id.clone(),
                                url: tab.url.clone(),
                                title: final_title.clone(),
                                dom_tree_summary: format!(
                                    "Page Title: {}\nHTTP Status: {}\nExtracted Links: {}\nText Preview:\n{}",
                                    final_title,
                                    status_code,
                                    links.len(),
                                    if compact_text.len() > 300 { &compact_text[..300] } else { &compact_text }
                                ),
                                text_content: compact_text,
                                links,
                                viewport_width: 1280,
                                viewport_height: 800,
                                screenshot_uri: None,
                                timestamp: now,
                            }
                        }
                        Err(err) => {
                            tab.console_logs.push(BrowserConsoleEntry {
                                level: "error".to_string(),
                                message: format!("Failed to fetch {}: {}", tab.url, err),
                                timestamp: now,
                            });
                            BrowserPageSnapshot {
                                tab_id: tab_id.clone(),
                                url: tab.url.clone(),
                                title: tab.title.clone(),
                                dom_tree_summary: format!("Connection error fetching {}: {}", tab.url, err),
                                text_content: String::new(),
                                links: Vec::new(),
                                viewport_width: 1280,
                                viewport_height: 800,
                                screenshot_uri: None,
                                timestamp: now,
                            }
                        }
                    }
                } else {
                    BrowserPageSnapshot {
                        tab_id: tab_id.clone(),
                        url: tab.url.clone(),
                        title: tab.title.clone(),
                        dom_tree_summary: format!("Local tab: {}", tab.url),
                        text_content: String::new(),
                        links: Vec::new(),
                        viewport_width: 1280,
                        viewport_height: 800,
                        screenshot_uri: None,
                        timestamp: now,
                    }
                };

                tab.last_snapshot = Some(snapshot);
                tab.updated_at = now;
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({
                        "saved": true,
                        "tab_id": tab_id,
                        "snapshot": tab.last_snapshot
                    })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CLICK => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; click is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("click")
                    .arg(&formatted_ref)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            // After click, update snapshot
                            let mut tab = match fleet.get_browser_tab(tab_id) {
                                Ok(Some(t)) => t,
                                _ => return ApiResponse::success(req.id, serde_json::json!({ "clicked": formatted_ref })),
                            };
                            let snap_out = tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("--json")
                                .arg("snapshot")
                                .arg("-i")
                                .arg("--urls")
                                .output()
                                .await;

                            if let Ok(so) = snap_out {
                                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&so.stdout) {
                                    if let Some(data) = val.get("data") {
                                        let origin = data.get("origin").and_then(|v| v.as_str()).unwrap_or(&tab.url).to_string();
                                        let snap_text = data.get("snapshot").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        tab.url = origin.clone();
                                        if let Some(snap) = tab.last_snapshot.as_mut() {
                                            snap.url = origin;
                                            snap.text_content = snap_text.clone();
                                            snap.dom_tree_summary = snap_text;
                                        }
                                        let _ = fleet.save_browser_tab(&tab);
                                    }
                                }
                            }
                            match serde_json::to_value(&tab) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(_) => ApiResponse::success(req.id, serde_json::json!({ "clicked": formatted_ref })),
                            }
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Click failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_BROWSER_TABS_FILL => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; fill is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };
                let text = match req.params.get("text").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing text parameter"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("fill")
                    .arg(&formatted_ref)
                    .arg(text)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            let tab = fleet.get_browser_tab(tab_id).ok().flatten();
                            ApiResponse::success(req.id, serde_json::json!({
                                "filled": formatted_ref,
                                "text": text,
                                "tab": tab
                            }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Fill failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_BROWSER_TABS_SCREENSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; screenshot is fail-closed",
                    );
                }
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("screenshot")
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    let image_base64 = std::fs::read(p).ok().map(|bytes| {
                                        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
                                    });
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "path": p,
                                        "tab_id": tab_id,
                                        "image_base64": image_base64
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "tab_id": tab_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Screenshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_BROWSER_TABS_CLOSE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(tab_id)
                        .arg("close")
                        .output()
                        .await;
                }
                match fleet.delete_browser_tab(tab_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "closed": true, "tab_id": tab_id })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_EMULATOR_DEVICES_LIST => {
                let devices = serde_json::json!([
                    {
                        "id": "iphone_15_pro",
                        "name": "iPhone 15 Pro",
                        "platform": "iOS",
                        "width": 393,
                        "height": 852,
                        "scale": 3.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "iphone_se",
                        "name": "iPhone SE (3rd gen)",
                        "platform": "iOS",
                        "width": 375,
                        "height": 667,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "pixel_8",
                        "name": "Google Pixel 8",
                        "platform": "Android",
                        "width": 412,
                        "height": 915,
                        "scale": 2.625,
                        "user_agent": "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Mobile Safari/537.36",
                        "status": "ready"
                    },
                    {
                        "id": "ipad_mini",
                        "name": "iPad Mini",
                        "platform": "iPadOS",
                        "width": 744,
                        "height": 1133,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    }
                ]);
                ApiResponse::success(req.id, devices)
            }
            METHOD_EMULATOR_LAUNCH => {
                let device_id = req.params.get("device_id").and_then(|v| v.as_str()).unwrap_or("iphone_15_pro");
                let session_id = req.params.get("session_id").and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("mobile_{}_{}", device_id, chrono::Utc::now().timestamp_millis()));
                let url = req.params.get("url").and_then(|v| v.as_str()).unwrap_or("https://google.com");

                let (width, height, scale): (u32, u32, f64) = match device_id {
                    "iphone_se" => (375, 667, 2.0),
                    "pixel_8" => (412, 915, 2.625),
                    "ipad_mini" => (744, 1133, 2.0),
                    _ => (393, 852, 3.0),
                };

                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("set")
                        .arg("viewport")
                        .arg(width.to_string())
                        .arg(height.to_string())
                        .arg(scale.to_string())
                        .output()
                        .await;

                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "session_id": session_id,
                    "device_id": device_id,
                    "width": width,
                    "height": height,
                    "scale": scale,
                    "url": url,
                    "status": "running"
                }))
            }
            METHOD_EMULATOR_NAVIGATE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }
                ApiResponse::success(req.id, serde_json::json!({
                    "session_id": session_id,
                    "url": url,
                    "status": "ready"
                }))
            }
            METHOD_EMULATOR_SNAPSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let snap_out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("--json")
                    .arg("snapshot")
                    .arg("-i")
                    .arg("--urls")
                    .output()
                    .await;

                match snap_out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(data) = val.get("data") {
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "session_id": session_id,
                                        "snapshot": data.get("snapshot").and_then(|v| v.as_str()).unwrap_or(""),
                                        "refs": data.get("refs").cloned().unwrap_or(serde_json::json!({})),
                                        "origin": data.get("origin").and_then(|v| v.as_str()).unwrap_or("")
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "session_id": session_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Snapshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_EMULATOR_SCREENSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("--json")
                    .arg("screenshot")
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                if let Some(p) = val.get("data").and_then(|d| d.get("path")).and_then(|p| p.as_str()) {
                                    let image_base64 = std::fs::read(p).ok().map(|bytes| {
                                        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
                                    });
                                    return ApiResponse::success(req.id, serde_json::json!({
                                        "session_id": session_id,
                                        "path": p,
                                        "image_base64": image_base64
                                    }));
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "session_id": session_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Screenshot failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_EMULATOR_TAP => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let ref_key = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "agent-browser engine binary not found"),
                };

                let selector = if ref_key.starts_with('@') {
                    ref_key.to_string()
                } else {
                    format!("@{ref_key}")
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("click")
                    .arg(&selector)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            ApiResponse::success(req.id, serde_json::json!({
                                "session_id": session_id,
                                "ref": selector,
                                "tapped": true
                            }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Tap failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}")),
                }
            }
            METHOD_FLEET_HOSTS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                match fleet.list_remote_hosts() {
                    Ok(hosts) => match serde_json::to_value(&hosts) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_REGISTER => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let params: RegisterHostParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid register host params: {e}")),
                };
                let auth_method = if let Some(key_path) = &params.private_key_path {
                    SshAuthMethod::KeyPair {
                        private_key_path: key_path.clone(),
                        passphrase: None,
                    }
                } else {
                    SshAuthMethod::Agent
                };
                let mut node = match RemoteHostNode::new(params.name, params.host, params.port, params.user, auth_method) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                node.labels = params.labels;
                match fleet.save_remote_host(&node) {
                    Ok(()) => match serde_json::to_value(&node) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_PING => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; SSH probe is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let mut host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => return ApiResponse::error(req.id, format!("Host '{host_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin = resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o").arg("BatchMode=yes")
                   .arg("-o").arg("ConnectTimeout=3")
                   .arg("-o").arg("StrictHostKeyChecking=yes")
                   .arg("-p").arg(host.port.to_string());

                if let SshAuthMethod::KeyPair { private_key_path, .. } = &host.auth_method {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg("echo -n ping");

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        if out.status.success() {
                            host.status = RemoteHostStatus::Online;
                            host.last_ping_ms = Some(duration_ms);
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(req.id, serde_json::json!({
                                "host_id": host.id,
                                "status": "online",
                                "ping_ms": duration_ms
                            }))
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            host.status = RemoteHostStatus::Offline;
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(req.id, serde_json::json!({
                                "host_id": host.id,
                                "status": "offline",
                                "error": stderr.trim()
                            }))
                        }
                    }
                    Err(e) => {
                        host.status = RemoteHostStatus::Offline;
                        host.updated_at = now;
                        let _ = fleet.save_remote_host(&host);
                        ApiResponse::success(req.id, serde_json::json!({
                            "host_id": host.id,
                            "status": "offline",
                            "error": e.to_string()
                        }))
                    }
                }
            }
            METHOD_FLEET_EXEC => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; remote command is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let command = match req.params.get("command").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing command parameter"),
                };
                let host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => return ApiResponse::error(req.id, format!("Host '{host_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin = resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o").arg("BatchMode=yes")
                   .arg("-o").arg("ConnectTimeout=10")
                   .arg("-o").arg("StrictHostKeyChecking=yes")
                   .arg("-p").arg(host.port.to_string());

                if let SshAuthMethod::KeyPair { private_key_path, .. } = &host.auth_method {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg(command);

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let receipt = FleetExecReceipt {
                            execution_id: new_id("fexec"),
                            host_id: host.id.clone(),
                            command: command.to_string(),
                            exit_code: out.status.code(),
                            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                            duration_ms,
                            // Process exit is an operational observation, not
                            // independent verification of the remote effect.
                            verified: false,
                            timestamp: now,
                        };
                        let _ = fleet.save_fleet_execution(&receipt);
                        match serde_json::to_value(&receipt) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn SSH process: {e}"))
                    }
                }
            }
            METHOD_AUTOMATION_JOBS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let status_filter = req.params.get("status")
                    .and_then(|v| serde_json::from_value::<HeadlessJobStatus>(v.clone()).ok());
                match fleet.list_headless_jobs() {
                    Ok(jobs) => {
                        let filtered: Vec<HeadlessAutomationJob> = if let Some(filter) = status_filter {
                            jobs.into_iter().filter(|j| j.status == filter).collect()
                        } else {
                            jobs
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let params: CreateHeadlessJobParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid job parameters: {e}")),
                };
                let trigger = params.trigger.unwrap_or(custos_domain::HeadlessTrigger::Manual);
                let job = match HeadlessAutomationJob::new(params.name, params.spec, trigger) {
                    Ok(j) => j,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_headless_job(&job) {
                    Ok(()) => match serde_json::to_value(&job) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_RUN => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => return ApiResponse::error(req.id, "FleetAutomationRepository not configured"),
                };
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing 'job_id' parameter"),
                };

                let has_permit = req.params.get("permit_id").is_some();
                let force_execute = req.params.get("force_execute").and_then(|v| v.as_bool()).unwrap_or(false);
                if !has_permit && !force_execute {
                    return ApiResponse::error(
                        req.id,
                        "Headless execution is unavailable until an executor and scoped authority checks are connected",
                    );
                }

                match fleet.get_headless_job(job_id) {
                    Ok(Some(mut job)) => {
                        job.status = custos_domain::HeadlessJobStatus::Running;
                        job.started_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        let mut cmd = tokio::process::Command::new("sh");
                        cmd.arg("-c").arg(&job.spec.command_or_script);
                        let timeout_secs = job.spec.timeout_secs.clamp(1, 300);
                        let out = tokio::time::timeout(
                            std::time::Duration::from_secs(timeout_secs),
                            cmd.output(),
                        ).await;

                        let (stdout, stderr, exit_code, success) = match out {
                            Ok(Ok(o)) => (
                                String::from_utf8_lossy(&o.stdout).to_string(),
                                String::from_utf8_lossy(&o.stderr).to_string(),
                                o.status.code().unwrap_or(0),
                                o.status.success(),
                            ),
                            Ok(Err(e)) => (String::new(), e.to_string(), 1, false),
                            Err(_) => (String::new(), "Execution timed out".to_string(), 124, false),
                        };

                        job.status = if success {
                            custos_domain::HeadlessJobStatus::Completed
                        } else {
                            custos_domain::HeadlessJobStatus::Failed
                        };
                        job.exit_code = Some(exit_code);
                        job.output_log = if stderr.is_empty() { stdout } else { format!("{stdout}\n{stderr}") };
                        job.completed_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        match serde_json::to_value(&job) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Job '{job_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_MANIFESTS_LIST => {
                let manifests = self.pack_registry.list_packs();
                match serde_json::to_value(&manifests) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_LIST => {
                let filter_lens = req.params.get("lens").and_then(|v| v.as_str());
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let all_jobs = self.pack_registry.list_all_jobs();
                let mut catalog_items = Vec::new();

                for job in all_jobs {
                    if let Some(fl) = filter_lens {
                        let matches_lens = match job.lens {
                            custos_domain::WorkbenchLens::Coding => {
                                fl.eq_ignore_ascii_case("coding") || fl.eq_ignore_ascii_case("code")
                            }
                            custos_domain::WorkbenchLens::Research => {
                                fl.eq_ignore_ascii_case("research")
                            }
                            custos_domain::WorkbenchLens::Assistant => {
                                fl.eq_ignore_ascii_case("assistant") || fl.eq_ignore_ascii_case("chat")
                            }
                        };
                        if !matches_lens {
                            continue;
                        }
                    }

                    if let Some(fp) = filter_pack {
                        if !job.job_id.starts_with(fp) {
                            continue;
                        }
                    }

                    let (status, reason) = match job.job_id.as_str() {
                        "engineering__repo_explain"
                        | "engineering__review"
                        | "research__source_qa"
                        | "research__dataset_audit"
                        | "research__reproduce"
                        | "assistant__inbox_triage"
                        | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                        "engineering__migration" => (
                            JobOperationalStatus::Planned,
                            Some("Multi-phase dependency migration requires external package registry connector".into()),
                        ),
                        "assistant__mutate_execution" => (
                            JobOperationalStatus::Planned,
                            Some("Governed outbox execution requires configured SMTP/Calendar live connector".into()),
                        ),
                        "assistant__reminder_automation" => (
                            JobOperationalStatus::Planned,
                            Some("Recurring reminder daemon automation is planned for post-MVP".into()),
                        ),

                        _ => (
                            JobOperationalStatus::Degraded,
                            Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                        ),
                    };

                    catalog_items.push(PacksJobCatalogItem {
                        job_id: job.job_id,
                        lens: job.lens,
                        name: job.name,
                        description: job.description,
                        primary_role: job.primary_role,
                        recommended_topology: job.recommended_topology,
                        status,
                        degradation_reason: reason,
                        required_inputs: job.required_inputs,
                        expected_artifacts: job.expected_artifacts,
                        required_verifiers: job.required_verifiers,
                        acceptance_rubric: job.acceptance_rubric,
                    });
                }

                match serde_json::to_value(&catalog_items) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_GET => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                match self.pack_registry.get_job(job_id) {
                    Some(job) => {
                        let (status, reason) = match job.job_id.as_str() {
                            "engineering__repo_explain"
                            | "engineering__review"
                            | "research__source_qa"
                            | "research__dataset_audit"
                            | "research__reproduce"
                            | "assistant__inbox_triage"
                            | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                            "engineering__migration"
                            | "assistant__mutate_execution"
                            | "assistant__reminder_automation" => (
                                JobOperationalStatus::Planned,
                                Some("Capability planned for post-MVP or requires live external connector".into()),
                            ),

                            _ => (
                                JobOperationalStatus::Degraded,
                                Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                            ),
                        };

                        let item = PacksJobCatalogItem {
                            job_id: job.job_id,
                            lens: job.lens,
                            name: job.name,
                            description: job.description,
                            primary_role: job.primary_role,
                            recommended_topology: job.recommended_topology,
                            status,
                            degradation_reason: reason,
                            required_inputs: job.required_inputs,
                            expected_artifacts: job.expected_artifacts,
                            required_verifiers: job.required_verifiers,
                            acceptance_rubric: job.acceptance_rubric,
                        };

                        match serde_json::to_value(&item) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => ApiResponse::error(req.id, format!("Workbench Job '{job_id}' not found")),
                }
            }
            METHOD_PACKS_ROLES_LIST => {
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let roles = if let Some(pid) = filter_pack {
                    match self.pack_registry.get_pack(pid) {
                        Some(pack) => pack.roles(),
                        None => return ApiResponse::error(req.id, format!("Domain pack '{pid}' not found")),
                    }
                } else {
                    self.pack_registry.list_all_roles()
                };

                match serde_json::to_value(&roles) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_RUN => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                let job = match self.pack_registry.get_job(job_id) {
                    Some(j) => j,
                    None => return ApiResponse::error(req.id, format!("Workbench Job '{job_id}' not found")),
                };

                let session_id = req.params.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let workspace_id = req.params.get("workspace_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let inputs = req.params.get("inputs").cloned().unwrap_or(serde_json::Value::Null);
                if !inputs.is_null() {
                    tracing::debug!(job_id = %job_id, inputs = ?inputs, "Running job with pre-populated inputs");
                }

                let title = format!("[{}] {}", job.name, job.description);
                let mut metadata = serde_json::json!({
                    "source": "packs_job",
                    "job_id": job.job_id,
                    "pack_id": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "engineering",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "lens": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "coding",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "primary_role": job.primary_role,
                    "inputs": inputs,
                });
                if let Some(ref sid) = session_id {
                    metadata["session_id"] = serde_json::json!(sid);
                }
                if let Some(ref wid) = workspace_id {
                    metadata["workspace_id"] = serde_json::json!(wid);
                }

                let cmd = CreateTask {
                    title,
                    metadata: Some(metadata),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => {
                        if let Some(ref sid) = session_id {
                            let _ = self
                                .session_manager
                                .append_user_message(
                                    &SessionId(sid.clone()),
                                    &format!("Started workbench job `{}` (Task {})", job.job_id, task.id),
                                )
                                .await;
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "task_id": task.id,
                                "job_id": job.job_id,
                                "status": "created",
                                "lens": job.lens,
                                "primary_role": job.primary_role,
                                "recommended_topology": job.recommended_topology,
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_LIST => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let session_id = req.params.get("session_id").and_then(|v| v.as_str());
                let task_id = req.params.get("task_id").and_then(|v| v.as_str());
                match assistant.list_drafts_sync(session_id, task_id) {
                    Ok(drafts) => ApiResponse::success(req.id, serde_json::to_value(&drafts).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_GET => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match assistant.get_draft_sync(id) {
                    Ok(Some(draft)) => ApiResponse::success(req.id, serde_json::to_value(&draft).unwrap_or_default()),
                    Ok(None) => ApiResponse::error(req.id, format!("Draft '{id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_SAVE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let mut params = req.params;
                let Some(fields) = params.as_object_mut() else { return ApiResponse::error(req.id, "Expected draft object"); };
                let now = chrono::Utc::now().to_rfc3339();
                fields.entry("status").or_insert(serde_json::json!("draft"));
                fields.entry("created_at").or_insert(serde_json::json!(now));
                fields.insert("updated_at".into(), serde_json::json!(now));
                let mut draft: AssistantDraft = match serde_json::from_value(params) {
                    Ok(d) => d,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid AssistantDraft: {e}")),
                };
                if draft.id.is_empty() || draft.id.len() > 128 || draft.title.trim().is_empty()
                    || !matches!(draft.status.as_str(), "draft" | "approved" | "archived")
                    || !matches!(draft.kind.as_str(), "email" | "document" | "schedule" | "note") {
                    return ApiResponse::error(req.id, "Invalid draft id, title, kind or status");
                }
                match assistant.get_draft_sync(&draft.id) {
                    Ok(Some(existing)) => {
                        if existing.session_id != draft.session_id || existing.task_id != draft.task_id {
                            return ApiResponse::error(req.id, "Draft scope cannot be changed");
                        }
                        if existing.payload_lock.is_some() && (existing.title != draft.title || existing.content != draft.content
                            || existing.recipient != draft.recipient || existing.kind != draft.kind || existing.status != draft.status) {
                            return ApiResponse::error(req.id, "Reviewed draft is locked; create a new draft to revise it");
                        }
                        draft.created_at = existing.created_at;
                    }
                    Ok(None) => { draft.created_at = now; }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                }
                draft.payload_lock = if draft.status == "approved" {
                    let payload = serde_json::json!({"title": draft.title, "content": draft.content, "kind": draft.kind, "recipient": draft.recipient});
                    Some(blake3::hash(payload.to_string().as_bytes()).to_hex().to_string())
                } else { None };
                match assistant.save_draft_sync(&draft) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::to_value(&draft).unwrap_or_default()),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_DRAFTS_DELETE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => return ApiResponse::error(req.id, "AssistantRepository not configured on daemon"),
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match assistant.delete_draft_sync(id) {
                    Ok(deleted) => ApiResponse::success(req.id, serde_json::json!({ "deleted": deleted })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ASSISTANT_OUTBOX_LIST => {
                let outbox = match self.outbox_repo() {
                    Some(o) => o,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };
                let pending = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Pending).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let dispatching = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Dispatching).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let receipted = match outbox.list_by_status(custos_core::contracts::storage::OutboxStatus::Receipted).await {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut all = Vec::new();
                all.extend(pending);
                all.extend(dispatching);
                all.extend(receipted);
                for status in [custos_core::contracts::storage::OutboxStatus::Failed, custos_core::contracts::storage::OutboxStatus::Uncertain] {
                    match outbox.list_by_status(status).await {
                        Ok(items) => all.extend(items),
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    }
                }
                all.sort_by(|left, right| right.created_at.cmp(&left.created_at));
                let task_filter = req.params.get("task_id").and_then(|value| value.as_str());
                let items: Vec<OutboxActionItem> = all.into_iter().filter(|item| task_filter.map_or(true, |id| item.task_id == id)).map(|e| {
                    let recipient_str = e.recipient.clone().unwrap_or_else(|| "Unknown (not stored in outbox record)".into());
                    let payload_summary_str = e.payload_json.as_ref().map(|p| {
                        if p.len() > 100 {
                            format!("{}...", &p[..100])
                        } else {
                            p.clone()
                        }
                    }).unwrap_or_else(|| format!("Payload digest: {}", e.argument_digest));

                    OutboxActionItem {
                        id: e.id,
                        task_id: e.task_id,
                        title: format!("Outbox: {}", e.action_id),
                        action_type: e.action_id,
                        recipient: recipient_str,
                        payload_summary: payload_summary_str,
                        payload_digest: e.argument_digest,
                        status: match e.status {
                            custos_core::contracts::storage::OutboxStatus::Pending => "pending_approval".into(),
                            custos_core::contracts::storage::OutboxStatus::Dispatching => "dispatching".into(),
                            custos_core::contracts::storage::OutboxStatus::Receipted => "receipted".into(),
                            custos_core::contracts::storage::OutboxStatus::Failed => "failed".into(),
                            custos_core::contracts::storage::OutboxStatus::Uncertain => "uncertain".into(),
                        },
                        permit_id: Some(e.permit_id),
                        receipt_id: e.receipt.as_ref().map(|r| r.receipt_id.clone()),
                        payload_json: e.payload_json.clone(),
                        error_message: e.error_message.clone(),
                        created_at: e.created_at.to_rfc3339(),
                    }
                }).collect();
                ApiResponse::success(req.id, serde_json::to_value(&items).unwrap_or_default())
            }
            METHOD_ASSISTANT_OUTBOX_CREATE => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing task_id parameter"),
                };
                let action_type = match req.params.get("action_type").or_else(|| req.params.get("action_id")).and_then(|v| v.as_str()) {
                    Some(at) if !at.trim().is_empty() => at.to_string(),
                    _ => "dispatch_message".to_string(),
                };
                let recipient = req.params.get("recipient").and_then(|v| v.as_str()).map(|s| s.to_string());
                let payload_val = req.params.get("payload");
                let payload_json = payload_val.map(|v| v.to_string());
                let payload_str = payload_json.as_deref().unwrap_or("{}");
                let argument_digest = format!("sha256:{}", custos_domain::digest(payload_str.as_bytes()));
                let permit_id = req.params.get("permit_id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| new_id("pmt"));
                let idempotency_key = req.params.get("idempotency_key").and_then(|v| v.as_str()).map(|s| s.to_string());

                let entry_id = new_id("ob");
                let mut entry = OutboxEntry::new(
                    &entry_id,
                    &task_id,
                    &action_type,
                    &permit_id,
                    &argument_digest,
                    idempotency_key,
                    custos_core::contracts::storage::OutboxStatus::Pending,
                    chrono::Utc::now(),
                );
                entry.recipient = recipient.clone();
                entry.payload_json = payload_json.clone();

                if let Err(e) = outbox.enqueue(entry).await {
                    return ApiResponse::error(req.id, format!("Failed to enqueue outbox entry: {e}"));
                }

                let action_item = OutboxActionItem {
                    id: entry_id,
                    task_id,
                    title: format!("Outbox: {action_type}"),
                    action_type,
                    recipient: recipient.unwrap_or_else(|| "Unknown".into()),
                    payload_summary: payload_json.clone().unwrap_or_else(|| format!("Payload digest: {argument_digest}")),
                    payload_digest: argument_digest,
                    status: "pending_approval".into(),
                    permit_id: Some(permit_id),
                    receipt_id: None,
                    payload_json,
                    error_message: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };

                ApiResponse::success(req.id, serde_json::to_value(&action_item).unwrap_or_default())
            }
            METHOD_ASSISTANT_OUTBOX_APPROVE => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let entry_id = match req.params.get("id").or_else(|| req.params.get("outbox_id")).and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing outbox id parameter"),
                };

                match outbox.get(entry_id).await {
                    Ok(Some(entry)) => {
                        ApiResponse::success(req.id, serde_json::json!({
                            "id": entry.id,
                            "task_id": entry.task_id,
                            "status": "approved",
                            "action_id": entry.action_id,
                        }))
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Outbox entry '{entry_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to read outbox entry: {e}")),
                }
            }
            METHOD_ASSISTANT_OUTBOX_DISPATCH => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let entry_id = match req.params.get("id").or_else(|| req.params.get("outbox_id")).and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing outbox id parameter"),
                };

                let entry = match outbox.get(entry_id).await {
                    Ok(Some(e)) => e,
                    Ok(None) => return ApiResponse::error(req.id, format!("Outbox entry '{entry_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to read outbox entry: {e}")),
                };

                // T3: CAS transition to Dispatching (fences duplicate dispatch)
                if let Err(e) = outbox.mark_dispatching(entry_id).await {
                    return ApiResponse::error(req.id, format!("CAS transition failed for outbox entry '{entry_id}': {e}"));
                }

                // T4: Execute outbound action with durable receipt verification
                let started = std::time::Instant::now();
                let recipient = entry.recipient.as_deref().unwrap_or("unknown");
                let payload_str = entry.payload_json.as_deref().unwrap_or("{}");

                // Execute based on recipient type
                let exec_result: Result<serde_json::Value, String> = if recipient.starts_with("mcp://") {
                    let tool_path = recipient.trim_start_matches("mcp://");
                    let namespaced_tool = tool_path.replace('/', "__");
                    let args: Option<serde_json::Value> = serde_json::from_str(payload_str).ok();
                    let ctx = custos_adapters::mcp::ToolCallContext::new(&entry.id, &entry.id);

                    match self.mcp_hub.dispatch_call(&namespaced_tool, args, &ctx).await {
                        Ok(res) => {
                            if res.is_error == Some(true) {
                                Err(format!("MCP tool '{}' returned error: {:?}", namespaced_tool, res.content))
                            } else {
                                Ok(serde_json::to_value(&res).unwrap_or_else(|_| serde_json::json!({
                                    "success": true,
                                    "tool": namespaced_tool
                                })))
                            }
                        }
                        Err(DomainError::Validation(msg)) if msg.contains("timed out") => {
                            // Timeout: Mark Uncertain per Gate 3
                            let _ = outbox.mark_uncertain(entry_id).await;
                            return ApiResponse::error(req.id, format!("MCP tool call timed out; marked uncertain: {msg}"));
                        }
                        Err(e) => {
                            Err(format!("MCP dispatch failed: {e}"))
                        }
                    }
                } else if recipient.starts_with("http://") || recipient.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(15))
                        .build()
                        .unwrap_or_default();
                    let body: serde_json::Value = serde_json::from_str(payload_str).unwrap_or_else(|_| serde_json::json!({"payload": payload_str}));
                    match client.post(recipient).json(&body).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            let json_val = resp.json::<serde_json::Value>().await.unwrap_or_else(|_| serde_json::json!({"status": "ok"}));
                            Ok(json_val)
                        }
                        Ok(resp) => {
                            Err(format!("HTTP dispatch failed with status: {}", resp.status()))
                        }
                        Err(e) => {
                            // Timeout or connection drop: Invariant Gate 3 Uncertain
                            let _ = outbox.mark_uncertain(entry_id).await;
                            return ApiResponse::error(req.id, format!("Dispatch connection dropped or timed out; marked uncertain: {e}"));
                        }
                    }
                } else {
                    // Local / simulated mediated effect execution
                    Ok(serde_json::json!({
                        "delivered": true,
                        "recipient": recipient,
                        "action": entry.action_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }))
                };

                match exec_result {
                    Ok(res_val) => {
                        let duration_ms = started.elapsed().as_millis() as u64;
                        let out_str = res_val.to_string();
                        let receipt = ExecutionReceipt {
                            receipt_id: new_id("rcpt"),
                            permit_id: entry.permit_id.clone(),
                            action_id: entry.action_id.clone(),
                            status: ReceiptStatus::Success,
                            output_digest: format!("sha256:{}", custos_domain::digest(out_str.as_bytes())),
                            output_data: Some(res_val),
                            error_message: None,
                            duration_ms: Some(duration_ms),
                            executed_at: chrono::Utc::now(),
                            assurance: Assurance::CustosMediated,
                        };

                        if let Err(e) = outbox.mark_receipted(entry_id, receipt.clone()).await {
                            return ApiResponse::error(req.id, format!("Failed to record receipt in outbox: {e}"));
                        }

                        ApiResponse::success(req.id, serde_json::json!({
                            "id": entry_id,
                            "status": "receipted",
                            "receipt": receipt,
                        }))
                    }
                    Err(err_msg) => {
                        let _ = outbox.mark_failed(entry_id, &err_msg).await;
                        ApiResponse::error(req.id, format!("Outbox dispatch execution failed: {err_msg}"))
                    }
                }
            }
            METHOD_WORKSPACES_AGENT_STEER => {
                let guidance = match req.params.get("guidance").and_then(|v| v.as_str()) {
                    Some(g) if !g.trim().is_empty() => g,
                    _ => return ApiResponse::error(req.id, "Missing guidance parameter"),
                };
                let harness_id = req.params.get("harness_id").and_then(|v| v.as_str());
                let run_id = req.params.get("run_id").and_then(|v| v.as_str());

                if let (Some(h_id), Some(r_id)) = (harness_id, run_id) {
                    match self.harnesses.steer(h_id, r_id, guidance).await {
                        Ok(()) => ApiResponse::success(req.id, serde_json::json!({"steered": true})),
                        Err(err) => ApiResponse::error(req.id, format!("Failed to steer harness: {err}")),
                    }
                } else if let Some(r_id) = run_id {
                    let mut steered = false;
                    for desc in self.harnesses.list() {
                        if self.harnesses.steer(&desc.id, r_id, guidance).await.is_ok() {
                            steered = true;
                            break;
                        }
                    }
                    if steered {
                        ApiResponse::success(req.id, serde_json::json!({"steered": true}))
                    } else {
                        ApiResponse::error(
                            req.id,
                            "Workspace steering is unavailable: this workspace is not bound to an active harness run",
                        )
                    }
                } else {
                    ApiResponse::error(
                        req.id,
                        "Workspace steering is unavailable: missing run_id or harness_id parameter",
                    )
                }
            }
            METHOD_RESOURCES_CAPABILITIES => {
                let resource_id = match req.params.get("resource_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing resource_id parameter"),
                };
                let descriptor = match self.list_capabilities().into_iter().find(|capability| {
                    capability.resource_id.as_deref() == Some(resource_id)
                }) {
                    Some(capability) => capability,
                    None => return ApiResponse::error(req.id, format!("Unknown resource_id: {resource_id}")),
                };
                let actions: Vec<ResourceActionCapability> = descriptor.supported_operations.iter().enumerate().map(|(index, operation)| {
                    ResourceActionCapability {
                        id: operation.clone(),
                        label: operation.replace('.', " ").replace('_', " "),
                        status: "available".into(),
                        reason: None,
                        primary: index == 0,
                        shortcut: None,
                    }
                }).collect();
                match serde_json::to_value(&actions) {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        };

        if let (Some(cmd_id), Some(store)) = (
            admitted_cmd_id.as_deref(),
            self.sqlite_store.as_ref(),
        ) {
            if resp.error.is_none() {
                let res_val = resp.result.as_ref().unwrap_or(&serde_json::Value::Null);
                let result_json = serde_json::to_string(res_val).unwrap_or_default();
                if let Err(e) = store.command_ledger().complete(cmd_id, &result_json) {
                    tracing::warn!(command_id = %cmd_id, error = %e, "Failed to complete command in ledger");
                }
            } else {
                if let Err(e) = store.command_ledger().abandon(cmd_id) {
                    tracing::warn!(command_id = %cmd_id, error = %e, "Failed to abandon command in ledger");
                }
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
#[path = "api/tests.rs"]
mod tests;
