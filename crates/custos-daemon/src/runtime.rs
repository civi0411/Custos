use crate::api::LocalApiDispatcher;
use custos_adapters::harness::ClaudeCodeHarnessAdapter;
use custos_adapters::providers::{
    AntigravityProvider, ClaudeProvider, CodexProvider, FakeProvider, LocalModelProvider,
};
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_bridge::BridgeService;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::TaskService;
use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;
use custos_provider::ModelPort;
use custos_runtime::session::SessionManager;
use custos_runtime::workflow::TaskRuntime;
use std::sync::Arc;

#[allow(dead_code)]
pub struct CustosRuntime {
    pub store: Arc<SqliteTaskStore>,
    pub task_service: Arc<TaskService>,
    pub session_manager: Arc<SessionManager>,
    pub bridge_service: Arc<BridgeService>,
    pub local_api: Arc<LocalApiDispatcher>,

    // Architecture Ports
    pub kernel: Arc<dyn KernelPort>,
    pub sandbox: Arc<dyn SandboxPort>,
    pub model: Arc<dyn ModelPort>,
    pub harness: Arc<dyn AgentRuntimePort>,
    pub workflow: Arc<dyn WorkflowPort>,
    pub lease_manager: Arc<custos_runtime::workflow::WorkspaceLeaseManager>,
    pub workspace_coordinator: Arc<custos_runtime::workspace::WorkspaceCoordinator>,
}

impl CustosRuntime {
    pub fn bootstrap_profile(profile: &crate::profile::ProfileResolver) -> Result<Self, DomainError> {
        let db_path = profile.database_path();
        Self::bootstrap(&db_path.to_string_lossy())
    }

    pub fn bootstrap(database_path: &str) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);

        // Crash Recovery Reconcile (Gate 3): transition any InFlight effects to Uncertain on startup
        let _ = store.outbox().reconcile_on_startup();
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));

        let kernel = Arc::new(TrustedKernel::new(store.clone()));

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root.clone()));

        // Dynamic provider selection from CUSTOS_PROVIDER env, falling back to FakeProvider
        let provider_name = std::env::var("CUSTOS_PROVIDER").unwrap_or_else(|_| "fake".into());
        let model: Arc<dyn ModelPort> = match provider_name.to_lowercase().as_str() {
            "claude" | "anthropic" => Arc::new(ClaudeProvider::new()),
            "codex" | "openai" => Arc::new(CodexProvider::new()),
            "antigravity" | "gemini" => Arc::new(AntigravityProvider::new()),
            "local" => Arc::new(LocalModelProvider::new()),
            _ => Arc::new(FakeProvider::new("fake")),
        };

        // Sovereign Coding Harness Adapter (Claude Code CLI / sub-process agent runtime)
        let lease_manager = Arc::new(custos_runtime::workflow::WorkspaceLeaseManager::new(
            workspace_root.clone(),
        ));

        let harness: Arc<dyn AgentRuntimePort> =
            Arc::new(ClaudeCodeHarnessAdapter::new(workspace_root));

        let workflow = Arc::new(
            TaskRuntime::new()
                .with_kernel(kernel.clone())
                .with_model(model.clone())
                .with_harness(harness.clone())
                .with_sandbox(sandbox.clone())
                .with_outbox(store.clone())
                .with_effect_ledger(store.clone())
                .with_run_store(store.clone())
                .with_lease_manager(lease_manager.clone()),
        );

        let workspace_provider = Arc::new(custos_adapters::workspace::LocalWorkspaceProvider::new());
        let workspace_coordinator = Arc::new(custos_runtime::workspace::WorkspaceCoordinator::new(
            store.clone(),
            workspace_provider,
        ));

        let local_api = Arc::new(
            LocalApiDispatcher::new(
                task_service.clone(),
                session_manager.clone(),
                bridge_service.clone(),
            )
            .with_workflow(workflow.clone())
            .with_workspace(workspace_coordinator.clone()),
        );

        Ok(Self {
            store,
            task_service,
            session_manager,
            bridge_service,
            local_api,
            kernel,
            sandbox,
            model,
            harness,
            workflow,
            lease_manager,
            workspace_coordinator,
        })
    }
}
