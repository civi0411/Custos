//! Research reviews, recipes, synthesis proposals, and coding handoff use cases.

use crate::api::*;
use custos_domain::{
    ClaimGroundingLevel, ClaimHandoffSummary, HandoffToCodingParams,
    HandoffToCodingResult, Recipe, RecipeHandoffSummary, RecordReviewParams,
    ResearchClaim, ResearchSynthesisProposal, ReviewStatus,
    ReviewerRecord, SaveSynthesisProposalParams,
};
#[cfg(test)]
use custos_domain::EnvironmentSpec;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research_reviews(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_HANDOFF_CODING => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim_id = match req.params.get("claim_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing claim_id parameter"),
                };
                let claim = match research.list_claims() {
                    Ok(claims) => match claims.into_iter().find(|c| c.id == claim_id) {
                        Some(claim) => claim,
                        None => return ApiResponse::error(req.id, "Claim does not exist"),
                    },
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let title = format!("Investigate research claim: {}", claim.statement);

                let cmd = CreateTask {
                    title,
                    metadata: Some(serde_json::json!({
                        "source": "research_workbench",
                        "claim_id": claim_id,
                        "claim_statement": claim.statement,
                        "claim_grounding": "unverified_client_draft",
                    })),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "handoff_status": "task_created",
                                "task": val,
                            }),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                match research.list_recipes() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe: custos_domain::Recipe = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid recipe payload: {e}"))
                    }
                };
                match research.save_recipe(&recipe) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": recipe.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.get" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.get_recipe(recipe_id) {
                    Ok(Some(recipe)) => match serde_json::to_value(&recipe) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Recipe {recipe_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let target_type = req
                    .params
                    .get("target_type")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                let target_id = req.params.get("target_id").and_then(|v| v.as_str());
                match research.list_reviews(target_type, target_id) {
                    Ok(reviews) => match serde_json::to_value(&reviews) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.get_review(id) {
                    Ok(Some(review)) => match serde_json::to_value(&review) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, "Review not found"),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_RECORD => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: RecordReviewParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid record review params: {e}"),
                        )
                    }
                };
                let record = match ReviewerRecord::new(
                    params.target_type,
                    params.target_id,
                    params.reviewer,
                    params.method,
                    params.status,
                    params.evidence_summary,
                    params.evidence_digest,
                    params.findings.unwrap_or_default(),
                ) {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                if let Err(e) = research.save_review(&record) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                match serde_json::to_value(&record) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_MARK_STALE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.mark_review_stale(id) {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "marked_stale": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_synthesis_proposals() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("proposal_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty proposal id parameter")
                    }
                };
                match research.get_synthesis_proposal(id) {
                    Ok(Some(prop)) => match serde_json::to_value(&prop) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: SaveSynthesisProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid save proposal params: {e}"),
                        )
                    }
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = research.list_reviews(None, None).unwrap_or_default();

                let claims: Vec<ClaimHandoffSummary> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| {
                        all_claims.iter().find(|c| &c.id == cid).map(|c| {
                            let has_fresh_review = all_reviews.iter().any(|r| {
                                r.target_id == c.id
                                    && r.is_fresh
                                    && r.status == ReviewStatus::Approved
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
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let recipes: Vec<RecipeHandoffSummary> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| {
                        all_recipes
                            .iter()
                            .find(|r| &r.id == rid)
                            .map(|r| RecipeHandoffSummary {
                                recipe_id: r.id.clone(),
                                name: r.name.clone(),
                                command: r.command.clone(),
                                inputs_count: r.inputs.len(),
                                outputs: r.outputs.clone(),
                            })
                    })
                    .collect();

                let mut proposal = match ResearchSynthesisProposal::new(
                    params.title,
                    params.summary,
                    claims,
                    recipes,
                    params.artifact_paths.unwrap_or_default(),
                    params.workspace_id,
                    params.target_branch,
                ) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                if let Some(existing_id) = params.id {
                    proposal.id = existing_id;
                }

                if let Err(e) = research.save_synthesis_proposal(&proposal) {
                    return ApiResponse::error(req.id, e.to_string());
                }

                match serde_json::to_value(&proposal) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: HandoffToCodingParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid handoff params: {e}"))
                    }
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = research.list_reviews(None, None).unwrap_or_default();

                // Invariant: Verify all requested claim IDs actually exist in the repository
                for cid in &params.claim_ids {
                    if !all_claims.iter().any(|c| &c.id == cid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Claim '{cid}' not found in research repository"),
                        );
                    }
                }
                let selected_claims: Vec<&ResearchClaim> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| all_claims.iter().find(|c| &c.id == cid))
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Invariant: Verify all requested recipe IDs actually exist in the repository
                for rid in &params.recipe_ids {
                    if !all_recipes.iter().any(|r| &r.id == rid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Recipe '{rid}' not found in research repository"),
                        );
                    }
                }
                let selected_recipes: Vec<&Recipe> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| all_recipes.iter().find(|r| &r.id == rid))
                    .collect();

                // Fail-closed verification gate: EVERY claim must be grounded/verified
                if params.enforce_verification && !selected_claims.is_empty() {
                    let unverified = selected_claims.iter().find(|c| {
                        !(c.level >= ClaimGroundingLevel::L2Verified
                            || all_reviews.iter().any(|r| {
                                r.target_id == c.id
                                    && r.is_fresh
                                    && r.status == ReviewStatus::Approved
                            }))
                    });
                    if let Some(bad_claim) = unverified {
                        return ApiResponse::error(
                            req.id,
                            format!(
                                "Fail-closed verification gate: claim '{}' is ungrounded (L0/unverified) and lacks approved active verifier record or L2+ proof",
                                bad_claim.id
                            ),
                        );
                    }
                }

                let claim_summaries: Vec<ClaimHandoffSummary> = selected_claims
                    .iter()
                    .map(|c| {
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
                    .collect();

                let recipe_summaries: Vec<RecipeHandoffSummary> = selected_recipes
                    .iter()
                    .map(|r| RecipeHandoffSummary {
                        recipe_id: r.id.clone(),
                        name: r.name.clone(),
                        command: r.command.clone(),
                        inputs_count: r.inputs.len(),
                        outputs: r.outputs.clone(),
                    })
                    .collect();

                let caveats = ResearchSynthesisProposal::generate_caveats(
                    &claim_summaries,
                    &recipe_summaries,
                );
                let mut created_task_ids = Vec::new();

                // 1. Create coding tasks for claims
                for claim in &selected_claims {
                    let cmd = CreateTask {
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
                            description: format!("Empirically implement and produce diff / test verification for claim: {}", claim.statement),
                            required_capabilities: vec!["code.files".into(), "code.changes".into(), "compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::Diff,
                                    required: true,
                                },
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to child task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create task: {e}"),
                            );
                        }
                    }
                }

                // 2. Create coding tasks for recipes
                for recipe in &selected_recipes {
                    let cmd = CreateTask {
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
                            evidence_requirements: vec![custos_domain::task::ContractEvidence {
                                kind: custos_domain::task::EvidenceKind::TestResult,
                                required: true,
                            }],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to recipe task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create recipe task: {e}"),
                            );
                        }
                    }
                }

                // If proposal_id is provided, mark it completed
                if let Some(ref prop_id) = params.proposal_id {
                    if let Ok(Some(mut prop)) = research.get_synthesis_proposal(prop_id) {
                        prop.mark_completed();
                        let _ = research.save_synthesis_proposal(&prop);
                    }
                }

                let handoff_res = HandoffToCodingResult {
                    handoff_id: custos_domain::new_id("handoff"),
                    proposal_id: params.proposal_id,
                    task_ids: created_task_ids,
                    workspace_id: params.workspace_id,
                    target_branch: params.target_branch,
                    verified_claims_count: selected_claims.len(),
                    converted_recipes_count: selected_recipes.len(),
                    caveats,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                match serde_json::to_value(&handoff_res) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown research reviews method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_persistence::SqliteTaskStore;

    #[tokio::test]
    async fn reviews_group_test_without_dispatcher() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.research();

        let initial = repo.list_recipes().unwrap();
        assert_eq!(initial.len(), 0);

        let mut recipe = Recipe::new("Train Tokenizer", "python train.py", EnvironmentSpec::new())
            .with_description("Train BPE tokenizer");
        recipe.id = "recipe-1".to_string();

        repo.save_recipe(&recipe).unwrap();
        let fetched = repo.get_recipe("recipe-1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Train Tokenizer");
    }
}
