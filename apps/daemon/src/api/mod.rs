//! Custos Local API Contracts & Dispatcher
//!
//! Protocol types and dispatcher for IPC communication between CLI / UI / VS Code and custosd.

use custos_domain::DomainError;
use std::sync::Arc;

mod builder;

pub use crate::custos_local_api::{
    AdvanceTaskRequest, ApiRequest, ApiResponse, ArchiveWorkspaceApiRequest,
    ArtifactDetailResponse, CancelHarnessRunRequest, CancelRunRequest, CancelTaskRequest,
    CompleteTaskRequest, CreateTaskRequest, CreateWorkspaceApiRequest, GetCapabilityApiRequest,
    GetHarnessRequest, GetWorkspaceApiRequest, InspectWorkspaceDirtyApiRequest,
    JobOperationalStatus, OAuthAuthorizeParams, OAuthExchangeParams, OAuthRefreshParams,
    PacksJobCatalogItem, RecoverWorkspaceApiRequest, RunNativeHarnessRequest, SaveNoteParams,
    StartRunRequest, SteerHarnessRunRequest, METHOD_ARTIFACTS_GET, METHOD_ARTIFACTS_LINEAGE_GRAPH,
    METHOD_ARTIFACTS_LIST, METHOD_ARTIFACTS_RECORD_LINEAGE, METHOD_ASSISTANT_DRAFTS_DELETE,
    METHOD_ASSISTANT_DRAFTS_GET, METHOD_ASSISTANT_DRAFTS_LIST, METHOD_ASSISTANT_DRAFTS_SAVE,
    METHOD_ASSISTANT_OUTBOX_APPROVE, METHOD_ASSISTANT_OUTBOX_CREATE,
    METHOD_ASSISTANT_OUTBOX_DISPATCH, METHOD_ASSISTANT_OUTBOX_LIST, METHOD_AUTOMATION_JOBS_CREATE,
    METHOD_AUTOMATION_JOBS_LIST, METHOD_AUTOMATION_JOBS_RUN, METHOD_BROWSER_SESSIONS_CREATE,
    METHOD_BROWSER_SESSIONS_LIST, METHOD_BROWSER_TABS_CLICK, METHOD_BROWSER_TABS_CLOSE,
    METHOD_BROWSER_TABS_CREATE, METHOD_BROWSER_TABS_FILL, METHOD_BROWSER_TABS_LIST,
    METHOD_BROWSER_TABS_NAVIGATE, METHOD_BROWSER_TABS_SCREENSHOT, METHOD_BROWSER_TABS_SNAPSHOT,
    METHOD_CAPABILITIES_GET, METHOD_CAPABILITIES_LIST, METHOD_COMPUTE_TARGETS_LIST,
    METHOD_EMULATOR_DEVICES_LIST, METHOD_EMULATOR_LAUNCH, METHOD_EMULATOR_NAVIGATE,
    METHOD_EMULATOR_SCREENSHOT, METHOD_EMULATOR_SNAPSHOT, METHOD_EMULATOR_TAP,
    METHOD_EXPERIMENT_COLLECT, METHOD_EXPERIMENT_OBSERVE, METHOD_EXPERIMENT_PREFLIGHT,
    METHOD_EXPERIMENT_RELEASE, METHOD_EXPERIMENT_STOP, METHOD_EXPERIMENT_SUBMIT, METHOD_FLEET_EXEC,
    METHOD_FLEET_HOSTS_LIST, METHOD_FLEET_HOSTS_PING, METHOD_FLEET_HOSTS_REGISTER,
    METHOD_GITHUB_ACCOUNTS, METHOD_GITHUB_AUTH_STATUS, METHOD_GITHUB_LOGIN, METHOD_GITHUB_VIEWER,
    METHOD_GIT_ACCOUNT, METHOD_GIT_FETCH, METHOD_GIT_GITHUB, METHOD_GIT_REPOSITORY,
    METHOD_GIT_SWITCH, METHOD_HARNESS_CANCEL, METHOD_HARNESS_GET, METHOD_HARNESS_LIST,
    METHOD_HARNESS_RUN_NATIVE, METHOD_HARNESS_STEER, METHOD_MODELS_CATALOG, METHOD_MODELS_PRICING,
    METHOD_MODELS_PROBE, METHOD_NOTEBOOK_CELLS_LIST, METHOD_NOTEBOOK_CELLS_SAVE,
    METHOD_NOTEBOOK_EXECUTE, METHOD_NOTEBOOK_INTERRUPT, METHOD_NOTEBOOK_RESET,
    METHOD_NOTEBOOK_STATUS, METHOD_NOTES_GET, METHOD_NOTES_HISTORY, METHOD_NOTES_LIST,
    METHOD_NOTES_SAVE, METHOD_OAUTH_AUTHORIZE, METHOD_OAUTH_CANCEL, METHOD_OAUTH_DELETE,
    METHOD_OAUTH_EXCHANGE, METHOD_OAUTH_GET, METHOD_OAUTH_REFRESH, METHOD_OAUTH_STATUS,
    METHOD_PACKS_JOBS_GET, METHOD_PACKS_JOBS_LIST, METHOD_PACKS_JOBS_RUN,
    METHOD_PACKS_MANIFESTS_LIST, METHOD_PACKS_ROLES_LIST, METHOD_PIPELINE_RUN,
    METHOD_PIPELINE_STATUS, METHOD_PIPELINE_STRUCTURE_DETECT, METHOD_PROVIDERS_DELETE,
    METHOD_PROVIDERS_GET, METHOD_PROVIDERS_LIST, METHOD_PROVIDERS_SAVE,
    METHOD_QUANT_BACKTEST_SUBMIT, METHOD_QUANT_FACTORS_DISCOVER, METHOD_QUANT_GATE_EVALUATE,
    METHOD_QUANT_GRID_EVALUATE, METHOD_QUANT_INDICATORS_COMPUTE, METHOD_QUANT_KILL_SWITCH,
    METHOD_QUANT_MARKET_BARS_GET, METHOD_QUANT_MARKET_BARS_INGEST, METHOD_QUANT_ORDERBOOK_GET,
    METHOD_QUANT_ORDERS_APPROVE, METHOD_QUANT_ORDERS_CREATE, METHOD_QUANT_ORDERS_LIST,
    METHOD_QUANT_PBO_COMPUTE, METHOD_QUANT_REVIEW_COMPUTE, METHOD_QUANT_TEARSHEET_GET,
    METHOD_RESEARCH_ANCHORS_LIST, METHOD_RESEARCH_ANCHORS_SAVE, METHOD_RESEARCH_CLAIMS_LIST,
    METHOD_RESEARCH_CLAIMS_SAVE, METHOD_RESEARCH_HANDOFF_CODING, METHOD_RESEARCH_LINEAGE_LIST,
    METHOD_RESEARCH_RUNS_LIST, METHOD_RESEARCH_RUNS_SAVE, METHOD_RESEARCH_SOURCES_LIST,
    METHOD_RESEARCH_SOURCES_SAVE, METHOD_RESOURCES_CAPABILITIES, METHOD_REVIEWS_GET,
    METHOD_REVIEWS_LIST, METHOD_REVIEWS_MARK_STALE, METHOD_REVIEWS_RECORD, METHOD_SESSIONS_DELETE,
    METHOD_SETTINGS_GET, METHOD_SETTINGS_SET, METHOD_SYNTHESIS_HANDOFF_EXECUTE,
    METHOD_SYNTHESIS_PROPOSALS_GET, METHOD_SYNTHESIS_PROPOSALS_LIST,
    METHOD_SYNTHESIS_PROPOSALS_SAVE, METHOD_TERMINAL_GET, METHOD_TERMINAL_LIST,
    METHOD_TERMINAL_READ, METHOD_TERMINAL_RESIZE, METHOD_TERMINAL_SPAWN, METHOD_TERMINAL_TERMINATE,
    METHOD_TERMINAL_WRITE, METHOD_WORKFLOW_CANCEL_RUN, METHOD_WORKFLOW_START_RUN,
    METHOD_WORKSPACES_AGENT_STEER, METHOD_WORKSPACES_ARCHIVE, METHOD_WORKSPACES_CANDIDATES_COMPARE,
    METHOD_WORKSPACES_CANDIDATES_RACE, METHOD_WORKSPACES_CANDIDATES_SPAWN,
    METHOD_WORKSPACES_CREATE, METHOD_WORKSPACES_DETECT, METHOD_WORKSPACES_GET,
    METHOD_WORKSPACES_INSPECT_DIRTY, METHOD_WORKSPACES_INTEGRATE, METHOD_WORKSPACES_LIST,
    METHOD_WORKSPACES_PICK_FOLDER, METHOD_WORKSPACES_RECOVER, METHOD_WORKSPACE_DIFF,
    METHOD_WORKSPACE_FILES_READ, METHOD_WORKSPACE_FILES_TREE, METHOD_WORKSPACE_FILES_WRITE,
    METHOD_WORKSPACE_FILE_DIFF, METHOD_WORKSPACE_GIT_COMMIT, METHOD_WORKSPACE_GIT_DISCARD,
    METHOD_WORKSPACE_GIT_STAGE, METHOD_WORKSPACE_GIT_UNSTAGE,
};
pub(crate) use custos_adapters::harness::HarnessRegistry;
pub(crate) use custos_runtime::bridge::BridgeService;
pub(crate) use custos_core::contracts::compute::{
    ExecutionTargetPort, ExecutionTargetResolver,
};
pub(crate) use custos_core::contracts::workflow::WorkflowPort;
pub(crate) use custos_core::contracts::workspace_files::WorkspaceFilesPort;
#[allow(unused_imports)]
pub(crate) use custos_core::{AdvanceTask, CreateTask, TaskService};
#[allow(unused_imports)]
pub(crate) use custos_domain::{
    new_id, validate_browser_url, BrowserConsoleEntry,
    BrowserNetworkRequest, BrowserPageSnapshot, BrowserSession, BrowserTab, BrowserTabStatus,
    ComputeBackendKind, CreateHeadlessJobParams,
    FleetExecReceipt, HeadlessAutomationJob, HeadlessJobStatus,
    OutboxActionItem, RegisterHostParams, RemoteHostNode, RemoteHostStatus,
    SessionId, SshAuthMethod, TaskContract,
};
pub(crate) use custos_persistence::repositories::OutboxRepository;
pub(crate) use custos_persistence::{
    BrowserRepository, FleetAutomationRepository, ProviderRepository,
};
#[cfg(feature = "custos-workbench-research")]
pub(crate) use custos_workbench_research::ResearchRepository;
#[cfg(feature = "custos-workbench-assistant")]
pub(crate) use custos_workbench_assistant::AssistantRepository;
pub(crate) use custos_runtime::session::SessionManager;
#[cfg(test)]
pub(crate) use custos_domain::ResourceActionCapability;
#[cfg(test)]
pub(crate) use custos_runtime::workspace::CreateWorkspaceRequest;
pub(crate) use custos_adapters::workspace::LocalWorkspaceFilesAdapter;
pub(crate) use custos_runtime::workspace::WorkspaceCoordinator;
pub(crate) use custos_runtime::{PythonKernelCoordinator, TerminalCoordinator};

pub mod admission;
pub mod commands;
mod contexts;

/// Local API Dispatcher wrapping 4 cohesive bounded sub-contexts for IPC callers.
pub struct LocalApiDispatcher {
    pub(crate) kernel: contexts::KernelServices,
    pub(crate) env: contexts::EnvironmentServices,
    pub(crate) providers: contexts::ProviderServices,
    pub(crate) workbenches: contexts::WorkbenchServices,
}

/// Concrete coordinators and registries injected by daemon composition root (A02).
pub struct LocalApiDependencies {
    pub terminal: Arc<TerminalCoordinator>,
    pub files: Arc<dyn WorkspaceFilesPort>,
    pub harnesses: Arc<HarnessRegistry>,
    pub python_kernel: Option<Arc<PythonKernelCoordinator>>,
    pub pack_registry: Arc<custos_packs::DomainPackRegistry>,
    pub mcp_hub: Arc<custos_adapters::mcp::McpFederationHub>,
    pub target_resolver: Option<Arc<dyn ExecutionTargetResolver>>,
}

impl LocalApiDispatcher {
    pub fn ensure_module_registered(component_id: &str) {
        let reg = custos_packs::ModuleRegistry::global();
        let _ = &reg;
        match component_id {
            #[cfg(feature = "custos-workbench-assistant")]
            "assistant" | "copilot" if !reg.has("assistant") => {
                reg.register(Arc::new(custos_workbench_assistant::AssistantModule::new()));
            }
            #[cfg(feature = "custos-workbench-coding")]
            "coding" | "engineering" if !reg.has("coding") => {
                reg.register(Arc::new(custos_workbench_coding::CodingModule::new()));
            }
            #[cfg(feature = "custos-workbench-research")]
            "research" if !reg.has("research") => {
                reg.register(Arc::new(custos_workbench_research::ResearchModule::new()));
            }
            #[cfg(feature = "custos-workbench-quant")]
            "quant" if !reg.has("quant") => {
                reg.register(Arc::new(custos_workbench_quant::QuantModule::new()));
            }
            _ => {}
        }
    }

    pub fn ensure_default_modules_registered() {
        for id in ["assistant", "coding", "research", "quant"] {
            Self::ensure_module_registered(id);
        }
    }

    /// Production constructor: receives explicit graph dependencies instantiated by the composition root.
    pub fn with_dependencies(
        task_service: Arc<TaskService>,
        session_manager: Arc<SessionManager>,
        bridge_service: Arc<BridgeService>,
        deps: LocalApiDependencies,
    ) -> Self {
        Self {
            kernel: contexts::KernelServices {
                task_service,
                session_manager,
                bridge_service,
                workflow: None,
                usage_runs: None,
                sqlite_store: None,
                supervision: Arc::new(custos_runtime::workflow::SupervisionCoordinator::default()),
            },
            env: contexts::EnvironmentServices {
                workspace: None,
                candidate_mutation: tokio::sync::Mutex::new(()),
                terminal: deps.terminal,
                files: deps.files,
                harnesses: deps.harnesses,
                python_kernel: deps.python_kernel,
                admission_coordinator: None,
                environment_coordinator: None,
                target_resolver: deps
                    .target_resolver
                    .unwrap_or_else(|| Arc::new(custos_adapters::compute::DefaultExecutionTargetResolver)),
            },
            providers: contexts::ProviderServices {
                repo: None,
                credentials: None,
                model_availability: Default::default(),
                model_catalog: custos_provider::catalog::ModelCatalogService::new(),
                oauth_callback_server: None,
            },
            workbenches: contexts::WorkbenchServices {
                pack_registry: deps.pack_registry,
                mcp_hub: deps.mcp_hub,
                shared_capabilities: None,
                outbox: None,
                #[cfg(feature = "custos-workbench-research")]
                research: std::sync::RwLock::new(None),
                #[cfg(feature = "custos-workbench-quant")]
                quant: std::sync::RwLock::new(None),
                #[cfg(feature = "custos-workbench-assistant")]
                assistant: std::sync::RwLock::new(None),
                browser: None,
                fleet_automation: None,
            },
        }
    }

    /// Legacy constructor for test fixtures. Production should use `with_dependencies`.
    pub fn new(
        task_service: Arc<TaskService>,
        session_manager: Arc<SessionManager>,
        bridge_service: Arc<BridgeService>,
    ) -> Self {
        Self::ensure_default_modules_registered();
        Self::with_dependencies(
            task_service,
            session_manager,
            bridge_service,
            LocalApiDependencies {
                terminal: Arc::new(TerminalCoordinator::new(Arc::new(
                    custos_adapters::terminal::NativePtyDriver,
                ))),
                files: Arc::new(LocalWorkspaceFilesAdapter::new()),
                harnesses: Arc::new(HarnessRegistry::new()),
                python_kernel: Some(Arc::new(PythonKernelCoordinator::new(Arc::new(
                    custos_adapters::notebook::LocalPythonDriver,
                )))),
                pack_registry: Arc::new(custos_packs::create_default_pack_registry()),
                mcp_hub: Arc::new(custos_adapters::mcp::McpFederationHub::new()),
                target_resolver: None,
            },
        )
    }

    pub fn with_target_resolver(
        mut self,
        target_resolver: Arc<dyn ExecutionTargetResolver>,
    ) -> Self {
        self.env.target_resolver = target_resolver;
        self
    }

    #[cfg(test)]
    pub fn for_test(
        task_service: Arc<TaskService>,
        session_manager: Arc<SessionManager>,
        bridge_service: Arc<BridgeService>,
    ) -> Self {
        Self::new(task_service, session_manager, bridge_service)
    }

    // Builder and dependency injection methods are decomposed into api/builder.rs

    pub(crate) async fn resolve_workspace_for_dispatch(
        &self,
        req_id: &str,
        ws_id_str: &str,
    ) -> Result<custos_domain::ExecutionWorkspace, ApiResponse> {
        let coordinator = match self.env.workspace.as_ref() {
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
    pub(crate) fn resolve_target_adapter(
        &self,
        backend: ComputeBackendKind,
        work_dir: &std::path::Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
    ) -> Arc<dyn ExecutionTargetPort> {
        self.env.target_resolver.resolve(
            backend,
            work_dir,
            ssh_host,
            ssh_port,
            ssh_user,
            pod_id,
            cloud_api_key,
        )
    }

    pub fn is_capability_available(&self, cap: &str) -> bool {
        match cap {
            "notebook" => self.env.python_kernel.is_some(),
            "workspace" => true,
            "terminal" => true,
            "documents" => true,
            "compute" => true,
            other => {
                self.list_capabilities().iter().any(|c| {
                    (c.id == other || c.resource_id.as_deref() == Some(other))
                        && c.status == custos_domain::CapabilityStatus::Available
                })
            }
        }
    }

    pub fn validate_pack_capabilities(&self, pack_id: &str) -> Result<(), String> {
        let canonical = match pack_id {
            "quant" => "quant",
            "research" => "research",
            "coding" | "engineering" => "engineering",
            "assistant" | "copilot" => "assistant",
            other => other,
        };
        let manifest = self
            .workbenches
            .pack_registry
            .get_pack_manifest(canonical)
            .or_else(|| custos_packs::get_manifest_for_pack(canonical));

        if let Some(manifest) = manifest {
            for cap in &manifest.required_capabilities {
                if !self.is_capability_available(cap) {
                    return Err(format!(
                        "Cannot enable workbench '{pack_id}': missing required capability '{cap}'"
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let response = match serde_json::from_str::<ApiRequest>(raw) {
            Ok(request) => self.handle_request(request).await,
            Err(error) => ApiResponse::error("", format!("Invalid request JSON: {error}")),
        };
        serde_json::to_string(&response).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    pub async fn live_run(
        &self,
        session_id: &str,
        run_id: &str,
        cursor: u64,
    ) -> Result<custos_domain::LiveRunReplay, DomainError> {
        if self
            .kernel
            .session_manager
            .get_session(&SessionId(session_id.into()))
            .await
            .is_none()
        {
            return Err(DomainError::Validation("Unknown session".into()));
        }
        self.kernel
            .workflow
            .as_ref()
            .ok_or_else(|| DomainError::Validation("Workflow unavailable".into()))?
            .live_run(session_id, run_id, cursor)
            .await
    }
}

#[async_trait::async_trait]
impl crate::custos_local_api::ApiTransport for LocalApiDispatcher {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        Ok(self.handle_request(req).await)
    }
}

#[cfg(test)]
mod tests;
