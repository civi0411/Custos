use crate::api::tests::harness::*;
use crate::api::*;
use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_workflow_start_and_cancel_run_api() {
    let store = Arc::new(MockStore {
        tasks: Mutex::new(Vec::new()),
    });
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let workflow = Arc::new(custos_runtime::workflow::TaskRuntime::new());

    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
        .with_workflow(workflow);

    // 1. Start run via API
    let start_req = ApiRequest {
        id: "req_run_1".into(),
        method: "v1.workflow.start_run".into(),
        params: serde_json::json!({
            "task_id": "task_api_workflow_1",
            "actor": "tester",
            "preferred_mode": "model"
        }),
    };
    let start_resp = dispatcher.handle_request(start_req).await;
    assert!(start_resp.error.is_none(), "Error: {:?}", start_resp.error);
    let handle: custos_domain::RunHandle =
        serde_json::from_value(start_resp.result.unwrap()).unwrap();
    assert_eq!(handle.task_id, "task_api_workflow_1");
    assert_eq!(handle.status, custos_domain::RunStatus::Active);

    // 2. Double-dispatch via API on same task must fail with Conflict
    let start_dup_req = ApiRequest {
        id: "req_run_dup".into(),
        method: "v1.workflow.start_run".into(),
        params: serde_json::json!({
            "task_id": "task_api_workflow_1",
            "actor": "second_caller"
        }),
    };
    let dup_resp = dispatcher.handle_request(start_dup_req).await;
    assert!(dup_resp.error.is_some());
    assert!(dup_resp.error.unwrap().contains("already claimed"));

    // 3. Cancel run via API
    let cancel_req = ApiRequest {
        id: "req_cancel_1".into(),
        method: "v1.workflow.cancel_run".into(),
        params: serde_json::json!({
            "run_id": handle.run_id,
            "reason": "Test cancel via API"
        }),
    };
    let cancel_resp = dispatcher.handle_request(cancel_req).await;
    assert!(
        cancel_resp.error.is_none(),
        "Error: {:?}",
        cancel_resp.error
    );
    let receipt: custos_domain::CancelReceipt =
        serde_json::from_value(cancel_resp.result.unwrap()).unwrap();
    assert_eq!(receipt.run_id, handle.run_id);
    assert_eq!(receipt.reason, "Test cancel via API");

    // 4. Start run with unverified session_id must be refused by admission gate
    let invalid_sess_req = ApiRequest {
        id: "req_invalid_sess".into(),
        method: "v1.workflow.start_run".into(),
        params: serde_json::json!({
            "task_id": "task_api_workflow_invalid_sess",
            "session_id": "non_existent_sess_999",
            "actor": "tester"
        }),
    };
    let invalid_resp = dispatcher.handle_request(invalid_sess_req).await;
    assert!(invalid_resp.error.is_some());
    assert!(invalid_resp
        .error
        .unwrap()
        .contains("Admission gate refused unverified session_id"));

    // 5. Start run with privileged actor override (system/kernel/root) must be refused
    let priv_actor_req = ApiRequest {
        id: "req_priv_actor".into(),
        method: "v1.workflow.start_run".into(),
        params: serde_json::json!({
            "task_id": "task_api_workflow_priv",
            "actor": "system"
        }),
    };
    let priv_resp = dispatcher.handle_request(priv_actor_req).await;
    assert!(priv_resp.error.is_some());
    assert!(priv_resp
        .error
        .unwrap()
        .contains("Admission gate refused privileged actor override"));

    // 6. Start run with invalid actor characters must be refused
    let bad_actor_req = ApiRequest {
        id: "req_bad_actor".into(),
        method: "v1.workflow.start_run".into(),
        params: serde_json::json!({
            "task_id": "task_api_workflow_bad_actor",
            "actor": "bad actor; rm -rf /"
        }),
    };
    let bad_actor_resp = dispatcher.handle_request(bad_actor_req).await;
    assert!(bad_actor_resp.error.is_some());
    assert!(bad_actor_resp
        .error
        .unwrap()
        .contains("Admission gate refused invalid actor identifier"));
}

#[tokio::test]
async fn test_capabilities_api_dispatch() {
    let store = Arc::new(MockStore {
        tasks: Mutex::new(Vec::new()),
    });
    let task_service = Arc::new(TaskService::new(store));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service);

    // 1. List all capabilities
    let list_resp = dispatcher
        .handle_request(ApiRequest {
            id: "req_caps_1".into(),
            method: METHOD_CAPABILITIES_LIST.into(),
            params: serde_json::Value::Null,
        })
        .await;
    assert!(list_resp.is_success());

    let caps: Vec<custos_domain::CapabilityDescriptor> =
        serde_json::from_value(list_resp.result.unwrap()).unwrap();
    assert!(!caps.is_empty());

    let mut projected_resources = std::collections::HashSet::new();
    for capability in &caps {
        if let Some(resource_id) = capability.resource_id.as_deref() {
            assert!(
                projected_resources.insert(resource_id),
                "resource_id '{resource_id}' must map to exactly one capability"
            );
        }
    }

    let research_runs = caps.iter().find(|c| c.id == "research.runs").unwrap();
    assert_eq!(research_runs.resource_id.as_deref(), Some("runs"));
    let synthesis = caps.iter().find(|c| c.id == "synthesis.handoff").unwrap();
    assert_eq!(synthesis.resource_id.as_deref(), Some("synthesis"));

    // Kernel task engine is available
    let tasks_cap = caps.iter().find(|c| c.id == "tasks.core").unwrap();
    assert!(tasks_cap.status.is_available());

    // A dispatcher without a workspace coordinator cannot offer scoped tools.
    let pty_cap = caps.iter().find(|c| c.id == "compute.pty").unwrap();
    assert!(!pty_cap.status.is_available());
    assert_eq!(pty_cap.resource_id.as_deref(), Some("terminal"));
    assert!(pty_cap.supported_operations.is_empty());

    let files_cap = caps.iter().find(|c| c.id == "code.files").unwrap();
    assert!(!files_cap.status.is_available());
    assert!(files_cap.supported_operations.is_empty());

    let changes_cap = caps.iter().find(|c| c.id == "code.changes").unwrap();
    assert!(!changes_cap.status.is_available());
    assert!(changes_cap.supported_operations.is_empty());

    let outbox_actions = dispatcher
        .handle_request(ApiRequest {
            id: "req_outbox_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "outbox" }),
        })
        .await;
    assert!(outbox_actions.is_success());
    let actions: Vec<ResourceActionCapability> =
        serde_json::from_value(outbox_actions.result.unwrap()).unwrap();
    assert!(
        actions.is_empty(),
        "unconfigured outbox must not advertise dispatch"
    );

    let unknown_actions = dispatcher
        .handle_request(ApiRequest {
            id: "req_unknown_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "made_up" }),
        })
        .await;
    assert!(!unknown_actions.is_success());

    // Notebook capability is truthfully available when PythonKernelCoordinator is configured
    let notebook_cap = caps.iter().find(|c| c.id == "compute.notebook").unwrap();
    assert!(notebook_cap.status.is_available());
    assert!(notebook_cap
        .supported_operations
        .contains(&"execute".to_string()));
    assert!(notebook_cap
        .supported_operations
        .contains(&"reset".to_string()));

    // Evidence criteria verifier capability is truthfully unavailable without SharedCapabilitiesRepository
    let evidence_cap = caps.iter().find(|c| c.id == "evidence.criteria").unwrap();
    assert!(!evidence_cap.status.is_available());
    assert_eq!(
        evidence_cap.status.reason(),
        Some("SharedCapabilitiesRepository is not configured on this daemon instance.")
    );

    // Synthesis handoff capability is truthfully unavailable without ResearchRepository
    let synth_cap = caps.iter().find(|c| c.id == "synthesis.handoff").unwrap();
    assert!(!synth_cap.status.is_available());
    assert_eq!(
        synth_cap.status.reason(),
        Some("ResearchRepository is not configured on this daemon instance.")
    );

    // 2. Query capability by capability_id
    let get_resp = dispatcher
        .handle_request(ApiRequest {
            id: "req_caps_2".into(),
            method: METHOD_CAPABILITIES_GET.into(),
            params: serde_json::json!({ "capability_id": "compute.pty" }),
        })
        .await;
    assert!(get_resp.is_success());
    let cap: custos_domain::CapabilityDescriptor =
        serde_json::from_value(get_resp.result.unwrap()).unwrap();
    assert_eq!(cap.id, "compute.pty");

    // 3. Query capability by resource_id
    let get_res_resp = dispatcher
        .handle_request(ApiRequest {
            id: "req_caps_3".into(),
            method: METHOD_CAPABILITIES_GET.into(),
            params: serde_json::json!({ "resource_id": "terminal" }),
        })
        .await;
    assert!(get_res_resp.is_success());
    let res_cap: custos_domain::CapabilityDescriptor =
        serde_json::from_value(get_res_resp.result.unwrap()).unwrap();
    assert_eq!(res_cap.id, "compute.pty");

    // 4. Missing capability returns error
    let missing_resp = dispatcher
        .handle_request(ApiRequest {
            id: "req_caps_4".into(),
            method: METHOD_CAPABILITIES_GET.into(),
            params: serde_json::json!({ "capability_id": "nonexistent" }),
        })
        .await;
    assert!(missing_resp.error.is_some());
}
