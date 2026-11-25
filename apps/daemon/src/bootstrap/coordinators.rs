//! Coordinators, TaskRuntime, and LocalApiDispatcher Bootstrap (SADE R8 §6.8)

use std::path::Path;
use std::sync::Arc;

use custos_contracts::DomainError;
use custos_contracts::ModelPort;
use custos_engine::bridge::BridgeService;
use custos_engine::compute::WorkloadAdmissionCoordinator;
use custos_engine::contracts::kernel::TrustedKernel;
use custos_engine::session::SessionManager;
use custos_engine::workflow::TaskRuntime;
use custos_engine::workspace::{WorkspaceCoordinator, WorkspaceLeaseManager};
use custos_engine::DomainPackRegistry;
use custos_engine::TaskService;
use custos_engine::{PythonKernelCoordinator, TerminalCoordinator};
use custos_integrations::mcp::McpFederationHub;
use custos_integrations::routing::availability::ModelAvailability;
use custos_integrations::sandbox::SovereignDeveloperAdapter;
use custos_integrations::terminal::NativePtyDriver;
use custos_integrations::workspace::LocalWorkspaceFilesAdapter;
use custos_integrations::workspace::LocalWorkspaceProvider;
use custos_storage::SqliteTaskStore;

use crate::api::{LocalApiDependencies, LocalApiDispatcher};
use crate::credentials::ProviderCredentials;
use crate::oauth_callback_server::OAuthCallbackServer;
use custos_integrations::harness::HarnessRegistry;

pub struct CoordinatorServices {
    pub workflow: Arc<TaskRuntime>,
    pub lease_manager: Arc<WorkspaceLeaseManager>,
    pub workspace_coordinator: Arc<WorkspaceCoordinator>,
    pub environment_coordinator: Arc<custos_engine::environment::EnvironmentCoordinator>,
    pub terminal_coordinator: Arc<TerminalCoordinator>,
    pub python_coordinator: Option<Arc<PythonKernelCoordinator>>,
    pub admission_coordinator: Arc<WorkloadAdmissionCoordinator>,
    pub local_api: Arc<LocalApiDispatcher>,
}

#[allow(clippy::too_many_arguments)]
pub fn init_coordinators_and_api(
    store: Arc<SqliteTaskStore>,
    credentials: Arc<ProviderCredentials>,
    kernel: Arc<TrustedKernel>,
    model: Arc<dyn ModelPort>,
    sandbox: Arc<SovereignDeveloperAdapter>,
    task_service: Arc<TaskService>,
    session_manager: Arc<SessionManager>,
    bridge_service: Arc<BridgeService>,
    harnesses: Arc<HarnessRegistry>,
    pack_registry: Arc<DomainPackRegistry>,
    mcp_hub: Arc<McpFederationHub>,
    workspace_root: &Path,
    availability: ModelAvailability,
    live_runs: custos_engine::workflow::live::LiveRuns,
    supervision: Arc<custos_engine::workflow::SupervisionCoordinator>,
) -> Result<CoordinatorServices, DomainError> {
    let lease_manager = Arc::new(WorkspaceLeaseManager::new(workspace_root.to_path_buf()));
    let workspace_provider = Arc::new(LocalWorkspaceProvider::new());
    let workspace_coordinator =
        Arc::new(WorkspaceCoordinator::new(store.clone(), workspace_provider));

    let admission_coordinator = Arc::new(WorkloadAdmissionCoordinator::default());
    let _ =
        custos_integrations::local::InferenceRuntime::with_admission(admission_coordinator.clone());

    let default_harness_id = harnesses.default_harness_id().map(str::to_string);
    let harness_runtimes = harnesses.runtimes();

    let workflow = Arc::new(
        TaskRuntime::new()
            .with_live_runs(live_runs)
            .with_kernel(kernel)
            .with_model(model)
            .with_harnesses(harness_runtimes, default_harness_id.as_deref())
            .with_sandbox(sandbox)
            .with_command_runner(Arc::new(
                custos_integrations::operations::LocalCommandRunner,
            ))
            .with_outbox(store.clone())
            .with_effect_ledger(store.clone())
            .with_run_store(store.clone())
            .with_session_store(store.clone())
            .with_journal_manager(session_manager.clone())
            .with_workspace_coordinator(workspace_coordinator.clone())
            .with_admission_coordinator(admission_coordinator.clone())
            .with_supervision(supervision.clone()),
    );
    workflow.validate_ready()?;
    let active_claims = store.runs().list_active_claims()?;
    workflow
        .dispatcher()
        .restore_from_active_claims(active_claims);

    let terminal_coordinator = Arc::new(TerminalCoordinator::new(Arc::new(NativePtyDriver)));
    // Register lazily-spawned PythonKernelCoordinator so on-demand packages requiring `notebook`
    // (e.g. `research`) can be enabled dynamically without restarting the daemon; no Python worker
    // process is spawned until a cell executes.
    let python_coordinator = Some(Arc::new(PythonKernelCoordinator::new(Arc::new(
        custos_integrations::notebook::LocalPythonDriver,
    ))));
    let files_coordinator = Arc::new(LocalWorkspaceFilesAdapter::new());

    let oauth_callback_server =
        Arc::new(OAuthCallbackServer::with_credentials(credentials.clone()));

    let api_deps = LocalApiDependencies {
        terminal: terminal_coordinator.clone(),
        files: files_coordinator,
        harnesses: harnesses.clone(),
        python_kernel: python_coordinator.clone(),
        pack_registry: pack_registry.clone(),
        mcp_hub,
        target_resolver: None,
    };

    let environment_coordinator =
        Arc::new(custos_engine::environment::EnvironmentCoordinator::new(
            workspace_coordinator.clone(),
            Some(terminal_coordinator.clone() as Arc<dyn custos_engine::contracts::TerminalPort>),
        ));

    let local_api = LocalApiDispatcher::with_dependencies(
        task_service,
        session_manager,
        bridge_service,
        api_deps,
    )
    .with_workflow(workflow.clone())
    .with_supervision(supervision)
    .with_usage_runs(store.clone())
    .with_sqlite_store(store.clone())
    .with_workspace(workspace_coordinator.clone())
    .with_environment_coordinator(environment_coordinator.clone())
    .with_model_availability(availability)
    .with_providers(Arc::new(store.providers().clone()))
    .with_credentials(credentials)
    .with_fleet_automation(Arc::new(store.fleet_automation().clone()))
    .with_oauth_callback_server(oauth_callback_server);

    let local_api = Arc::new(
        local_api
            .with_outbox(Arc::new(store.outbox().clone()))
            .with_admission_coordinator(admission_coordinator.clone()),
    );

    Ok(CoordinatorServices {
        workflow,
        lease_manager,
        workspace_coordinator,
        environment_coordinator,
        terminal_coordinator,
        python_coordinator,
        admission_coordinator,
        local_api,
    })
}
