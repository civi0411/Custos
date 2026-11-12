//! Omni-Intelligence (OI) API handler for local API transport.

use super::LocalApiDispatcher;
use crate::custos_local_api::{ApiRequest, ApiResponse, ExplainPlanRequest};
use custos_runtime::oi::ExplainService;

pub struct OiApiHandler;

impl OiApiHandler {
    pub fn explain(req_id: &str, params_val: serde_json::Value) -> ApiResponse {
        let params: ExplainPlanRequest = match serde_json::from_value(params_val) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req_id, format!("Invalid params: {e}")),
        };

        let task_id = params
            .task_id
            .unwrap_or_else(|| custos_domain::new_id("task"));
        let mut snapshot = custos_domain::oi::DecisionSnapshot::new(&task_id);
        if let Some(budget) = params.budget_limit_tokens {
            snapshot.remaining_budget_tokens = budget;
        }
        if let Some(ref contract) = params.contract {
            snapshot.required_capabilities = contract.required_capabilities.clone();
        }

        match ExplainService::explain(&snapshot) {
            Ok(report) => match serde_json::to_value(&report) {
                Ok(val) => ApiResponse::success(req_id, val),
                Err(e) => ApiResponse::error(req_id, e.to_string()),
            },
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }
}

impl LocalApiDispatcher {
    pub(crate) async fn handle_oi(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.oi.explain" => OiApiHandler::explain(&req.id, req.params),
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
