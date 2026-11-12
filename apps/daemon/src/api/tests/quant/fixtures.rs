use super::super::super::*;
use custos_core::{CreateTask, TaskService};
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use std::sync::Arc;

pub async fn setup_quant_env() -> (LocalApiDispatcher, String) {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    store
        .packages()
        .commit_installed("quant", "0.1.0", "any", None, "packages/quant/0.1.0")
        .unwrap();
    let service = Arc::new(TaskService::new(store.clone()));
    let task = service
        .execute_create(CreateTask {
            title: "Quant Pipeline Test Task".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;

    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let outbox = Arc::new(store.outbox().clone());
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store)
        .with_outbox(outbox);

    (dispatcher, task.id)
}
