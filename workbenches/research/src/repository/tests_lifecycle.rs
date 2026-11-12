use super::*;
use crate::domain::claim::{
    ClaimEvidenceLink, ClaimGroundingLevel, EvidenceRelation, PassageAnchor, ResearchClaim,
    ResearchExperimentRun, SourceRecord,
};
use crate::domain::synthesis::{
    ClaimHandoffSummary, RecipeHandoffSummary, ResearchSynthesisProposal, SynthesisProposalStatus,
};
use custos_domain::{
    EnvSnapshot, ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus, Recipe,
    RecipeEnvironmentSpec, RecipeInput,
};

#[test]
fn test_research_repository_full_lifecycle() {
    let db = DbConnection::open_in_memory().expect("open db");
    let repo = ResearchRepository::new(db);

    // 1. Source
    let src = SourceRecord {
        id: "src_123".to_string(),
        source_type: "paper".to_string(),
        title: "Attention Is All You Need".to_string(),
        doi: Some("10.1145/test".to_string()),
        authors: vec!["Vaswani et al.".to_string()],
        year: Some(2017),
        content_hash: "hash_vaswani".to_string(),
        local_path: Some("/papers/transformer.pdf".to_string()),
        verified: true,
        abstract_text: Some("We propose Transformer...".to_string()),
        created_at: 1700000000,
    };
    repo.save_source(&src).expect("save source");
    let sources = repo.list_sources().expect("list sources");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].title, "Attention Is All You Need");
    assert!(sources[0].verified);

    // 2. Anchor
    let anchor = PassageAnchor {
        id: "anc_01".to_string(),
        source_id: "src_123".to_string(),
        source_title: Some("Attention Is All You Need".to_string()),
        section_title: Some("3.2.2 Multi-Head Attention".to_string()),
        page_number: Some(3),
        start_offset: 120,
        end_offset: 250,
        exact_text: "Multi-Head Attention consists of several attention layers".to_string(),
        passage_hash: "hash_anchor_01".to_string(),
    };
    repo.save_anchor(&anchor).expect("save anchor");
    let anchors = repo
        .list_anchors_for_source("src_123")
        .expect("list anchors");
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].exact_text, anchor.exact_text);

    // 3. Claim
    let claim = ResearchClaim {
        id: "claim_01".to_string(),
        statement: "Multi-Head Attention runs 8 parallel heads in base model".to_string(),
        level: ClaimGroundingLevel::L1Cited,
        confidence_score: 0.95,
        invariants: vec!["head_count == 8".to_string()],
        evidence_links: vec![ClaimEvidenceLink {
            passage_anchor_id: "anc_01".to_string(),
            source_title: Some("Attention Is All You Need".to_string()),
            exact_text: Some(
                "Multi-Head Attention consists of several attention layers".to_string(),
            ),
            relation: EvidenceRelation::Supports,
            rationale: "Confirmed in section 3.2.2".to_string(),
            verified_by: "expert_review".to_string(),
        }],
        created_at: 1700000200,
        sealed_proof_uri: None,
    };
    repo.save_claim(&claim).expect("save claim");
    let claims = repo.list_claims().expect("list claims");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].statement, claim.statement);
    assert_eq!(claims[0].level, ClaimGroundingLevel::L1Cited);
    assert_eq!(claims[0].evidence_links.len(), 1);

    // 4. Run
    let run = ResearchExperimentRun {
        run_id: "run_exp_99".to_string(),
        session_id: "sess_01".to_string(),
        command: "python train_transformer.py".to_string(),
        cwd: "/workspace".to_string(),
        status: "ok".to_string(),
        wall_ms: 45000,
        surface: Some("sade_container".to_string()),
        reproducibility: "deterministic".to_string(),
        input_merkle_root: "merkle_in_root".to_string(),
        output_merkle_root: "merkle_out_root".to_string(),
        env_snapshot: EnvSnapshot {
            python_version: "3.11.4".to_string(),
            lockfile_hash: "lock_abc".to_string(),
            package_count: Some(42),
            hardware: "Apple M3 Max".to_string(),
            platform: Some("macos-arm64".to_string()),
        },
        sade_permit_id: Some("permit_01".to_string()),
        cas_log_uri: Some("cas://bafy_log".to_string()),
        ts: 1700000300,
    };
    repo.save_run(&run).expect("save run");
    let runs = repo.list_runs().expect("list runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, "run_exp_99");
    assert_eq!(runs[0].reproducibility, "deterministic");

    // 5. Recipe
    let mut env_spec = RecipeEnvironmentSpec::new();
    env_spec.python_version = Some("3.11.4".to_string());
    env_spec.requirements = vec!["torch>=2.0.0".to_string(), "pandas".to_string()];
    let recipe = Recipe::new(
        "Train Ablation Model",
        "python train.py --lr 1e-4",
        env_spec,
    )
    .with_description("Ablation study on learning rate")
    .with_input(RecipeInput::new("train_data", "cas://bafy_data", true))
    .with_output("loss_curve.png");
    let recipe_id = recipe.id.clone();
    repo.save_recipe(&recipe).expect("save recipe");

    let fetched_recipe = repo
        .get_recipe(&recipe_id)
        .expect("get recipe")
        .expect("recipe exists");
    assert_eq!(fetched_recipe.name, "Train Ablation Model");
    assert_eq!(fetched_recipe.inputs.len(), 1);
    assert_eq!(fetched_recipe.outputs, vec!["loss_curve.png"]);

    let recipes = repo.list_recipes().expect("list recipes");
    assert_eq!(recipes.len(), 1);

    // 7. Execution Record
    let mut exec_record = ExecutionRecord::new(&recipe_id).with_session_id("session_res_01");
    exec_record.stdout_cas_uri = Some("cas://bafy_exec_stdout".to_string());
    exec_record.artifacts.push(ExecutionArtifact::new(
        "loss_curve.png",
        "hash_loss_curve",
        10240,
    ));
    let exec_id = exec_record.id.clone();
    repo.save_execution_record(&exec_record)
        .expect("save exec record");

    let fetched_exec = repo
        .get_execution_record(&exec_id)
        .expect("get exec")
        .expect("exec exists");
    assert_eq!(fetched_exec.status, ExecutionRecordStatus::Pending);
    assert_eq!(fetched_exec.artifacts.len(), 1);

    exec_record.transition(ExecutionRecordStatus::Completed);
    exec_record.exit_code = Some(0);
    repo.save_execution_record(&exec_record)
        .expect("update exec record");

    let updated_exec = repo
        .get_execution_record(&exec_id)
        .expect("get exec updated")
        .expect("exec exists");
    assert_eq!(updated_exec.status, ExecutionRecordStatus::Completed);
    assert_eq!(updated_exec.exit_code, Some(0));

    let execs_for_recipe = repo
        .list_execution_records_for_recipe(&recipe_id)
        .expect("list execs for recipe");
    assert_eq!(execs_for_recipe.len(), 1);

    // 8. Research Synthesis Proposal
    let proposal = ResearchSynthesisProposal::new(
        "Empirical Attention Bounds",
        "Synthesis of multi-head attention claims and verification tasks",
        vec![ClaimHandoffSummary {
            claim_id: "claim_01".into(),
            statement: "Multi-Head Attention runs 8 parallel heads in base model".into(),
            level: ClaimGroundingLevel::L1Cited,
            confidence_score: 0.95,
            has_fresh_review: true,
            sealed_proof_uri: None,
        }],
        vec![RecipeHandoffSummary {
            recipe_id: "recipe_01".into(),
            name: "Train transformer".into(),
            command: "python train.py".into(),
            inputs_count: 1,
            outputs: vec!["weights.pt".into()],
        }],
        vec!["eval_table.csv".into()],
        Some("ws_default".into()),
        Some("feature/attention-bounds".into()),
    )
    .expect("create synthesis proposal");

    repo.save_synthesis_proposal(&proposal)
        .expect("save proposal");
    let fetched_prop = repo
        .get_synthesis_proposal(&proposal.id)
        .expect("get proposal")
        .expect("proposal exists");
    assert_eq!(fetched_prop.title, "Empirical Attention Bounds");
    assert_eq!(fetched_prop.claims.len(), 1);
    assert_eq!(fetched_prop.recipes.len(), 1);
    assert_eq!(fetched_prop.status, SynthesisProposalStatus::Draft);

    let proposals = repo.list_synthesis_proposals().expect("list proposals");
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].id, proposal.id);
}
