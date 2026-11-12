use crate::api::LocalApiDispatcher;
pub use crate::credential_resolver::*;
pub use crate::tool_executor::*;
use custos_adapters::harness::HarnessRegistry;
use custos_provider::routing::router::RouterModelProvider;
use custos_runtime::bridge::BridgeService;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::TaskService;
use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;
use custos_provider::ModelPort;
use custos_runtime::session::SessionManager;
use std::sync::Arc;

#[allow(dead_code)]
pub struct CustosRuntime {
    pub store: Arc<SqliteTaskStore>,
    pub credentials: Arc<crate::credentials::ProviderCredentials>,
    pub task_service: Arc<TaskService>,
    pub session_manager: Arc<SessionManager>,
    pub bridge_service: Arc<BridgeService>,
    pub local_api: Arc<LocalApiDispatcher>,

    // Architecture Ports
    pub kernel: Arc<dyn KernelPort>,
    pub sandbox: Arc<dyn SandboxPort>,
    pub model: Arc<dyn ModelPort>,
    pub harness: Arc<dyn AgentRuntimePort>,
    pub harnesses: Arc<HarnessRegistry>,
    pub workflow: Arc<dyn WorkflowPort>,
    pub lease_manager: Arc<custos_runtime::workspace::WorkspaceLeaseManager>,
    pub workspace_coordinator: Arc<custos_runtime::workspace::WorkspaceCoordinator>,
    pub terminal_coordinator: Arc<custos_runtime::TerminalCoordinator>,
    pub python_coordinator: Option<Arc<custos_runtime::PythonKernelCoordinator>>,
    pub pack_registry: Arc<custos_packs::DomainPackRegistry>,
}

impl CustosRuntime {
    pub fn bootstrap_profile(
        profile: &crate::profile::ProfileResolver,
    ) -> Result<Self, DomainError> {
        let db_path = profile.database_path();
        Self::bootstrap(&db_path.to_string_lossy())
    }

    pub fn bootstrap(database_path: &str) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);

        // Multi-provider router: dynamically dispatches to OpenAI, Anthropic, Gemini, DeepSeek, or Ollama/local
        let credentials = Self::credential_service(&store);
        let availability = custos_provider::routing::availability::ModelAvailability::default();
        let credential_resolver = Arc::new(DaemonCredentialResolver {
            availability: availability.clone(),
            store: store.clone(),
            credentials: credentials.clone(),
        });

        let local_adapter = match custos_provider::local::InferenceRuntime::get_or_init()
        {
            Ok(runtime) => {
                let local_provider = Arc::new(
                    custos_provider::local::LocalInferenceProvider::new(runtime),
                );
                Some(Arc::new(
                    custos_provider::local::NativeLocalModelAdapter::new(local_provider),
                ) as Arc<dyn ModelPort>)
            }
            Err(err) => {
                tracing::warn!(error = %err, "Local inference engine not available on this platform");
                None
            }
        };

        let mut router = RouterModelProvider::new(credential_resolver);
        if let Some(adapter) = local_adapter {
            router = router.with_native_local(adapter);
        }
        let router_provider = Arc::new(router);

        // Production inference always goes through the configured provider router.
        // Deterministic fake providers remain test fixtures and cannot be enabled
        // by an environment variable in a user daemon.
        let model: Arc<dyn ModelPort> = router_provider.clone();

        Self::bootstrap_with_store_and_model(store, model, credentials, availability)
    }

    pub fn bootstrap_with_model(
        database_path: &str,
        model: Arc<dyn ModelPort>,
    ) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);
        let credentials = Self::credential_service(&store);
        Self::bootstrap_with_store_and_model(store, model, credentials, Default::default())
    }

    pub(crate) fn credential_service(
        store: &Arc<SqliteTaskStore>,
    ) -> Arc<crate::credentials::ProviderCredentials> {
        let repo = Arc::new(store.providers().clone());
        #[cfg(test)]
        {
            crate::credential_tests::fake_credentials(repo)
        }
        #[cfg(not(test))]
        {
            let vault: Arc<dyn custos_core::contracts::credentials::CredentialVault> =
                if std::env::var("CUSTOS_VAULT_BACKEND").as_deref() == Ok("keyring") {
                    Arc::new(custos_adapters::host::vault::KeyringCredentialVault)
                } else {
                    Arc::new(custos_adapters::host::vault::FileCredentialVault::new(
                        custos_adapters::host::vault::FileCredentialVault::default_dir(),
                    ))
                };
            Arc::new(crate::credentials::ProviderCredentials::new(repo, vault))
        }
    }

    fn bootstrap_with_store_and_model(
        store: Arc<SqliteTaskStore>,
        model: Arc<dyn ModelPort>,
        credentials: Arc<crate::credentials::ProviderCredentials>,
        availability: custos_provider::routing::availability::ModelAvailability,
    ) -> Result<Self, DomainError> {
        // Phase 1: Storage Crash Recovery and Reconciliation (Gate 3)
        crate::bootstrap::reconcile_storage_on_startup(&store)?;

        // Phase 2: Background OAuth Sweeper and Uncertain Runs Reconciler
        crate::bootstrap::spawn_background_sweepers(credentials.clone(), store.clone());

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| ".".into());

        // Phase 3: Kernel and Isolation Services
        let kernel_services = crate::bootstrap::init_kernel_services(store.clone(), &workspace_root);

        // Phase 4: Workbenches, Packs, and Governed Harness Setup
        let wb_setup = crate::bootstrap::init_workbenches_and_harness(
            &store,
            &workspace_root,
            kernel_services.kernel.clone(),
            model.clone(),
            kernel_services.sandbox.clone(),
            Some(kernel_services.supervision.clone()),
        );

        // Phase 5: Coordinators, TaskRuntime, and LocalApiDispatcher
        let coords = crate::bootstrap::init_coordinators_and_api(
            store.clone(),
            credentials.clone(),
            kernel_services.kernel.clone(),
            model.clone(),
            kernel_services.sandbox.clone(),
            kernel_services.task_service.clone(),
            kernel_services.session_manager.clone(),
            kernel_services.bridge_service.clone(),
            wb_setup.harnesses.clone(),
            wb_setup.pack_registry.clone(),
            wb_setup.mcp_hub,
            &workspace_root,
            availability,
            kernel_services.live_runs,
            kernel_services.supervision,
        )?;

        Ok(Self {
            store,
            credentials,
            task_service: kernel_services.task_service,
            session_manager: kernel_services.session_manager,
            bridge_service: kernel_services.bridge_service,
            local_api: coords.local_api,
            kernel: kernel_services.kernel,
            sandbox: kernel_services.sandbox,
            model,
            harness: wb_setup.harness,
            harnesses: wb_setup.harnesses,
            workflow: coords.workflow,
            lease_manager: coords.lease_manager,
            workspace_coordinator: coords.workspace_coordinator,
            terminal_coordinator: coords.terminal_coordinator,
            python_coordinator: coords.python_coordinator,
            pack_registry: wb_setup.pack_registry,
        })
    }
}
