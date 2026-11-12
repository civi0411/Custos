//! Pure Research Synthesis & Coding Handoff Application Service.

use custos_core::CreateTask;
use custos_domain::{
    new_id,
    task::{ContractEvidence, EvidenceKind, TaskContract},
    DomainError, Recipe, ReviewStatus,
};

use crate::domain::claim::{ClaimGroundingLevel, ResearchClaim};
use crate::domain::synthesis::{
    ClaimHandoffSummary, HandoffToCodingParams, RecipeHandoffSummary, ResearchSynthesisProposal,
    SaveSynthesisProposalParams,
};
use crate::ResearchRepository;

#[derive(Debug, Clone)]
pub struct PreparedCodingHandoff {
    pub task_commands: Vec<CreateTask>,
    pub verified_claims_count: usize,
    pub converted_recipes_count: usize,
    pub caveats: Vec<String>,
}

pub struct SynthesisService;

impl SynthesisService {
    pub fn build_and_save_proposal(
        research: &ResearchRepository,
        all_reviews: &[custos_domain::ReviewerRecord],
        params: SaveSynthesisProposalParams,
    ) -> Result<ResearchSynthesisProposal, DomainError> {
        let all_claims = research.list_claims()?;

        let claims = collect_claim_summaries(&params.claim_ids, &all_claims, all_reviews);
        let all_recipes = research.list_recipes()?;
        let recipes = collect_recipe_summaries(&params.recipe_ids, &all_recipes);

        let mut proposal = ResearchSynthesisProposal::new(
            params.title,
            params.summary,
            claims,
            recipes,
            params.artifact_paths.unwrap_or_default(),
            params.workspace_id,
            params.target_branch,
        )?;

        if let Some(existing_id) = params.id {
            proposal.id = existing_id;
        }

        research.save_synthesis_proposal(&proposal)?;
        Ok(proposal)
    }

    pub fn prepare_claim_investigation(
        research: &ResearchRepository,
        claim_id: &str,
    ) -> Result<CreateTask, DomainError> {
        let claim = research
            .list_claims()?
            .into_iter()
            .find(|c| c.id == claim_id)
            .ok_or_else(|| DomainError::Validation("Claim does not exist".into()))?;

        Ok(CreateTask {
            title: format!("Investigate research claim: {}", claim.statement),
            metadata: Some(serde_json::json!({
                "source": "research_workbench",
                "claim_id": claim_id,
                "claim_statement": claim.statement,
                "claim_grounding": "unverified_client_draft",
            })),
            contract: None,
        })
    }

    pub fn prepare_coding_handoff(
        research: &ResearchRepository,
        all_reviews: &[custos_domain::ReviewerRecord],
        params: &HandoffToCodingParams,
    ) -> Result<PreparedCodingHandoff, DomainError> {
        let all_claims = research.list_claims()?;

        let mut selected_claims = Vec::with_capacity(params.claim_ids.len());
        for cid in &params.claim_ids {
            let claim = all_claims
                .iter()
                .find(|c| &c.id == cid)
                .ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Claim '{cid}' not found in research repository"
                    ))
                })?;
            selected_claims.push(claim);
        }

        let all_recipes = research.list_recipes()?;
        let mut selected_recipes: Vec<&Recipe> = Vec::with_capacity(params.recipe_ids.len());
        for rid in &params.recipe_ids {
            let recipe = all_recipes
                .iter()
                .find(|r| &r.id == rid)
                .ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Recipe '{rid}' not found in research repository"
                    ))
                })?;
            selected_recipes.push(recipe);
        }

        if params.enforce_verification && !selected_claims.is_empty() {
            let unverified = selected_claims.iter().find(|c| {
                !(c.level >= ClaimGroundingLevel::L2Verified
                    || all_reviews.iter().any(|r| {
                        r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                    }))
            });
            if let Some(bad_claim) = unverified {
                return Err(DomainError::Validation(format!(
                    "Fail-closed verification gate: claim '{}' is ungrounded (L0/unverified) and lacks approved active verifier record or L2+ proof",
                    bad_claim.id
                )));
            }
        }

        let claim_summaries = collect_claim_summaries(&params.claim_ids, &all_claims, all_reviews);
        let recipe_summaries = collect_recipe_summaries(&params.recipe_ids, &all_recipes);
        let caveats = ResearchSynthesisProposal::generate_caveats(&claim_summaries, &recipe_summaries);

        let mut task_commands = Vec::with_capacity(selected_claims.len() + selected_recipes.len());
        for claim in &selected_claims {
            task_commands.push(CreateTask {
                title: format!("[Research Claim] {}", claim.statement),
                metadata: Some(serde_json::json!({
                    "source": "research_synthesis_handoff",
                    "proposal_id": params.proposal_id,
                    "claim_id": claim.id,
                    "claim_statement": claim.statement,
                    "claim_level": claim.level,
                    "workspace_id": params.workspace_id,
                    "target_branch": params.target_branch,
                    "caveats": caveats,
                })),
                contract: Some(TaskContract {
                    pack_id: "engineering_verification".into(),
                    name: format!("Verify: {}", claim.statement),
                    description: format!(
                        "Empirically implement and produce diff / test verification for claim: {}",
                        claim.statement
                    ),
                    required_capabilities: vec![
                        "code.files".into(),
                        "code.changes".into(),
                        "compute.pty".into(),
                    ],
                    evidence_requirements: vec![
                        ContractEvidence {
                            kind: EvidenceKind::Diff,
                            required: true,
                        },
                        ContractEvidence {
                            kind: EvidenceKind::TestResult,
                            required: true,
                        },
                    ],
                }),
            });
        }

        for recipe in &selected_recipes {
            task_commands.push(CreateTask {
                title: format!("[Recipe Reproduction] {}", recipe.name),
                metadata: Some(serde_json::json!({
                    "source": "research_synthesis_handoff",
                    "proposal_id": params.proposal_id,
                    "recipe_id": recipe.id,
                    "recipe_name": recipe.name,
                    "recipe_command": recipe.command,
                    "workspace_id": params.workspace_id,
                    "target_branch": params.target_branch,
                })),
                contract: Some(TaskContract {
                    pack_id: "recipe_execution".into(),
                    name: format!("Execute: {}", recipe.name),
                    description: format!("Execute recipe command: {}", recipe.command),
                    required_capabilities: vec!["compute.pty".into()],
                    evidence_requirements: vec![ContractEvidence {
                        kind: EvidenceKind::TestResult,
                        required: true,
                    }],
                }),
            });
        }

        Ok(PreparedCodingHandoff {
            task_commands,
            verified_claims_count: selected_claims.len(),
            converted_recipes_count: selected_recipes.len(),
            caveats,
        })
    }

    pub fn mark_proposal_completed(research: &ResearchRepository, proposal_id: &str) {
        if let Ok(Some(mut prop)) = research.get_synthesis_proposal(proposal_id) {
            prop.mark_completed();
            let _ = research.save_synthesis_proposal(&prop);
        }
    }

    pub fn new_handoff_id() -> String {
        new_id("handoff")
    }
}

fn collect_claim_summaries(
    claim_ids: &[String],
    all_claims: &[ResearchClaim],
    all_reviews: &[custos_domain::ReviewerRecord],
) -> Vec<ClaimHandoffSummary> {
    claim_ids
        .iter()
        .filter_map(|cid| {
            all_claims.iter().find(|c| &c.id == cid).map(|c| {
                let has_fresh_review = all_reviews.iter().any(|r| {
                    r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                });
                ClaimHandoffSummary {
                    claim_id: c.id.clone(),
                    statement: c.statement.clone(),
                    level: c.level,
                    confidence_score: c.confidence_score,
                    has_fresh_review,
                    sealed_proof_uri: c.sealed_proof_uri.clone(),
                }
            })
        })
        .collect()
}

fn collect_recipe_summaries(recipe_ids: &[String], all_recipes: &[Recipe]) -> Vec<RecipeHandoffSummary> {
    recipe_ids
        .iter()
        .filter_map(|rid| {
            all_recipes.iter().find(|r| &r.id == rid).map(|r| RecipeHandoffSummary {
                recipe_id: r.id.clone(),
                name: r.name.clone(),
                command: r.command.clone(),
                inputs_count: r.inputs.len(),
                outputs: r.outputs.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::claim::{ClaimGroundingLevel, ResearchClaim};

    #[test]
    fn test_synthesis_service_saves_proposal_and_enforces_handoff_gate() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let repo = ResearchRepository::from_raw(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let unverified = ResearchClaim {
            id: "claim_l0".into(),
            statement: "Unverified hypothesis".into(),
            level: ClaimGroundingLevel::L0Ungrounded,
            confidence_score: 0.3,
            invariants: vec![],
            evidence_links: vec![],
            created_at: 1000,
            sealed_proof_uri: None,
        };
        let verified = ResearchClaim {
            id: "claim_l2".into(),
            statement: "Verified result".into(),
            level: ClaimGroundingLevel::L2Verified,
            confidence_score: 0.9,
            invariants: vec![],
            evidence_links: vec![],
            created_at: 1001,
            sealed_proof_uri: Some("cas://bafy123".into()),
        };
        repo.save_claim(&unverified).unwrap();
        repo.save_claim(&verified).unwrap();

        let prop = SynthesisService::build_and_save_proposal(
            &repo,
            &[],
            SaveSynthesisProposalParams {
                id: None,
                title: "SADE Synthesis".into(),
                summary: "Proposal summary".into(),
                claim_ids: vec![verified.id.clone()],
                recipe_ids: vec![],
                artifact_paths: None,
                workspace_id: None,
                target_branch: None,
            },
        )
        .unwrap();
        assert_eq!(prop.claims.len(), 1);

        let blocked = SynthesisService::prepare_coding_handoff(
            &repo,
            &[],
            &HandoffToCodingParams {
                proposal_id: Some(prop.id.clone()),
                title: "Handoff".into(),
                claim_ids: vec![unverified.id.clone()],
                recipe_ids: vec![],
                artifact_paths: None,
                workspace_id: None,
                target_branch: None,
                enforce_verification: true,
            },
        );
        assert!(blocked.is_err());

        let plan = SynthesisService::prepare_coding_handoff(
            &repo,
            &[],
            &HandoffToCodingParams {
                proposal_id: Some(prop.id.clone()),
                title: "Handoff".into(),
                claim_ids: vec![verified.id],
                recipe_ids: vec![],
                artifact_paths: None,
                workspace_id: Some("ws_1".into()),
                target_branch: Some("main".into()),
                enforce_verification: true,
            },
        )
        .unwrap();
        assert_eq!(plan.task_commands.len(), 1);
        assert_eq!(plan.verified_claims_count, 1);
    }
}
