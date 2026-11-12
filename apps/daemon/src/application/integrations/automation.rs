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
                    .unwrap_or(custos_domain::HeadlessTrigger::Manual);
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
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing 'job_id' parameter"),
                };

                let has_permit = req.params.get("permit_id").is_some();
                let force_execute = req
                    .params
                    .get("force_execute")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !has_permit && !force_execute {
                    return ApiResponse::error(
                        req.id,
                        "Headless execution is unavailable until an executor and scoped authority checks are connected",
                    );
                }

                match fleet.get_headless_job(job_id) {
                    Ok(Some(mut job)) => {
                        job.status = custos_domain::HeadlessJobStatus::Running;
                        job.started_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        use custos_core::contracts::sandbox::CommandRunnerPort;
                        let runner = custos_adapters::operations::LocalCommandRunner;
                        let timeout_ms = job.spec.timeout_secs.clamp(1, 300).saturating_mul(1000);
                        let out = runner
                            .run_shell(
                                &job.spec.command_or_script,
                                std::path::Path::new("."),
                                timeout_ms,
                            )
                            .await;

                        let (stdout, stderr, exit_code, success) = match out {
                            Ok(o) => (o.stdout, o.stderr, o.exit_code, o.exit_code == 0),
                            Err(e) => {
                                let code = if e.to_string().contains("timed out") { 124 } else { 1 };
                                (String::new(), e.to_string(), code, false)
                            }
                        };

                        job.status = if success {
                            custos_domain::HeadlessJobStatus::Completed
                        } else {
                            custos_domain::HeadlessJobStatus::Failed
                        };
                        job.exit_code = Some(exit_code);
                        job.output_log = if stderr.is_empty() {
                            stdout
                        } else {
                            format!("{stdout}\n{stderr}")
                        };
                        job.completed_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        match serde_json::to_value(&job) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Job '{job_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
