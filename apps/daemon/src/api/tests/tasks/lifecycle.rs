use crate::api::tests::harness::*;
use crate::api::*;
use custos_core::TaskService;
use custos_domain::*;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_local_api_dispatcher_create_and_get() {
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

    // 1. Create task via API
    let create_req = ApiRequest {
        id: "req_1".into(),
        method: "v1.tasks.create".into(),
        params: serde_json::json!({
            "title": "Build Local API",
            "metadata": {"source": "cli"}
        }),
    };

    let create_res = dispatcher.handle_request(create_req).await;
    assert!(create_res.error.is_none());
    let created_task: Task = serde_json::from_value(create_res.result.unwrap()).unwrap();
    assert_eq!(created_task.title, "Build Local API");
    assert_eq!(created_task.status, TaskStatus::Draft);

    // 2. Get task via API
    let get_req = ApiRequest {
        id: "req_2".into(),
        method: "v1.tasks.get".into(),
        params: serde_json::json!({
            "task_id": created_task.id
        }),
    };

    let get_res = dispatcher.handle_request(get_req).await;
    assert!(get_res.error.is_none());
    let fetched_task: Task = serde_json::from_value(get_res.result.unwrap()).unwrap();
    assert_eq!(fetched_task.id, created_task.id);
    assert_eq!(fetched_task.title, "Build Local API");

    // 3. Advance task to Queued via API
    let advance_req = ApiRequest {
        id: "req_adv".into(),
        method: "v1.tasks.advance".into(),
        params: serde_json::json!({
            "task_id": created_task.id,
            "target_status": "queued"
        }),
    };
    let adv_res = dispatcher.handle_request(advance_req).await;
    assert!(adv_res.error.is_none());
    let advanced_task: Task = serde_json::from_value(adv_res.result.unwrap()).unwrap();
    assert_eq!(advanced_task.status, TaskStatus::Queued);

    // 3b. Verify advancing directly to Succeeded is REJECTED
    let bad_advance_req = ApiRequest {
        id: "req_bad_adv".into(),
        method: "v1.tasks.advance".into(),
        params: serde_json::json!({
            "task_id": created_task.id,
            "target_status": "succeeded"
        }),
    };
    let bad_adv_res = dispatcher.handle_request(bad_advance_req).await;
    assert!(bad_adv_res.error.is_some());
    assert!(bad_adv_res
        .error
        .unwrap()
        .contains("Cannot advance directly to Succeeded"));

    // 3c. Advance to Running
    let run_req = ApiRequest {
        id: "req_run".into(),
        method: "v1.tasks.advance".into(),
        params: serde_json::json!({
            "task_id": created_task.id,
            "target_status": "running"
        }),
    };
    let run_res = dispatcher.handle_request(run_req).await;
    assert!(run_res.error.is_none());

    // 3d. Complete task via v1.tasks.complete
    let complete_req = ApiRequest {
        id: "req_comp".into(),
        method: "v1.tasks.complete".into(),
        params: serde_json::json!({
            "task_id": created_task.id,
            "summary": "Finished with proof closure"
        }),
    };
    let comp_res = dispatcher.handle_request(complete_req).await;
    assert!(comp_res.error.is_none());
    let completed_task: Task = serde_json::from_value(comp_res.result.unwrap()).unwrap();
    assert_eq!(completed_task.status, TaskStatus::Succeeded);

    // 3e. List tasks via API
    let list_req = ApiRequest {
        id: "req_list".into(),
        method: "v1.tasks.list".into(),
        params: serde_json::json!({}),
    };
    let list_res = dispatcher.handle_request(list_req).await;
    assert!(list_res.error.is_none());
    let tasks_list: Vec<Task> = serde_json::from_value(list_res.result.unwrap()).unwrap();
    assert_eq!(tasks_list.len(), 1);
    assert_eq!(tasks_list[0].id, created_task.id);

    // 4. Create and Get session via API
    let session_create_req = ApiRequest {
        id: "req_sess".into(),
        method: "v1.sessions.create".into(),
        params: serde_json::json!({"mode": "bare"}),
    };
    let session_res = dispatcher.handle_request(session_create_req).await;
    assert!(session_res.error.is_none());
    let created_session: Session = serde_json::from_value(session_res.result.unwrap()).unwrap();

    let delete_session_req = ApiRequest {
        id: "req_sess_delete".into(),
        method: METHOD_SESSIONS_DELETE.into(),
        params: serde_json::json!({ "session_id": created_session.id.0 }),
    };
    let delete_session_res = dispatcher.handle_request(delete_session_req).await;
    assert!(delete_session_res.error.is_none());
    assert_eq!(
        delete_session_res
            .result
            .unwrap()
            .get("deleted")
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    // 5. Unknown method
    let invalid_req = ApiRequest {
        id: "req_3".into(),
        method: "v1.unknown.action".into(),
        params: serde_json::json!({}),
    };
    let invalid_res = dispatcher.handle_request(invalid_req).await;
    assert!(invalid_res.error.is_some());
    assert!(invalid_res.error.unwrap().contains("Unknown method"));
}

#[tokio::test]
async fn test_proof_closure_evidence_gating_via_local_api() {
    use custos_domain::{ContractEvidence, EvidenceKind, TaskContract, VerificationClaim};

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

    // 1. Create a task with strict contract requiring TestResult and Diff evidence
    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: "Verified Fix".into(),
        description: "Must pass tests and produce verified patch diff".into(),
        required_capabilities: vec![],
        evidence_requirements: vec![
            ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            },
            ContractEvidence {
                kind: EvidenceKind::Diff,
                required: true,
            },
        ],
    };

    let create_req = ApiRequest {
        id: "req_c1".into(),
        method: "v1.tasks.create".into(),
        params: serde_json::json!({
            "title": "Fix memory leak",
            "contract": contract,
        }),
    };

    let create_res = dispatcher.handle_request(create_req).await;
    assert!(create_res.error.is_none());
    let task: Task = serde_json::from_value(create_res.result.unwrap()).unwrap();

    // 2. Advance Draft -> Queued -> Running
    let adv1 = dispatcher
        .handle_request(ApiRequest {
            id: "adv_1".into(),
            method: "v1.tasks.advance".into(),
            params: serde_json::json!({ "task_id": task.id, "target_status": "queued" }),
        })
        .await;
    assert!(adv1.error.is_none());

    let adv2 = dispatcher
        .handle_request(ApiRequest {
            id: "adv_2".into(),
            method: "v1.tasks.advance".into(),
            params: serde_json::json!({ "task_id": task.id, "target_status": "running" }),
        })
        .await;
    assert!(adv2.error.is_none());

    // 3. Attempt completion WITHOUT claims -> MUST FAIL (Proof-closure violation)
    let fail_comp1 = dispatcher
        .handle_request(ApiRequest {
            id: "comp_fail1".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "summary": "Try complete without proof",
                "evidence_claims": []
            }),
        })
        .await;
    assert!(fail_comp1.error.is_some());
    assert!(fail_comp1
        .error
        .unwrap()
        .contains("Proof-closure violation"));

    // 4. Attempt completion with failing claim -> MUST FAIL
    let failing_claim = VerificationClaim::new(
        task.id.clone(),
        "Tests failed".into(),
        "command_exit_code".into(),
        false,
        serde_json::json!({"exit_code": 1}),
    );
    let fail_comp2 = dispatcher
        .handle_request(ApiRequest {
            id: "comp_fail2".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "summary": "Try complete with failing claim",
                "evidence_claims": [failing_claim]
            }),
        })
        .await;
    assert!(fail_comp2.error.is_some());
    assert!(fail_comp2
        .error
        .unwrap()
        .contains("Proof-closure violation"));

    // 4b. Attempt completion with foreign task claim -> MUST FAIL
    let foreign_claim = VerificationClaim::new(
        "other_task_999".into(),
        "Unit tests passed".into(),
        "command_exit_code".into(),
        true,
        serde_json::json!({"exit_code": 0}),
    );
    let fail_foreign = dispatcher
        .handle_request(ApiRequest {
            id: "comp_foreign".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "summary": "Try complete with foreign claim",
                "evidence_claims": [foreign_claim]
            }),
        })
        .await;
    assert!(fail_foreign.error.is_some());
    assert!(fail_foreign
        .error
        .unwrap()
        .contains("foreign task"));

    // 4c. Attempt completion with empty verifier_id -> MUST FAIL
    let mut invalid_claim = VerificationClaim::new(
        task.id.clone(),
        "Fabricated claim".into(),
        "".into(),
        true,
        serde_json::json!({}),
    );
    invalid_claim.verifier_id = "".into();
    let fail_invalid = dispatcher
        .handle_request(ApiRequest {
            id: "comp_invalid".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "summary": "Try complete with invalid claim",
                "evidence_claims": [invalid_claim]
            }),
        })
        .await;
    assert!(fail_invalid.error.is_some());
    assert!(fail_invalid
        .error
        .unwrap()
        .contains("missing verifier_id"));

    // 5. Complete WITH passing claims for all required evidence -> MUST SUCCEED
    let test_claim = VerificationClaim::new(
        task.id.clone(),
        "Unit tests passed with exit code 0".into(),
        "command_exit_code".into(),
        true,
        serde_json::json!({"exit_code": 0}),
    );
    let diff_claim = VerificationClaim::new(
        task.id.clone(),
        "Simulated patch preview generated".into(),
        "patch_preview".into(),
        true,
        serde_json::json!({"simulated": true, "diff_digest": "sha256:abc12345"}),
    );

    let success_comp = dispatcher
        .handle_request(ApiRequest {
            id: "comp_ok".into(),
            method: "v1.tasks.complete".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "summary": "All proof closed successfully",
                "evidence_claims": [test_claim, diff_claim]
            }),
        })
        .await;
    assert!(
        success_comp.error.is_none(),
        "Error: {:?}",
        success_comp.error
    );
    let completed: Task = serde_json::from_value(success_comp.result.unwrap()).unwrap();
    assert_eq!(completed.status, TaskStatus::Succeeded);
}
