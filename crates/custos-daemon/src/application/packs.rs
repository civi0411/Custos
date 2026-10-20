//! packs application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_packs(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_PACKS_MANIFESTS_LIST => {
                let manifests = self.pack_registry.list_packs();
                match serde_json::to_value(&manifests) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_LIST => {
                let filter_lens = req.params.get("lens").and_then(|v| v.as_str());
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let all_jobs = self.pack_registry.list_all_jobs();
                let mut catalog_items = Vec::new();

                for job in all_jobs {
                    if let Some(fl) = filter_lens {
                        let matches_lens = match job.lens {
                            custos_domain::WorkbenchLens::Coding => {
                                fl.eq_ignore_ascii_case("coding") || fl.eq_ignore_ascii_case("code")
                            }
                            custos_domain::WorkbenchLens::Research => {
                                fl.eq_ignore_ascii_case("research")
                            }
                            custos_domain::WorkbenchLens::Assistant => {
                                fl.eq_ignore_ascii_case("assistant")
                                    || fl.eq_ignore_ascii_case("chat")
                            }
                        };
                        if !matches_lens {
                            continue;
                        }
                    }

                    if let Some(fp) = filter_pack {
                        if !job.job_id.starts_with(fp) {
                            continue;
                        }
                    }

                    let (status, reason) = match job.job_id.as_str() {
                        "engineering__repo_explain"
                        | "engineering__review"
                        | "research__source_qa"
                        | "research__dataset_audit"
                        | "research__reproduce"
                        | "assistant__inbox_triage"
                        | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                        "engineering__migration" => (
                            JobOperationalStatus::Planned,
                            Some("Multi-phase dependency migration requires external package registry connector".into()),
                        ),
                        "assistant__mutate_execution" => (
                            JobOperationalStatus::Planned,
                            Some("Governed outbox execution requires configured SMTP/Calendar live connector".into()),
                        ),
                        "assistant__reminder_automation" => (
                            JobOperationalStatus::Planned,
                            Some("Recurring reminder daemon automation is planned for post-MVP".into()),
                        ),

                        _ => (
                            JobOperationalStatus::Degraded,
                            Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                        ),
                    };

                    catalog_items.push(PacksJobCatalogItem {
                        job_id: job.job_id,
                        lens: job.lens,
                        name: job.name,
                        description: job.description,
                        primary_role: job.primary_role,
                        recommended_topology: job.recommended_topology,
                        status,
                        degradation_reason: reason,
                        required_inputs: job.required_inputs,
                        expected_artifacts: job.expected_artifacts,
                        required_verifiers: job.required_verifiers,
                        acceptance_rubric: job.acceptance_rubric,
                    });
                }

                match serde_json::to_value(&catalog_items) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_GET => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                match self.pack_registry.get_job(job_id) {
                    Some(job) => {
                        let (status, reason) = match job.job_id.as_str() {
                            "engineering__repo_explain"
                            | "engineering__review"
                            | "research__source_qa"
                            | "research__dataset_audit"
                            | "research__reproduce"
                            | "assistant__inbox_triage"
                            | "assistant__schedule_proposal" => (JobOperationalStatus::Available, None),

                            "engineering__migration"
                            | "assistant__mutate_execution"
                            | "assistant__reminder_automation" => (
                                JobOperationalStatus::Planned,
                                Some("Capability planned for post-MVP or requires live external connector".into()),
                            ),

                            _ => (
                                JobOperationalStatus::Degraded,
                                Some("Requires model generation; currently operating in offline/mock mode pending provider configuration".into()),
                            ),
                        };

                        let item = PacksJobCatalogItem {
                            job_id: job.job_id,
                            lens: job.lens,
                            name: job.name,
                            description: job.description,
                            primary_role: job.primary_role,
                            recommended_topology: job.recommended_topology,
                            status,
                            degradation_reason: reason,
                            required_inputs: job.required_inputs,
                            expected_artifacts: job.expected_artifacts,
                            required_verifiers: job.required_verifiers,
                            acceptance_rubric: job.acceptance_rubric,
                        };

                        match serde_json::to_value(&item) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    None => {
                        ApiResponse::error(req.id, format!("Workbench Job '{job_id}' not found"))
                    }
                }
            }
            METHOD_PACKS_ROLES_LIST => {
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let roles = if let Some(pid) = filter_pack {
                    match self.pack_registry.get_pack(pid) {
                        Some(pack) => pack.roles(),
                        None => {
                            return ApiResponse::error(
                                req.id,
                                format!("Domain pack '{pid}' not found"),
                            )
                        }
                    }
                } else {
                    self.pack_registry.list_all_roles()
                };

                match serde_json::to_value(&roles) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_RUN => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                let job = match self.pack_registry.get_job(job_id) {
                    Some(j) => j,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            format!("Workbench Job '{job_id}' not found"),
                        )
                    }
                };

                let session_id = req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let workspace_id = req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let inputs = req
                    .params
                    .get("inputs")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if !inputs.is_null() {
                    tracing::debug!(job_id = %job_id, inputs = ?inputs, "Running job with pre-populated inputs");
                }

                let title = format!("[{}] {}", job.name, job.description);
                let mut metadata = serde_json::json!({
                    "source": "packs_job",
                    "job_id": job.job_id,
                    "pack_id": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "engineering",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "lens": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "coding",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                    },
                    "primary_role": job.primary_role,
                    "inputs": inputs,
                });
                if let Some(ref sid) = session_id {
                    metadata["session_id"] = serde_json::json!(sid);
                }
                if let Some(ref wid) = workspace_id {
                    metadata["workspace_id"] = serde_json::json!(wid);
                }

                let cmd = CreateTask {
                    title,
                    metadata: Some(metadata),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => {
                        if let Some(ref sid) = session_id {
                            let _ = self
                                .session_manager
                                .append_user_message(
                                    &SessionId(sid.clone()),
                                    &format!(
                                        "Started workbench job `{}` (Task {})",
                                        job.job_id, task.id
                                    ),
                                )
                                .await;
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "task_id": task.id,
                                "job_id": job.job_id,
                                "status": "created",
                                "lens": job.lens,
                                "primary_role": job.primary_role,
                                "recommended_topology": job.recommended_topology,
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
