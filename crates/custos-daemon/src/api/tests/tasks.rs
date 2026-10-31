use super::super::*;
use super::harness::*;
use std::sync::{Arc, Mutex};
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
use custos_domain::*;
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

        let outbox_actions = dispatcher.handle_request(ApiRequest {
            id: "req_outbox_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "outbox" }),
        }).await;
        assert!(outbox_actions.is_success());
        let actions: Vec<ResourceActionCapability> =
            serde_json::from_value(outbox_actions.result.unwrap()).unwrap();
        assert!(actions.is_empty(), "unconfigured outbox must not advertise dispatch");

        let unknown_actions = dispatcher.handle_request(ApiRequest {
            id: "req_unknown_actions".into(),
            method: METHOD_RESOURCES_CAPABILITIES.into(),
            params: serde_json::json!({ "resource_id": "made_up" }),
        }).await;
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

        // Evidence criteria verifier capability is truthfully unavailable without ResearchRepository
        let evidence_cap = caps.iter().find(|c| c.id == "evidence.criteria").unwrap();
        assert!(!evidence_cap.status.is_available());
        assert_eq!(
            evidence_cap.status.reason(),
            Some("ResearchRepository is not configured on this daemon instance.")
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


    #[tokio::test]
    async fn test_packs_api_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("custos_packs_test.db");
        let store =
            Arc::new(custos_persistence::SqliteTaskStore::new(&db_path.to_string_lossy()).unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::with_store(store.clone()));
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let pack_registry = Arc::new(custos_packs::create_default_pack_registry());
        let dispatcher = LocalApiDispatcher::new(task_service.clone(), session_manager.clone(), bridge_service)
            .with_packs(pack_registry);

        // 1. List Pack Manifests via v1.packs.manifests.list
        let man_req = ApiRequest {
            id: "pack_1".into(),
            method: METHOD_PACKS_MANIFESTS_LIST.into(),
            params: serde_json::json!({}),
        };
        let man_res = dispatcher.handle_request(man_req).await;
        assert!(man_res.is_success(), "man_res failed: {:?}", man_res.error);
        let manifests: Vec<custos_packs::DomainPackManifest> =
            serde_json::from_value(man_res.result.unwrap()).unwrap();
        assert_eq!(manifests.len(), 3);
        assert!(manifests.iter().any(|m| m.id == "engineering"));
        assert!(manifests.iter().any(|m| m.id == "research"));
        assert!(manifests.iter().any(|m| m.id == "assistant"));

        // 2. List Jobs (All 20 Jobs) via v1.packs.jobs.list
        let jobs_req = ApiRequest {
            id: "pack_2".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({}),
        };
        let jobs_res = dispatcher.handle_request(jobs_req).await;
        assert!(jobs_res.is_success());
        let catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(jobs_res.result.unwrap()).unwrap();
        assert_eq!(catalog.len(), 20, "Expected 20 standardized Workbench Jobs");

        // Verify status mapping:
        let repo_explain = catalog.iter().find(|j| j.job_id == "engineering__repo_explain").unwrap();
        assert_eq!(repo_explain.status, JobOperationalStatus::Available);
        assert!(repo_explain.degradation_reason.is_none());

        let source_qa = catalog.iter().find(|j| j.job_id == "research__source_qa").unwrap();
        assert_eq!(source_qa.status, JobOperationalStatus::Available);

        let inbox_triage = catalog.iter().find(|j| j.job_id == "assistant__inbox_triage").unwrap();
        assert_eq!(inbox_triage.status, JobOperationalStatus::Available);

        let bugfix = catalog.iter().find(|j| j.job_id == "engineering__bugfix").unwrap();
        assert_eq!(bugfix.status, JobOperationalStatus::Degraded);
        assert!(bugfix.degradation_reason.is_some());

        let migration = catalog.iter().find(|j| j.job_id == "engineering__migration").unwrap();
        assert_eq!(migration.status, JobOperationalStatus::Planned);

        // 2b. Filter Jobs by Lens
        let coding_req = ApiRequest {
            id: "pack_2b".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({ "lens": "coding" }),
        };
        let coding_res = dispatcher.handle_request(coding_req).await;
        assert!(coding_res.is_success());
        let coding_catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(coding_res.result.unwrap()).unwrap();
        assert_eq!(coding_catalog.len(), 7, "Expected 7 Engineering Jobs in coding lens");

        let research_req = ApiRequest {
            id: "pack_2c".into(),
            method: METHOD_PACKS_JOBS_LIST.into(),
            params: serde_json::json!({ "lens": "research" }),
        };
        let research_res = dispatcher.handle_request(research_req).await;
        let research_catalog: Vec<PacksJobCatalogItem> =
            serde_json::from_value(research_res.result.unwrap()).unwrap();
        assert_eq!(research_catalog.len(), 8, "Expected 8 Research Jobs in research lens");

        // 3. Get Specific Job via v1.packs.jobs.get
        let get_req = ApiRequest {
            id: "pack_3".into(),
            method: METHOD_PACKS_JOBS_GET.into(),
            params: serde_json::json!({ "job_id": "engineering__repo_explain" }),
        };
        let get_res = dispatcher.handle_request(get_req).await;
        assert!(get_res.is_success());
        let job_item: PacksJobCatalogItem =
            serde_json::from_value(get_res.result.unwrap()).unwrap();
        assert_eq!(job_item.job_id, "engineering__repo_explain");
        assert_eq!(job_item.primary_role, "repo_scout");
        assert_eq!(job_item.status, JobOperationalStatus::Available);

        // 3b. Get Missing Job
        let missing_req = ApiRequest {
            id: "pack_3b".into(),
            method: METHOD_PACKS_JOBS_GET.into(),
            params: serde_json::json!({ "job_id": "non_existent_job" }),
        };
        let missing_res = dispatcher.handle_request(missing_req).await;
        assert!(!missing_res.is_success());
        assert!(missing_res.error.unwrap().contains("not found"));

        // 4. List Roles via v1.packs.roles.list
        let roles_req = ApiRequest {
            id: "pack_4".into(),
            method: METHOD_PACKS_ROLES_LIST.into(),
            params: serde_json::json!({}),
        };
        let roles_res = dispatcher.handle_request(roles_req).await;
        assert!(roles_res.is_success());
        let all_roles: Vec<custos_packs::WorkerRole> =
            serde_json::from_value(roles_res.result.unwrap()).unwrap();
        assert_eq!(all_roles.len(), 12, "Expected 12 specialized Worker Roles");

        let eng_roles_req = ApiRequest {
            id: "pack_4b".into(),
            method: METHOD_PACKS_ROLES_LIST.into(),
            params: serde_json::json!({ "pack_id": "engineering" }),
        };
        let eng_roles_res = dispatcher.handle_request(eng_roles_req).await;
        let eng_roles: Vec<custos_packs::WorkerRole> =
            serde_json::from_value(eng_roles_res.result.unwrap()).unwrap();
        assert_eq!(eng_roles.len(), 4, "Expected 4 Engineering Worker Roles");

        // 5. Run Workbench Job via v1.packs.jobs.run
        let session = session_manager.create_session(custos_domain::SessionMode::Bare).await;
        let run_req = ApiRequest {
            id: "pack_5".into(),
            method: METHOD_PACKS_JOBS_RUN.into(),
            params: serde_json::json!({
                "job_id": "engineering__repo_explain",
                "session_id": session.id.0,
                "inputs": {
                    "query": "TokenBucket",
                    "scope": ["crates/custos-core"]
                }
            }),
        };
        let run_res = dispatcher.handle_request(run_req).await;
        assert!(run_res.is_success(), "run_res failed: {:?}", run_res.error);
        let run_val = run_res.result.unwrap();
        assert_eq!(run_val.get("status").unwrap().as_str(), Some("created"));
        assert_eq!(run_val.get("job_id").unwrap().as_str(), Some("engineering__repo_explain"));
        let task_id = run_val.get("task_id").unwrap().as_str().unwrap();

        // Verify task was created in TaskService with provenance metadata
        let task = task_service.get_task(task_id).await.unwrap().expect("task exists");
        assert_eq!(task.metadata.get("source").unwrap().as_str(), Some("packs_job"));
        assert_eq!(task.metadata.get("job_id").unwrap().as_str(), Some("engineering__repo_explain"));
        assert_eq!(task.metadata.get("pack_id").unwrap().as_str(), Some("engineering"));
        assert_eq!(task.metadata.get("primary_role").unwrap().as_str(), Some("repo_scout"));
    }

