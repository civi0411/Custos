use crate::api::LocalApiDispatcher;
use custos_bridge::BridgeService;
use custos_core_domain::DomainError;
use custos_kernel::TaskService;
use custos_persistence::SqliteTaskStore;
use custos_session::SessionManager;
use std::sync::Arc;

#[allow(dead_code)]
pub struct CustosRuntime {
    pub store: Arc<SqliteTaskStore>,
    pub task_service: Arc<TaskService>,
    pub session_manager: Arc<SessionManager>,
    pub bridge_service: Arc<BridgeService>,
    pub local_api: Arc<LocalApiDispatcher>,
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

        Ok(Self {
            store,
            task_service,
            session_manager,
            bridge_service,
            local_api,
        })
    }
}
