//! packs application use cases over the shared daemon context.

use super::*;

impl LocalApiDispatcher {
    pub(crate) fn require_pack_enabled(
        &self,
        pack_id: &str,
        req_id: &str,
    ) -> Result<(), ApiResponse> {
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
        if let Some(store) = self.sqlite_store.as_ref() {
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
        if self.pack_registry.has_pack(canonical) {
            Ok(())
        } else {
            Err(ApiResponse::error(
                req_id,
                format!("pack_disabled: The '{pack_id}' workbench is not installed or enabled on this profile"),
            ))
        }
    }

    pub(crate) async fn handle_packs(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "packages.list_inventory"
            | "v1.packages.list_inventory"
            | "custos.packages.list_inventory" => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                match store.packages().list_inventory() {
                    Ok(items) => {
                        let json_items: Vec<_> = items
                            .into_iter()
                            .map(|item| {
                                serde_json::json!({
                                    "component_id": item.component_id,
                                    "version": item.version,
                                    "channel": item.channel,
                                    "state": item.state,
                                    "platform": item.platform,
                                    "artifact_digest": item.artifact_digest,
                                    "installed_path": item.installed_path,
                                    "installed_at": item.installed_at,
                                    "updated_at": item.updated_at,
                                })
                            })
                            .collect();
                        ApiResponse::success(req.id, serde_json::json!({ "items": json_items }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "packages.install" | "v1.packages.install" | "custos.packages.install" => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let component_id = match req.params.get("component_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'component_id' parameter"),
                };
                let version = req
                    .params
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0");
                let platform = req
                    .params
                    .get("platform")
                    .and_then(|v| v.as_str())
                    .unwrap_or(std::env::consts::OS);
                let source_url = req.params.get("source_url").and_then(|v| v.as_str());
                let expected_digest = req.params.get("expected_digest").and_then(|v| v.as_str());
                let command_id = format!(
                    "cmd_install_{}_{}",
                    component_id,
                    custos_domain::new_id("inst")
                );

                // 1. Record activation intent
                if let Err(e) = store.packages().record_activation_intent(
                    component_id,
                    version,
                    platform,
                    &command_id,
                ) {
                    return ApiResponse::error(
                        req.id,
                        format!("Failed to record activation intent: {e}"),
                    );
                }

                let root_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from(".custos"));

                // 2. Physical transfer & validation
                let (installed_path_str, actual_digest_str) = if let (Some(url), Some(digest)) =
                    (source_url, expected_digest)
                {
                    let adapter =
                        custos_adapters::packages::LocalPackageTransferAdapter::new(&root_dir);
                    use custos_core::contracts::packages::PackageTransferPort;
                    let staged = match adapter.download_to_staging(url, component_id, None).await {
                        Ok(staged) => staged,
                        Err(e) => {
                            let _ = store.packages().mark_failed(
                                component_id,
                                version,
                                platform,
                                &e.to_string(),
                            );
                            return ApiResponse::error(req.id, format!("Download failed: {e}"));
                        }
                    };
                    if let Err(e) = adapter.verify_staged(&staged, digest).await {
                        let _ = store.packages().mark_failed(
                            component_id,
                            version,
                            platform,
                            &e.to_string(),
                        );
                        return ApiResponse::error(
                            req.id,
                            format!("Digest verification failed: {e}"),
                        );
                    }
                    let actual_digest = staged.actual_digest.clone();
                    match adapter.promote_staged(staged, component_id, version).await {
                        Ok(installed) => (installed.0.display().to_string(), Some(actual_digest)),
                        Err(e) => {
                            let _ = store.packages().mark_failed(
                                component_id,
                                version,
                                platform,
                                &e.to_string(),
                            );
                            return ApiResponse::error(req.id, format!("Promotion failed: {e}"));
                        }
                    }
                } else {
                    // Check if package already exists physically on disk with a valid bundle/worker/manifest
                    let target_dir = root_dir.join("packages").join(component_id).join(version);
                    let bundle = target_dir.join("package.bundle");
                    let worker = target_dir.join("custos-worker");
                    let manifest = target_dir.join("manifest.json");
                    let verified_path = if bundle.exists() {
                        Some(bundle)
                    } else if worker.exists() {
                        Some(worker)
                    } else if manifest.exists() {
                        Some(manifest)
                    } else {
                        None
                    };

                    if let Some(file_path) = verified_path {
                        let bytes = match std::fs::read(&file_path) {
                            Ok(b) => b,
                            Err(e) => {
                                let _ = store.packages().mark_failed(
                                    component_id,
                                    version,
                                    platform,
                                    &e.to_string(),
                                );
                                return ApiResponse::error(
                                    req.id,
                                    format!("Failed reading verified local artifact: {e}"),
                                );
                            }
                        };
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(&bytes);
                        let digest = format!("sha256:{:x}", hasher.finalize());
                        (target_dir.display().to_string(), Some(digest))
                    } else {
                        let reason = format!(
                            "Cannot install package '{component_id}': no source URL or digest specified and no verified local package found in {}",
                            target_dir.display()
                        );
                        let _ =
                            store
                                .packages()
                                .mark_failed(component_id, version, platform, &reason);
                        return ApiResponse::error(req.id, reason);
                    }
                };

                // 3. Commit installed in DB
                if let Err(e) = store.packages().commit_installed(
                    component_id,
                    version,
                    platform,
                    actual_digest_str.as_deref(),
                    &installed_path_str,
                ) {
                    return ApiResponse::error(
                        req.id,
                        format!("Failed to commit package installation: {e}"),
                    );
                }

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "status": "installed",
                        "component_id": component_id,
                        "version": version,
                        "command_id": command_id,
                        "installed_path": installed_path_str,
                    }),
                )
            }
            "packages.cancel_install"
            | "v1.packages.cancel_install"
            | "custos.packages.cancel_install" => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let component_id = match req.params.get("component_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'component_id' parameter"),
                };
                let version = req
                    .params
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0");
                let platform = req
                    .params
                    .get("platform")
                    .and_then(|v| v.as_str())
                    .unwrap_or(std::env::consts::OS);
                if let Err(e) = store.packages().mark_failed(
                    component_id,
                    version,
                    platform,
                    "Installation cancelled by user",
                ) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "status": "cancelled", "component_id": component_id }),
                )
            }
            "packages.enable" | "v1.packages.enable" | "custos.packages.enable" => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let component_id = match req.params.get("component_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'component_id' parameter"),
                };
                if let Err(e) = store.packages().enable(component_id) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "status": "enabled", "component_id": component_id }),
                )
            }
            "packages.disable" | "v1.packages.disable" | "custos.packages.disable" => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let component_id = match req.params.get("component_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'component_id' parameter"),
                };
                if let Err(e) = store.packages().disable(component_id) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "status": "disabled", "component_id": component_id }),
                )
            }
            METHOD_PACKS_MANIFESTS_LIST
            | "packs.manifests.list"
            | "custos.packs.manifests.list" => {
                let manifests = self.pack_registry.list_packs();
                match serde_json::to_value(&manifests) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PACKS_JOBS_LIST | "packs.jobs.list" | "custos.packs.jobs.list" => {
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
                            custos_domain::WorkbenchLens::Quant => {
                                fl.eq_ignore_ascii_case("quant")
                                    || fl.eq_ignore_ascii_case("trading")
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

                match self.pack_registry.get_job(job_id) {
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
            METHOD_PACKS_JOBS_RUN | "packs.jobs.run" | "custos.packs.jobs.run" => {
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
                        custos_domain::WorkbenchLens::Quant => "quant",
                    },
                    "lens": match job.lens {
                        custos_domain::WorkbenchLens::Coding => "coding",
                        custos_domain::WorkbenchLens::Research => "research",
                        custos_domain::WorkbenchLens::Assistant => "assistant",
                        custos_domain::WorkbenchLens::Quant => "quant",
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
