use std::sync::Arc;

use custos_core::TaskService;
use custos_runtime::bridge::BridgeService;
use custos_runtime::session::SessionManager;
use custos_workbench_research::{
    ClaimGroundingLevel, HandoffToCodingResult, ResearchClaim, ResearchSynthesisProposal,
    SynthesisProposalStatus,
};

use crate::api::*;

#[tokio::test]
async fn test_synthesis_handoff_api_lifecycle() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::new());
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let mem_conn = custos_workbench_research::repository::DbConnection::open_in_memory().expect("open memory db");
    let research_repo = Arc::new(custos_workbench_research::ResearchRepository::new(mem_conn));

    // Seed a verified claim (L2) and an ungrounded claim (L0)
    let verified_claim = ResearchClaim {
        id: "claim_v1".into(),
        statement: "Attention layers exhibit $O(N^2)$ quadratic complexity".into(),
        level: ClaimGroundingLevel::L2Verified,
        confidence_score: 0.98,
        invariants: vec!["quadratic_scaling".into()],
        evidence_links: vec![],
        created_at: chrono::Utc::now().timestamp(),
        sealed_proof_uri: Some("cas://bafy_proof_1".into()),
    };
    research_repo.save_claim(&verified_claim).unwrap();

    let ungrounded_claim = ResearchClaim {
        id: "claim_u1".into(),
        statement: "Unverified speculative optimization".into(),
        level: ClaimGroundingLevel::L0Ungrounded,
        confidence_score: 0.1,
        invariants: vec![],
        evidence_links: vec![],
        created_at: chrono::Utc::now().timestamp(),
        sealed_proof_uri: None,
    };
    research_repo.save_claim(&ungrounded_claim).unwrap();

    // Seed a recipe
    let recipe = custos_domain::Recipe::new(
        "Benchmark Attention",
        "cargo bench --bench attention",
        custos_domain::RecipeEnvironmentSpec {
            python_version: None,
            requirements: vec![],
            container_image: None,
            hardware: None,
        },
    );
    let recipe_id = recipe.id.clone();
    research_repo.save_recipe(&recipe).unwrap();

    let dispatcher = LocalApiDispatcher::new(task_service.clone(), session_manager, bridge_service)
        .with_research(research_repo.clone());

    // 1. Save synthesis proposal
    let save_req = ApiRequest {
        id: "prop_save_1".into(),
        method: METHOD_SYNTHESIS_PROPOSALS_SAVE.into(),
        params: serde_json::json!({
            "title": "Attention Quadratic Optimization",
            "summary": "Synthesize benchmarks and handoff invariant tasks to coding",
            "claim_ids": ["claim_v1", "claim_u1"],
            "recipe_ids": [recipe_id],
            "artifact_paths": ["artifacts/benchmark.csv"],
            "workspace_id": "ws_bench",
            "target_branch": "perf/attention-quad",
        }),
    };
    let save_resp = dispatcher.handle_request(save_req).await;
    assert!(
        save_resp.is_success(),
        "Save proposal failed: {:?}",
        save_resp.error
    );
    let prop: ResearchSynthesisProposal =
        serde_json::from_value(save_resp.result.unwrap()).unwrap();
    assert_eq!(prop.title, "Attention Quadratic Optimization");
    assert_eq!(prop.claims.len(), 2);
    assert_eq!(prop.recipes.len(), 1);
    assert!(
        !prop.caveats.is_empty(),
        "Should generate caveats for ungrounded claim"
    );

    // 2. Get and List proposal
    let get_req = ApiRequest {
        id: "prop_get_1".into(),
        method: METHOD_SYNTHESIS_PROPOSALS_GET.into(),
        params: serde_json::json!({ "id": prop.id }),
    };
    let get_resp = dispatcher.handle_request(get_req).await;
    assert!(get_resp.is_success());
    let fetched: Option<ResearchSynthesisProposal> =
        serde_json::from_value(get_resp.result.unwrap()).unwrap();
    assert_eq!(fetched.unwrap().id, prop.id);

    let list_req = ApiRequest {
        id: "prop_list_1".into(),
        method: METHOD_SYNTHESIS_PROPOSALS_LIST.into(),
        params: serde_json::json!({}),
    };
    let list_resp = dispatcher.handle_request(list_req).await;
    assert!(list_resp.is_success());
    let props: Vec<ResearchSynthesisProposal> =
        serde_json::from_value(list_resp.result.unwrap()).unwrap();
    assert_eq!(props.len(), 1);

    // 3. Test Fail-Closed Rejection: Handoff only ungrounded claim with enforce_verification = true
    let reject_req = ApiRequest {
        id: "handoff_rej_1".into(),
        method: METHOD_SYNTHESIS_HANDOFF_EXECUTE.into(),
        params: serde_json::json!({
            "title": "Premature Handoff",
            "claim_ids": ["claim_u1"],
            "recipe_ids": [],
            "enforce_verification": true,
        }),
    };
    let reject_resp = dispatcher.handle_request(reject_req).await;
    assert!(
        reject_resp.error.is_some(),
        "Fail-closed check should reject unverified L0 claims"
    );
    assert!(reject_resp
        .error
        .unwrap()
        .contains("Fail-closed verification gate"));

    // 4. Test Successful Handoff: includes verified claim and recipe
    let exec_req = ApiRequest {
        id: "handoff_exec_1".into(),
        method: METHOD_SYNTHESIS_HANDOFF_EXECUTE.into(),
        params: serde_json::json!({
            "proposal_id": prop.id,
            "title": "Execute Attention Handoff",
            "claim_ids": ["claim_v1"],
            "recipe_ids": [recipe_id],
            "workspace_id": "ws_bench",
            "target_branch": "perf/attention-quad",
            "enforce_verification": true,
        }),
    };
    let exec_resp = dispatcher.handle_request(exec_req).await;
    assert!(
        exec_resp.is_success(),
        "Handoff execution failed: {:?}",
        exec_resp.error
    );
    let handoff_res: HandoffToCodingResult =
        serde_json::from_value(exec_resp.result.unwrap()).unwrap();
    assert_eq!(handoff_res.verified_claims_count, 1);
    assert_eq!(handoff_res.converted_recipes_count, 1);
    assert_eq!(handoff_res.task_ids.len(), 2);

    // 5. Verify created tasks exist in TaskService
    let task_0 = task_service
        .get_task(&handoff_res.task_ids[0])
        .await
        .unwrap();
    assert!(task_0.is_some());
    let t0 = task_0.unwrap();
    assert!(t0.title.contains("[Research Claim]"));
    assert!(
        t0.contract.is_some(),
        "Task must have contract with evidence requirements"
    );

    let task_1 = task_service
        .get_task(&handoff_res.task_ids[1])
        .await
        .unwrap();
    assert!(task_1.is_some());
    let t1 = task_1.unwrap();
    assert!(t1.title.contains("[Recipe Reproduction]"));

    // 6. Verify proposal marked completed
    let check_prop_req = ApiRequest {
        id: "prop_check_1".into(),
        method: METHOD_SYNTHESIS_PROPOSALS_GET.into(),
        params: serde_json::json!({ "id": prop.id }),
    };
    let check_resp = dispatcher.handle_request(check_prop_req).await;
    let completed_prop: Option<ResearchSynthesisProposal> =
        serde_json::from_value(check_resp.result.unwrap()).unwrap();
    assert_eq!(
        completed_prop.unwrap().status,
        SynthesisProposalStatus::HandoffCompleted
    );
}
