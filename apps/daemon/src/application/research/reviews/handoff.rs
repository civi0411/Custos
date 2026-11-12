use crate::api::*;
use custos_workbench_research::{HandoffToCodingParams, HandoffToCodingResult, SynthesisService};

impl LocalApiDispatcher {
    pub(crate) async fn handle_research_handoff(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_HANDOFF_CODING => {
                let research = match self.research_repo() {
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
                let cmd = match SynthesisService::prepare_claim_investigation(&research, claim_id) {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                match self.kernel.task_service.execute_create(cmd).await {
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
            METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                let research = match self.research_repo() {
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

                let all_reviews = self
                    .workbenches.shared_capabilities
                    .as_ref()
                    .and_then(|s| s.list_reviews(None, None).ok())
                    .unwrap_or_default();
                let plan =
                    match SynthesisService::prepare_coding_handoff(&research, &all_reviews, &params)
                    {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let mut created_task_ids = Vec::with_capacity(plan.task_commands.len());
                for cmd in plan.task_commands {
                    match self.kernel.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self
                                    .kernel.task_service
                                    .execute_advance(AdvanceTask {
                                        task_id: created_id.clone(),
                                        next_status: custos_domain::task::TaskStatus::Failed,
                                        expected_epoch: 1,
                                        rationale: Some(format!(
                                            "Handoff aborted due to child task creation failure: {e}"
                                        )),
                                    })
                                    .await;
                            }
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create task: {e}"),
                            );
                        }
                    }
                }

                if let Some(ref prop_id) = params.proposal_id {
                    SynthesisService::mark_proposal_completed(&research, prop_id);
                }

                let handoff_res = HandoffToCodingResult {
                    handoff_id: SynthesisService::new_handoff_id(),
                    proposal_id: params.proposal_id,
                    task_ids: created_task_ids,
                    workspace_id: params.workspace_id,
                    target_branch: params.target_branch,
                    verified_claims_count: plan.verified_claims_count,
                    converted_recipes_count: plan.converted_recipes_count,
                    caveats: plan.caveats,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                match serde_json::to_value(&handoff_res) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown handoff method: {unknown}")),
        }
    }
}
