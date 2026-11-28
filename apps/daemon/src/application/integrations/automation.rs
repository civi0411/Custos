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
                let fleet = match self.workbenches.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let Some(job_id) = req.params.get("job_id").and_then(|v| v.as_str()) else {
                    return ApiResponse::error(req.id, "Missing required 'job_id' parameter");
                };
                let mut job = match fleet.get_headless_job(job_id) {
                    Ok(Some(j)) => j,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Automation job '{job_id}' not found"),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Bridge headless job through TaskService + Supervision instead of raw unguarded shell
                let title = format!("Automation: {}", job.name);
                let task = match self.kernel.task_service.create_task(title).await {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                job.status = HeadlessJobStatus::Running;
                job.started_at = Some(chrono::Utc::now().timestamp_millis());
                if let Err(e) = fleet.save_headless_job(&job) {
                    return ApiResponse::error(req.id, e.to_string());
                }

                tracing::info!(job_id = %job.id, task_id = %task.id, "Dispatched automation job into governed TaskService");
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "job": job,
                        "task_id": task.id,
                        "status": "dispatched_to_task_kernel"
                    }),
                )
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
