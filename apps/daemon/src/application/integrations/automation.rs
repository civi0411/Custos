//! Headless automation job scheduling and execution handlers.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations_automation(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_AUTOMATION_JOBS_LIST => {
                let fleet = match self.workbenches.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let status_filter = req
                    .params
                    .get("status")
                    .and_then(|v| serde_json::from_value::<HeadlessJobStatus>(v.clone()).ok());
                match fleet.list_headless_jobs() {
                    Ok(jobs) => {
                        let filtered: Vec<HeadlessAutomationJob> =
                            if let Some(filter) = status_filter {
                                jobs.into_iter().filter(|j| j.status == filter).collect()
                            } else {
                                jobs
                            };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_CREATE => {
                let fleet = match self.workbenches.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let params: CreateHeadlessJobParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid job parameters: {e}"))
                    }
                };
                let trigger = params
                    .trigger
                    .unwrap_or(custos_contracts::HeadlessTrigger::Manual);
                let job = match HeadlessAutomationJob::new(params.name, params.spec, trigger) {
                    Ok(j) => j,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_headless_job(&job) {
                    Ok(()) => match serde_json::to_value(&job) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_RUN => {
                // Fail closed: a bare `permit_id` string is not proof of authority. Headless shell runs stay
                // disabled until an executor verifies a real kernel ExecutionPermit bound to this job.
                ApiResponse::error(
                    req.id,
                    "Headless execution is unavailable until an executor and scoped authority checks are connected",
                )
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
