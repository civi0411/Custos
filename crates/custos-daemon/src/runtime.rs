use crate::api::LocalApiDispatcher;
use custos_bridge::BridgeService;
use custos_domain::DomainError;
use custos_core::TaskService;
use custos_persistence::SqliteTaskStore;
use custos_runtime::session::SessionManager;
use std::sync::Arc;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::workflow::WorkflowPort;
use custos_provider::ModelPort;
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_adapters::providers::fake::FakeProvider;
use custos_runtime::workflow::TaskRuntime;

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
    pub workflow: Arc<dyn WorkflowPort>,
}

impl CustosRuntime {
    pub fn bootstrap(database_path: &str) -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new(database_path)?);
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let local_api = Arc::new(LocalApiDispatcher::new(
            task_service.clone(),
            session_manager.clone(),
            bridge_service.clone(),
        ));

        let kernel = Arc::new(TrustedKernel::new(store.clone()));
        
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root));
        
        let model = Arc::new(FakeProvider::new("fake"));
        
        let workflow = Arc::new(TaskRuntime::new());

        Ok(Self {
            store,
            task_service,
            session_manager,
            bridge_service,
            local_api,
            kernel,
            sandbox,
            model,
            workflow,
        })
    }
}
