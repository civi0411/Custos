use std::sync::Arc;

use custos_adapters::workspace::LocalWorkspaceProvider;
use custos_core::TaskService;
use custos_domain::environment::{EnvironmentProfile, EnvironmentSpec};
use custos_persistence::SqliteTaskStore;
use custos_runtime::bridge::BridgeService;
use custos_runtime::environment::EnvironmentCoordinator;
use custos_runtime::session::SessionManager;
use custos_runtime::workspace::WorkspaceCoordinator;

use crate::api::*;

#[tokio::test]
async fn test_environment_api_dispatch_lifecycle() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let workspace_provider = Arc::new(LocalWorkspaceProvider::new());
    let workspace_coordinator =
        Arc::new(WorkspaceCoordinator::new(store.clone(), workspace_provider));
    let env_coordinator =
        Arc::new(EnvironmentCoordinator::new(workspace_coordinator.clone(), None));

    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_workspace(workspace_coordinator)
        .with_environment_coordinator(env_coordinator);

    // 1. Initially empty list
    let list_req = ApiRequest {
        id: "req_list_1".into(),
        method: "v1.environment.list".into(),
        params: serde_json::json!({}),
    };
    let list_res = dispatcher.handle_request(list_req).await;
    assert!(list_res.error.is_none());
    assert_eq!(list_res.result.unwrap().as_array().unwrap().len(), 0);

    // 2. Provision Assistant environment
    let spec = EnvironmentSpec::new(
        "env_api_01",
        EnvironmentProfile::Assistant {
            project_id: Some("proj_test".into()),
        },
    );
    let prov_req = ApiRequest {
        id: "req_prov".into(),
        method: "v1.environment.provision".into(),
        params: serde_json::to_value(&spec).unwrap(),
    };
    let prov_res = dispatcher.handle_request(prov_req).await;
    assert!(prov_res.error.is_none(), "Provision error: {:?}", prov_res.error);
    let handle_val = prov_res.result.unwrap();
    assert_eq!(handle_val["env_id"], "env_api_01");
    assert_eq!(handle_val["state"]["status"], "ready");

    // 3. Get environment
    let get_req = ApiRequest {
        id: "req_get".into(),
        method: "v1.environment.get".into(),
        params: serde_json::json!({ "env_id": "env_api_01" }),
    };
    let get_res = dispatcher.handle_request(get_req).await;
    assert!(get_res.error.is_none());
    assert_eq!(get_res.result.unwrap()["env_id"], "env_api_01");

    // 4. Verify environment
    let verify_req = ApiRequest {
        id: "req_verify".into(),
        method: "v1.environment.verify".into(),
        params: serde_json::json!({
            "env_id": "env_api_01",
            "spec": serde_json::to_value(&spec).unwrap(),
        }),
    };
    let verify_res = dispatcher.handle_request(verify_req).await;
    assert!(verify_res.error.is_none());
    assert_eq!(verify_res.result.unwrap()["passed"], true);

    // 5. Teardown environment
    let teardown_req = ApiRequest {
        id: "req_teardown".into(),
        method: "v1.environment.teardown".into(),
        params: serde_json::json!({ "env_id": "env_api_01" }),
    };
    let teardown_res = dispatcher.handle_request(teardown_req).await;
    assert!(teardown_res.error.is_none());
    assert_eq!(teardown_res.result.unwrap()["status"], "torn_down");

    // 6. After teardown, list is empty again
    let list_req_2 = ApiRequest {
        id: "req_list_2".into(),
        method: "v1.environment.list".into(),
        params: serde_json::json!({}),
    };
    let list_res_2 = dispatcher.handle_request(list_req_2).await;
    assert_eq!(list_res_2.result.unwrap().as_array().unwrap().len(), 0);
}

