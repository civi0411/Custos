use std::sync::Arc;

use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use custos_workbench_research::{
    ClaimGroundingLevel, PassageAnchor, ResearchClaim, SourceRecord,
};

use crate::api::*;

#[tokio::test]
async fn test_research_api_dispatch_lifecycle() {
    use custos_adapters::workspace::LocalWorkspaceProvider;
    use custos_persistence::SqliteTaskStore;
    use custos_runtime::workspace::WorkspaceCoordinator;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));

    let workspace = Arc::new(WorkspaceCoordinator::new(
        store.clone(),
        Arc::new(LocalWorkspaceProvider::new()),
    ));
    let research = Arc::new(custos_workbench_research::ResearchRepository::from_raw(store.db().writer_arc()));
    let dispatcher = LocalApiDispatcher::new(
        task_service.clone(),
        session_manager.clone(),
        bridge_service.clone(),
    )
    .with_workspace(workspace)
    .with_research(research.clone());

    // Research records and OrCa-style workspace lifecycle share one daemon,
    // but neither owns the other's canonical state.
    let temp = tempfile::tempdir().unwrap();
    let research_dir = temp.path().join("study");
    let create_workspace = dispatcher
        .handle_request(ApiRequest {
            id: "req_research_ws".into(),
            method: METHOD_WORKSPACES_CREATE.into(),
            params: serde_json::json!({
                "name": "study",
                "kind": { "type": "folder", "path": research_dir },
                "path": research_dir,
                "metadata": { "domain": "research" }
            }),
        })
        .await;
    assert!(
        create_workspace.is_success(),
        "Workspace failed: {:?}",
        create_workspace.error
    );
    assert!(research_dir.exists());

    // 1. Save & List Source
    let save_src_req = ApiRequest {
        id: "req_s1".into(),
        method: METHOD_RESEARCH_SOURCES_SAVE.into(),
        params: serde_json::json!({
            "id": "src_paper_01",
            "source_type": "paper",
            "title": "Quantum Error Mitigation",
            "doi": "10.1038/s41586-023-06096-3",
            "authors": ["Kim et al."],
            "year": 2023,
            "content_hash": "hash_qem",
            "local_path": "/papers/qem.pdf",
            "verified": true,
            "abstract_text": "Evidence for the utility of quantum computing before fault tolerance.",
            "created_at": 1700000000
        }),
    };
    let save_src_resp = dispatcher.handle_request(save_src_req).await;
    assert!(
        save_src_resp.is_success(),
        "Failed to save source: {:?}",
        save_src_resp.error
    );

    let list_src_req = ApiRequest {
        id: "req_s2".into(),
        method: METHOD_RESEARCH_SOURCES_LIST.into(),
        params: serde_json::json!({}),
    };
    let list_src_resp = dispatcher.handle_request(list_src_req).await;
    assert!(list_src_resp.is_success());
    let sources: Vec<SourceRecord> = serde_json::from_value(list_src_resp.result.unwrap()).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].title, "Quantum Error Mitigation");
    assert!(!sources[0].verified, "client cannot certify a source");

    // 2. Save & List Passage Anchor
    let save_anc_req = ApiRequest {
        id: "req_a1".into(),
        method: METHOD_RESEARCH_ANCHORS_SAVE.into(),
        params: serde_json::json!({
            "id": "anc_01",
            "source_id": "src_paper_01",
            "source_title": "Quantum Error Mitigation",
            "section_title": "Zero-Noise Extrapolation",
            "page_number": 4,
            "start_offset": 500,
            "end_offset": 620,
            "exact_text": "ZNE scales circuit noise artificially by pulse stretching.",
            "passage_hash": "hash_anchor_zne"
        }),
    };
    let save_anc_resp = dispatcher.handle_request(save_anc_req).await;
    assert!(save_anc_resp.is_success());

    let list_anc_req = ApiRequest {
        id: "req_a2".into(),
        method: METHOD_RESEARCH_ANCHORS_LIST.into(),
        params: serde_json::json!({ "source_id": "src_paper_01" }),
    };
    let list_anc_resp = dispatcher.handle_request(list_anc_req).await;
    assert!(list_anc_resp.is_success());
    let anchors: Vec<PassageAnchor> =
        serde_json::from_value(list_anc_resp.result.unwrap()).unwrap();
    assert_eq!(anchors.len(), 1);
    assert_eq!(
        anchors[0].exact_text,
        "ZNE scales circuit noise artificially by pulse stretching."
    );
    assert!(anchors[0].passage_hash.starts_with("sha256:"));
    assert_ne!(anchors[0].passage_hash, "hash_anchor_zne");

    // 3. Save & List Claims
    let save_claim_req = ApiRequest {
        id: "req_c1".into(),
        method: METHOD_RESEARCH_CLAIMS_SAVE.into(),
        params: serde_json::json!({
            "id": "claim_zne_01",
            "statement": "Zero noise extrapolation bounds expectation value bias within 2%",
            "level": "l1_cited",
            "confidence_score": 0.92,
            "invariants": ["abs(bias) <= 0.02"],
            "evidence_links": [{
                "passage_anchor_id": "anc_01",
                "source_title": "Quantum Error Mitigation",
                "exact_text": "ZNE scales circuit noise artificially by pulse stretching.",
                "relation": "supports",
                "rationale": "Empirical curve fitting verified",
                "verified_by": "expert_review"
            }],
            "created_at": 1700000100,
            "sealed_proof_uri": null
        }),
    };
    let save_claim_resp = dispatcher.handle_request(save_claim_req).await;
    assert!(
        save_claim_resp.is_success(),
        "Failed to save claim: {:?}",
        save_claim_resp.error
    );

    let list_claim_req = ApiRequest {
        id: "req_c2".into(),
        method: METHOD_RESEARCH_CLAIMS_LIST.into(),
        params: serde_json::json!({}),
    };
    let list_claim_resp = dispatcher.handle_request(list_claim_req).await;
    assert!(list_claim_resp.is_success());
    let claims: Vec<ResearchClaim> =
        serde_json::from_value(list_claim_resp.result.unwrap()).unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].id, "claim_zne_01");
    assert_eq!(
        claims[0].level,
        ClaimGroundingLevel::L0Ungrounded
    );
    assert_eq!(
        claims[0].evidence_links[0].verified_by,
        "unreviewed_client_proposal"
    );

    // 4. Recipe definitions and observed executions stay separate. The API
    // exposes the ledger read-only; it does not execute the recipe command.
    let recipe = custos_domain::Recipe::new(
        "QEM reproduction",
        "python reproduce.py --seed 7",
        custos_domain::RecipeEnvironmentSpec::new(),
    );
    research.save_recipe(&recipe).unwrap();
    let mut execution =
        custos_domain::ExecutionRecord::new(&recipe.id).with_session_id("research_session_01");
    execution.transition(custos_domain::ExecutionRecordStatus::Completed);
    execution.exit_code = Some(0);
    research.save_execution_record(&execution).unwrap();

    let list_executions = dispatcher
        .handle_request(ApiRequest {
            id: "req_exec_list".into(),
            method: "v1.research.executions.list".into(),
            params: serde_json::json!({ "recipe_id": recipe.id }),
        })
        .await;
    assert!(list_executions.is_success());
    let records: Vec<custos_domain::ExecutionRecord> =
        serde_json::from_value(list_executions.result.unwrap()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, execution.id);

    let get_execution = dispatcher
        .handle_request(ApiRequest {
            id: "req_exec_get".into(),
            method: "v1.research.executions.get".into(),
            params: serde_json::json!({ "execution_id": execution.id }),
        })
        .await;
    assert!(get_execution.is_success());

    let assessment = dispatcher
        .handle_request(ApiRequest {
            id: "req_exec_assess".into(),
            method: "v1.research.executions.resume.assess".into(),
            params: serde_json::json!({ "execution_id": execution.id }),
        })
        .await;
    assert!(assessment.is_success());
    let assessment = assessment.result.unwrap();
    assert_eq!(assessment["can_start"], false);
    assert_eq!(assessment["state_inventory_declared"], false);
    assert_eq!(assessment["blockers"].as_array().unwrap().len(), 1);
    for params in [
        serde_json::json!({}),
        serde_json::json!({"execution_id": "missing"}),
    ] {
        assert!(!dispatcher
            .handle_request(ApiRequest {
                id: "req_exec_assess_bad".into(),
                method: "v1.research.executions.resume.assess".into(),
                params,
            })
            .await
            .is_success());
    }

    // 5. Handoff Claim to Coding Task
    let handoff_req = ApiRequest {
        id: "req_h1".into(),
        method: METHOD_RESEARCH_HANDOFF_CODING.into(),
        params: serde_json::json!({
            "claim_id": "claim_zne_01",
            "statement": claims[0].statement
        }),
    };
    let handoff_resp = dispatcher.handle_request(handoff_req).await;
    assert!(
        handoff_resp.is_success(),
        "Failed handoff: {:?}",
        handoff_resp.error
    );
    let handoff_val = handoff_resp.result.unwrap();
    assert_eq!(handoff_val["handoff_status"], "task_created");
    assert!(handoff_val["task"]["id"].is_string());

    let forged_handoff = dispatcher
        .handle_request(ApiRequest {
            id: "req_h2".into(),
            method: METHOD_RESEARCH_HANDOFF_CODING.into(),
            params: serde_json::json!({"claim_id": "missing", "statement": "Trusted by client"}),
        })
        .await;
    assert!(forged_handoff.error.is_some());
}
