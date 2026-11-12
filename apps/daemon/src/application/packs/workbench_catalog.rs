use super::super::*;

impl LocalApiDispatcher {
    #[allow(dead_code)]
    pub(crate) fn require_pack_enabled(&self, pack_id: &str, req_id: &str) -> Result<(), ApiResponse> {
        let canonical = match pack_id {
            "quant" => "quant",
            "research" => "research",
            "coding" | "engineering" => "engineering",
            "assistant" | "copilot" => "assistant",
            _ => {
                return Err(ApiResponse::error(
                    req_id,
                    format!("Unknown pack: {pack_id}"),
                ))
            }
        };
        // Check dynamic enablement in persistent store first
        if let Some(store) = self.kernel.sqlite_store.as_ref() {
            if let Ok(enabled_components) = store.packages().list_enabled_components() {
                let is_enabled = enabled_components.iter().any(|c| match c.as_str() {
                    "quant" if canonical == "quant" => true,
                    "research" if canonical == "research" => true,
                    "coding" | "engineering" if canonical == "engineering" => true,
                    "assistant" | "copilot" if canonical == "assistant" => true,
                    other => other == canonical,
                });
                if !is_enabled {
                    return Err(ApiResponse::error(
                        req_id,
                        format!("pack_disabled: The '{pack_id}' workbench is not installed or enabled on this profile"),
                    ));
                }
            }
        }
        if self.workbenches.pack_registry.has_pack(canonical) {
            Ok(())
        } else {
            Err(ApiResponse::error(
                req_id,
                format!("pack_disabled: The '{pack_id}' workbench is not installed or enabled on this profile"),
            ))
        }
    }

    pub(crate) async fn handle_workbench_catalog(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_PACKS_MANIFESTS_LIST
            | "packs.manifests.list"
            | "custos.packs.manifests.list" => {
                let manifests = self.workbenches.pack_registry.list_packs();
                match serde_json::to_value(&manifests) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_LIST | "packs.jobs.list" | "custos.packs.jobs.list" => {
                let filter_lens = req.params.get("lens").and_then(|v| v.as_str());
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let all_jobs = self.workbenches.pack_registry.list_all_jobs();
                let mut catalog_items = Vec::new();

                for job in all_jobs {
                    if let Some(fl) = filter_lens {
                        let matches_lens = match &job.lens {
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
                            custos_domain::WorkbenchLens::Quant => {
                                fl.eq_ignore_ascii_case("quant")
                                    || fl.eq_ignore_ascii_case("trading")
                            }
                            custos_domain::WorkbenchLens::Custom(id) => {
                                fl.eq_ignore_ascii_case(id.as_str())
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
                        | "assistant__schedule_proposal"
                        | "quant__alpha_discover"
                        | "quant__backtest_full"
                        | "quant__risk_audit"
                        | "quant__execution_plan" => (JobOperationalStatus::Available, None),

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
            METHOD_PACKS_JOBS_GET | "packs.jobs.get" | "custos.packs.jobs.get" => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                match self.workbenches.pack_registry.get_job(job_id) {
                    Some(job) => {
                        let (status, reason) = match job.job_id.as_str() {
                            "engineering__repo_explain"
                            | "engineering__review"
                            | "research__source_qa"
                            | "research__dataset_audit"
                            | "research__reproduce"
                            | "assistant__inbox_triage"
                            | "assistant__schedule_proposal"
                            | "quant__alpha_discover"
                            | "quant__backtest_full"
                            | "quant__risk_audit"
                            | "quant__execution_plan" => (JobOperationalStatus::Available, None),

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
            METHOD_PACKS_ROLES_LIST | "packs.roles.list" | "custos.packs.roles.list" => {
                let filter_pack = req.params.get("pack_id").and_then(|v| v.as_str());
                let roles = if let Some(pid) = filter_pack {
                    match self.workbenches.pack_registry.get_pack(pid) {
                        Some(pack) => pack.roles(),
                        None => {
                            return ApiResponse::error(
                                req.id,
                                format!("Domain pack '{pid}' not found"),
                            )
                        }
                    }
                } else {
                    self.workbenches.pack_registry.list_all_roles()
                };

                match serde_json::to_value(&roles) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_RUN | "packs.jobs.run" | "custos.packs.jobs.run" => {
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing job_id parameter"),
                };

                let job = match self.workbenches.pack_registry.get_job(job_id) {
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
                    "pack_id": match &job.lens {
                        custos_domain::WorkbenchLens::Coding => "engineering",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                        custos_domain::WorkbenchLens::Quant => "quant",
                        custos_domain::WorkbenchLens::Custom(id) => id.as_str(),
                    },
                    "lens": job.lens.as_str(),
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

                match self.kernel.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => {
                        if let Some(ref sid) = session_id {
                            let _ = self
                                .kernel.session_manager
                                .append_user_message(
                                    &SessionId(sid.clone()),
                                    &format!(
                                        "Prepared workbench job `{}` (Task {})",
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
