use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) fn handle_local_models_folders(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.local_models.folders.list" => {
                let repo = match self.providers.repo.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match serde_json::to_value(&folders) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.add" => {
                let repo = match self.providers.repo.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let folder_path = match req.params.get("folder_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing folder_path parameter"),
                };
                let path = std::path::Path::new(folder_path);
                if !path.exists() || !path.is_dir() {
                    return ApiResponse::error(
                        req.id,
                        format!("Path does not exist or is not a directory: {}", folder_path),
                    );
                }
                let label = req
                    .params
                    .get("label")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let now = chrono::Utc::now().timestamp_millis();
                let folder = custos_domain::LocalModelFolder {
                    id: custos_domain::new_id("fld"),
                    folder_path: folder_path.to_string(),
                    label,
                    is_active: true,
                    auto_scan: true,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = repo.save_local_folder(&folder) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                let hardware = custos_provider::local::metal_probe::probe_local_hardware();
                let discovered =
                    custos_provider::local::folder_scanner::scan_folder_for_models(&folder, &hardware);
                for m in &discovered {
                    if let Err(error) = repo.upsert_cached_model(m) {
                        return ApiResponse::error(
                            req.id,
                            format!("Cannot save discovered local model: {error}"),
                        );
                    }
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "folder": folder,
                        "models_discovered": discovered.len(),
                        "models": discovered
                    }),
                )
            }
            "v1.local_models.folders.remove" => {
                let repo = match self.providers.repo.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match repo.delete_local_folder(id) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "id": id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.scan" => {
                let repo = match self.providers.repo.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let hardware = custos_provider::local::metal_probe::probe_local_hardware();
                let mut total_discovered = Vec::new();
                for f in &folders {
                    if let Some(fid) = folder_id {
                        if f.id != fid {
                            continue;
                        }
                    }
                    if !f.is_active {
                        continue;
                    }
                    let found =
                        custos_provider::local::folder_scanner::scan_folder_for_models(f, &hardware);
                    for m in &found {
                        if let Err(error) = repo.upsert_cached_model(m) {
                            return ApiResponse::error(
                                req.id,
                                format!("Cannot save discovered local model: {error}"),
                            );
                        }
                    }
                    total_discovered.extend(found);
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "scanned_folders": folders.len(),
                        "models_count": total_discovered.len(),
                        "models": total_discovered,
                        "hardware": hardware,
                        "scanned_at": chrono::Utc::now().timestamp_millis()
                    }),
                )
            }
            "v1.local_models.list" => {
                let repo = match self.providers.repo.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                match repo.list_cached_models(folder_id) {
                    Ok(mut models) => {
                        models.retain(|model| std::path::Path::new(&model.file_path).is_file());
                        ApiResponse::success(req.id, serde_json::json!({ "models": models }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.hardware" => {
                let hardware = custos_provider::local::metal_probe::probe_local_hardware();
                match serde_json::to_value(&hardware) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
