use super::super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_package_management(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "packages.list_inventory"
            | "v1.packages.list_inventory"
            | "custos.packages.list_inventory" => {
                let store = match self.kernel.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                match store.packages().list_inventory() {
                    Ok(items) => {
                        let json_items: Vec<_> = items
                            .into_iter()
                            .map(|item| {
                                let is_enabled = (item.state == "builtin" || item.state == "installed")
                                    && item.enabled_in_profile.as_deref() != Some("0");
                                serde_json::json!({
                                    "component_id": item.component_id,
                                    "version": item.version,
                                    "channel": item.channel,
                                    "state": item.state,
                                    "platform": item.platform,
                                    "artifact_digest": item.artifact_digest,
                                    "installed_path": item.installed_path,
                                    "enabled_in_profile": item.enabled_in_profile,
                                    "is_enabled": is_enabled,
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
                let store = match self.kernel.sqlite_store.as_ref() {
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
                        custos_adapters::host::packages::installer::LocalPackageTransferAdapter::new(&root_dir);
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

                // 3. Commit installed in DB and register installed module on demand
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
                Self::ensure_module_registered(component_id);

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
                let store = match self.kernel.sqlite_store.as_ref() {
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
                let store = match self.kernel.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let component_id = match req.params.get("component_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'component_id' parameter"),
                };
                if let Ok(inv) = store.packages().list_inventory() {
                    if inv.iter().any(|item| item.component_id == component_id) {
                        Self::ensure_module_registered(component_id);
                    }
                }
                if let Err(err_msg) = self.validate_pack_capabilities(component_id) {
                    return ApiResponse::error(req.id, err_msg);
                }
                if let Err(e) = store.packages().enable(component_id) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                // SADE Zero-Footprint Dynamic Activation: register into memory pack_registry and attach storage repo lazily
                if let Some(pack) = custos_packs::resolve_pack_by_id(component_id) {
                    self.workbenches.pack_registry.register(pack);
                }
                self.activate_workbench_resources(component_id);
                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "status": "enabled", "component_id": component_id }),
                )
            }
            "packages.disable" | "v1.packages.disable" | "custos.packages.disable" => {
                let store = match self.kernel.sqlite_store.as_ref() {
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
                // SADE Zero-Footprint Dynamic Deactivation: remove from pack_registry and release RAM/Python resources
                let canonical = match component_id {
                    "coding" => "engineering",
                    other => other,
                };
                self.workbenches.pack_registry.unregister(canonical);
                self.workbenches.pack_registry.unregister(component_id);
                self.release_workbench_resources(canonical).await;
                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "status": "disabled", "component_id": component_id }),
                )
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
