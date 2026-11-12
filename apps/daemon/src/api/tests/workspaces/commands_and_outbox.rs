use std::sync::Arc;

use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;

use crate::api::*;

#[tokio::test]
async fn test_command_id_dedup_and_conflict_rejection() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::with_store(store.clone()));
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let dispatcher =
        LocalApiDispatcher::new(service, sessions, bridge).with_sqlite_store(store.clone());

    let cmd_id = "cmd_stable_12345";
    let req1 = ApiRequest {
        id: cmd_id.into(),
        method: "v1.tasks.create".into(),
        params: serde_json::json!({
            "title": "Idempotent Task",
            "command_id": cmd_id,
        }),
    };

    // 1. First execution
    let res1 = dispatcher.handle_request(req1.clone()).await;
    assert!(res1.is_success());
    let task1 = res1.result.unwrap();
    let task_id1 = task1["id"].as_str().unwrap().to_string();

    // 2. Exact duplicate retry with same command_id and payload
    let res2 = dispatcher.handle_request(req1).await;
    assert!(res2.is_success());
    let task2 = res2.result.unwrap();
    let task_id2 = task2["id"].as_str().unwrap().to_string();

    // Must return identical task_id, NOT create a second task!
    assert_eq!(task_id1, task_id2);

    // 3. Same command_id with DIFFERENT payload -> must be rejected with Conflict error
    let req3 = ApiRequest {
        id: "retry_conflict".into(),
        method: "v1.tasks.create".into(),
        params: serde_json::json!({
            "title": "Different Title Attack",
            "command_id": cmd_id,
        }),
    };
    let res3 = dispatcher.handle_request(req3).await;
    assert!(!res3.is_success());
    assert!(res3.error.unwrap().contains("Command ID conflict"));
}

#[tokio::test]
async fn command_admission_failure_denies_mutation_and_errors_are_replayed() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::with_store(store.clone()));
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let dispatcher =
        LocalApiDispatcher::new(service, sessions, bridge).with_sqlite_store(store.clone());
    let request = ApiRequest::new(
        "first",
        "v1.tasks.create",
        serde_json::json!({"command_id":"error_once"}),
    );
    let first = dispatcher.handle_request(request.clone()).await;
    assert!(first.error.is_some());
    let mut retry = request;
    retry.id = "retry".into();
    let replay = dispatcher.handle_request(retry).await;
    assert_eq!(first.error, replay.error);
    assert_eq!(replay.id, "retry");
    assert!(store
        .command_ledger()
        .get_command("error_once")
        .unwrap()
        .unwrap()
        .result_json
        .contains("custos_command_response"));

    store
        .db()
        .lock()
        .unwrap()
        .execute("DROP TABLE command_ledger", [])
        .unwrap();
    let denied = dispatcher
        .handle_request(ApiRequest::new(
            "denied",
            "v1.tasks.create",
            serde_json::json!({"title":"Must not exist","command_id":"deny_me"}),
        ))
        .await;
    assert!(denied.error.unwrap().contains("Command admission failed"));
    assert!(store.list_tasks().unwrap().is_empty());
}

#[tokio::test]
async fn test_compute_targets_and_experiment_lifecycle() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store.clone())
        .with_target_resolver(Arc::new(
            custos_adapters::compute::MockExecutionTargetResolver,
        ));

    // 1. List compute targets -> must return Colab, Kaggle, Rented GPU / SSH
    let targets_resp = dispatcher
        .handle_request(ApiRequest {
            id: "targets_1".into(),
            method: METHOD_COMPUTE_TARGETS_LIST.into(),
            params: serde_json::json!({}),
        })
        .await;
    assert!(targets_resp.is_success());
    let targets = targets_resp.result.unwrap();
    let target_arr = targets.as_array().unwrap();
    assert!(target_arr.iter().any(|t| t["backend"] == "colab"));
    assert!(target_arr.iter().any(|t| t["backend"] == "kaggle"));
    assert!(target_arr.iter().any(|t| t["backend"] == "runpod"));

    // 2. Preflight on Colab
    let preflight_colab = dispatcher
        .handle_request(ApiRequest {
            id: "preflight_1".into(),
            method: METHOD_EXPERIMENT_PREFLIGHT.into(),
            params: serde_json::json!({ "backend": "colab", "recipe_id": "test_recipe" }),
        })
        .await;
    assert!(preflight_colab.is_success());

    // 3. Submit experiment attempt
    let submit_resp = dispatcher
        .handle_request(ApiRequest {
            id: "submit_1".into(),
            method: METHOD_EXPERIMENT_SUBMIT.into(),
            params: serde_json::json!({
                "experiment_id": "exp_research_1",
                "recipe_id": "recipe_finetune",
                "backend": "colab",
            }),
        })
        .await;
    assert!(submit_resp.is_success(), "{:?}", submit_resp.error);
    let submit_res = submit_resp.result.unwrap();
    let attempt_id = submit_res["attempt"]["id"].as_str().unwrap().to_string();

    // 4. Observe attempt
    let observe_resp = dispatcher
        .handle_request(ApiRequest {
            id: "obs_1".into(),
            method: METHOD_EXPERIMENT_OBSERVE.into(),
            params: serde_json::json!({ "attempt_id": attempt_id }),
        })
        .await;
    assert!(observe_resp.is_success(), "{:?}", observe_resp.error);

    // 5. Stop attempt
    let stop_resp = dispatcher
        .handle_request(ApiRequest {
            id: "stop_1".into(),
            method: METHOD_EXPERIMENT_STOP.into(),
            params: serde_json::json!({ "attempt_id": attempt_id }),
        })
        .await;
    assert!(stop_resp.is_success(), "{:?}", stop_resp.error);

    // 6. Release compute
    let release_resp = dispatcher
        .handle_request(ApiRequest {
            id: "release_1".into(),
            method: METHOD_EXPERIMENT_RELEASE.into(),
            params: serde_json::json!({ "attempt_id": attempt_id }),
        })
        .await;
    assert!(release_resp.is_success(), "{:?}", release_resp.error);

    // 7. Collect artifacts
    let collect_resp = dispatcher
        .handle_request(ApiRequest {
            id: "collect_1".into(),
            method: METHOD_EXPERIMENT_COLLECT.into(),
            params: serde_json::json!({ "attempt_id": attempt_id }),
        })
        .await;
    assert!(collect_resp.is_success(), "{:?}", collect_resp.error);
}

#[cfg(feature = "custos-workbench-assistant")]
#[tokio::test]
async fn test_outbox_mcp_dispatch_flow() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let outbox_repo = Arc::new(store.outbox().clone());
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store.clone())
        .with_outbox(outbox_repo);

    // Register MCP server in federation hub
    let client = Arc::new(custos_adapters::mcp::CustosMcpClient::new("kaggle-cluster"));
    dispatcher
        .mcp_hub()
        .register_server("kaggle", client)
        .await
        .unwrap();

    // 1. Create task first
    let create_task_resp = dispatcher
        .handle_request(ApiRequest {
            id: "task_create_mcp".into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({ "title": "Run Kaggle Job" }),
        })
        .await;
    assert!(create_task_resp.is_success());
    let task_id = create_task_resp.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Enqueue outbox item targeting mcp://kaggle/run_notebook
    let create_outbox_resp = dispatcher
        .handle_request(ApiRequest {
            id: "ob_create_1".into(),
            method: METHOD_ASSISTANT_OUTBOX_CREATE.into(),
            params: serde_json::json!({
                "task_id": task_id,
                "action_type": "run_notebook",
                "recipient": "mcp://kaggle/run_notebook",
                "payload": { "notebook_id": "kernel_123", "gpu": "t4" },
            }),
        })
        .await;
    assert!(
        create_outbox_resp.is_success(),
        "{:?}",
        create_outbox_resp.error
    );
    let outbox_entry_id = create_outbox_resp.result.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let approved = dispatcher
        .handle_request(ApiRequest {
            id: "ob_approve_1".into(),
            method: METHOD_ASSISTANT_OUTBOX_APPROVE.into(),
            params: serde_json::json!({"id":outbox_entry_id}),
        })
        .await;
    assert!(approved.is_success(), "{:?}", approved.error);

    // 3. Dispatch outbox item
    let dispatch_resp = dispatcher
        .handle_request(ApiRequest {
            id: "ob_dispatch_1".into(),
            method: METHOD_ASSISTANT_OUTBOX_DISPATCH.into(),
            params: serde_json::json!({ "id": outbox_entry_id }),
        })
        .await;
    assert!(dispatch_resp.is_success(), "{:?}", dispatch_resp.error);
    let dispatch_val = dispatch_resp.result.unwrap();
    assert_eq!(dispatch_val["status"], "receipted");
    assert!(dispatch_val["receipt"]["receipt_id"].is_string());
}
