//! Sub-contexts decomposing [`super::LocalApiDispatcher`] into 4 cohesive bounded contexts.

use super::*;

pub(crate) struct KernelServices {
    pub(crate) task_service: Arc<TaskService>,
    pub(crate) session_manager: Arc<SessionManager>,
    pub(crate) bridge_service: Arc<BridgeService>,
    pub(crate) workflow: Option<Arc<dyn WorkflowPort>>,
    pub(crate) usage_runs: Option<Arc<dyn custos_core::contracts::storage::RunPort>>,
    pub(crate) sqlite_store: Option<Arc<custos_persistence::SqliteTaskStore>>,
    pub(crate) supervision: Arc<custos_runtime::workflow::SupervisionCoordinator>,
}

pub(crate) struct EnvironmentServices {
    pub(crate) workspace: Option<Arc<WorkspaceCoordinator>>,
    pub(crate) candidate_mutation: tokio::sync::Mutex<()>,
    pub(crate) terminal: Arc<TerminalCoordinator>,
    pub(crate) files: Arc<dyn WorkspaceFilesPort>,
    pub(crate) harnesses: Arc<HarnessRegistry>,
    pub(crate) python_kernel: Option<Arc<PythonKernelCoordinator>>,
    pub(crate) admission_coordinator: Option<Arc<custos_runtime::compute::WorkloadAdmissionCoordinator>>,
    pub(crate) environment_coordinator: Option<Arc<custos_runtime::environment::EnvironmentCoordinator>>,
    pub(crate) target_resolver: Arc<dyn ExecutionTargetResolver>,
}

pub(crate) struct ProviderServices {
    pub(crate) repo: Option<Arc<ProviderRepository>>,
    pub(crate) credentials: Option<Arc<crate::credentials::ProviderCredentials>>,
    pub(crate) model_availability: custos_provider::routing::availability::ModelAvailability,
    pub(crate) model_catalog: custos_provider::catalog::ModelCatalogService,
    pub(crate) oauth_callback_server: Option<Arc<crate::oauth_callback_server::OAuthCallbackServer>>,
}

pub(crate) struct WorkbenchServices {
    pub(crate) pack_registry: Arc<custos_packs::DomainPackRegistry>,
    pub(crate) mcp_hub: Arc<custos_adapters::mcp::McpFederationHub>,
    pub(crate) shared_capabilities: Option<Arc<custos_persistence::SharedCapabilitiesRepository>>,
    pub(crate) outbox: Option<Arc<OutboxRepository>>,
    #[cfg(feature = "custos-workbench-research")]
    pub(crate) research: std::sync::RwLock<Option<Arc<ResearchRepository>>>,
    #[cfg(feature = "custos-workbench-quant")]
    pub(crate) quant: std::sync::RwLock<Option<Arc<dyn custos_workbench_quant::QuantPort>>>,
    #[cfg(feature = "custos-workbench-assistant")]
    pub(crate) assistant: std::sync::RwLock<Option<Arc<AssistantRepository>>>,
    pub(crate) browser: Option<Arc<BrowserRepository>>,
    pub(crate) fleet_automation: Option<Arc<FleetAutomationRepository>>,
}

