use crate::api::*;
use custos_domain::{RecordReviewParams, ReviewerRecord};

impl LocalApiDispatcher {
    pub(crate) async fn handle_shared_reviews(&self, req: ApiRequest) -> ApiResponse {
        let shared = match self.workbenches.shared_capabilities.as_ref() {
            Some(s) => s,
            None => {
                return ApiResponse::error(
                    req.id,
                    "SharedCapabilitiesRepository not configured on daemon",
                )
            }
        };

        match req.method.as_str() {
            METHOD_REVIEWS_LIST => {
                let target_type = req
                    .params
                    .get("target_type")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                let target_id = req.params.get("target_id").and_then(|v| v.as_str());
                match shared.list_reviews(target_type, target_id) {
                    Ok(reviews) => match serde_json::to_value(&reviews) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_GET => {
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match shared.get_review(id) {
                    Ok(Some(review)) => match serde_json::to_value(&review) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, "Review not found"),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_RECORD => {
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
                if let Err(e) = shared.save_review(&record) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                match serde_json::to_value(&record) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_MARK_STALE => {
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match shared.mark_review_stale(id) {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "marked_stale": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown reviews record method: {unknown}")),
        }
    }
}
