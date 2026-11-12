use crate::api::*;
use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use std::sync::Arc;

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
    let dispatcher = LocalApiDispatcher::new(
        task_service.clone(),
        session_manager.clone(),
        bridge_service,
    )
    .with_pack_registry(pack_registry);

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
    assert_eq!(manifests.len(), 4);
    assert!(manifests.iter().any(|m| m.id == "engineering"));
    assert!(manifests.iter().any(|m| m.id == "research"));
    assert!(manifests.iter().any(|m| m.id == "assistant"));
    assert!(manifests.iter().any(|m| m.id == "quant"));

    // 2. List Jobs (All 24 Jobs) via v1.packs.jobs.list
    let jobs_req = ApiRequest {
        id: "pack_2".into(),
        method: METHOD_PACKS_JOBS_LIST.into(),
        params: serde_json::json!({}),
    };
    let jobs_res = dispatcher.handle_request(jobs_req).await;
    assert!(jobs_res.is_success());
    let catalog: Vec<PacksJobCatalogItem> =
        serde_json::from_value(jobs_res.result.unwrap()).unwrap();
    assert_eq!(catalog.len(), 24, "Expected 24 standardized Workbench Jobs");

    // Verify status mapping:
    let repo_explain = catalog
        .iter()
        .find(|j| j.job_id == "engineering__repo_explain")
        .unwrap();
    assert_eq!(repo_explain.status, JobOperationalStatus::Available);
    assert!(repo_explain.degradation_reason.is_none());

    let source_qa = catalog
        .iter()
        .find(|j| j.job_id == "research__source_qa")
        .unwrap();
    assert_eq!(source_qa.status, JobOperationalStatus::Available);

    let inbox_triage = catalog
        .iter()
        .find(|j| j.job_id == "assistant__inbox_triage")
        .unwrap();
    assert_eq!(inbox_triage.status, JobOperationalStatus::Available);

    let bugfix = catalog
        .iter()
        .find(|j| j.job_id == "engineering__bugfix")
        .unwrap();
    assert_eq!(bugfix.status, JobOperationalStatus::Degraded);
    assert!(bugfix.degradation_reason.is_some());

    let migration = catalog
        .iter()
        .find(|j| j.job_id == "engineering__migration")
        .unwrap();
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
    assert_eq!(
        coding_catalog.len(),
        7,
        "Expected 7 Engineering Jobs in coding lens"
    );

    let research_req = ApiRequest {
        id: "pack_2c".into(),
        method: METHOD_PACKS_JOBS_LIST.into(),
        params: serde_json::json!({ "lens": "research" }),
    };
    let research_res = dispatcher.handle_request(research_req).await;
    let research_catalog: Vec<PacksJobCatalogItem> =
        serde_json::from_value(research_res.result.unwrap()).unwrap();
    assert_eq!(
        research_catalog.len(),
        8,
        "Expected 8 Research Jobs in research lens"
    );

    let quant_req = ApiRequest {
        id: "pack_2d".into(),
        method: METHOD_PACKS_JOBS_LIST.into(),
        params: serde_json::json!({ "lens": "quant" }),
    };
    let quant_res = dispatcher.handle_request(quant_req).await;
    let quant_catalog: Vec<PacksJobCatalogItem> =
        serde_json::from_value(quant_res.result.unwrap()).unwrap();
    assert_eq!(
        quant_catalog.len(),
        4,
        "Expected 4 Quant Jobs in quant lens"
    );
    assert!(quant_catalog
        .iter()
        .all(|j| j.status == JobOperationalStatus::Available));

    // 3. Get Specific Job via v1.packs.jobs.get
    let get_req = ApiRequest {
        id: "pack_3".into(),
        method: METHOD_PACKS_JOBS_GET.into(),
        params: serde_json::json!({ "job_id": "engineering__repo_explain" }),
    };
    let get_res = dispatcher.handle_request(get_req).await;
    assert!(get_res.is_success());
    let job_item: PacksJobCatalogItem = serde_json::from_value(get_res.result.unwrap()).unwrap();
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
    assert_eq!(all_roles.len(), 16, "Expected 16 specialized Worker Roles");

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
    let session = session_manager
        .create_session(custos_domain::SessionMode::Bare)
        .await;
    let run_req = ApiRequest {
        id: "pack_5".into(),
        method: METHOD_PACKS_JOBS_RUN.into(),
        params: serde_json::json!({
            "job_id": "engineering__repo_explain",
            "session_id": session.id.0,
            "inputs": {
                "query": "TokenBucket",
                "scope": ["platform/core"]
            }
        }),
    };
    let run_res = dispatcher.handle_request(run_req).await;
    assert!(run_res.is_success(), "run_res failed: {:?}", run_res.error);
    let run_val = run_res.result.unwrap();
    assert_eq!(run_val.get("status").unwrap().as_str(), Some("created"));
    assert_eq!(
        run_val.get("job_id").unwrap().as_str(),
        Some("engineering__repo_explain")
    );
    let task_id = run_val.get("task_id").unwrap().as_str().unwrap();

    // Verify task was created in TaskService with provenance metadata
    let task = task_service
        .get_task(task_id)
        .await
        .unwrap()
        .expect("task exists");
    assert_eq!(
        task.metadata.get("source").unwrap().as_str(),
        Some("packs_job")
    );
    assert_eq!(
        task.metadata.get("job_id").unwrap().as_str(),
        Some("engineering__repo_explain")
    );
    assert_eq!(
        task.metadata.get("pack_id").unwrap().as_str(),
        Some("engineering")
    );
    assert_eq!(
        task.metadata.get("primary_role").unwrap().as_str(),
        Some("repo_scout")
    );
}
