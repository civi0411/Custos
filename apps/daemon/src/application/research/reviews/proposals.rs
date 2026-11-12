use crate::api::*;
use custos_workbench_research::{SaveSynthesisProposalParams, SynthesisService};

impl LocalApiDispatcher {
    pub(crate) async fn handle_research_proposals(&self, req: ApiRequest) -> ApiResponse {
        let research = match self.research_repo() {
            Some(r) => r,
            None => {
                return ApiResponse::error(req.id, "ResearchRepository not configured on daemon")
            }
        };

        match req.method.as_str() {
            METHOD_SYNTHESIS_PROPOSALS_LIST => match research.list_synthesis_proposals() {
                Ok(list) => match serde_json::to_value(&list) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                },
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            METHOD_SYNTHESIS_PROPOSALS_GET => {
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
                let params: SaveSynthesisProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid save proposal params: {e}"),
                        )
                    }
                };
                let all_reviews = self
                    .workbenches.shared_capabilities
                    .as_ref()
                    .and_then(|s| s.list_reviews(None, None).ok())
                    .unwrap_or_default();
                match SynthesisService::build_and_save_proposal(&research, &all_reviews, params) {
                    Ok(proposal) => match serde_json::to_value(&proposal) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown proposals method: {unknown}")),
        }
    }
}
