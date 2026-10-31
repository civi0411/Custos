//! providers application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_providers(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.providers.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_providers() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.get" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match providers_repo.get_provider(id) {
                    Ok(Some(cfg)) => match serde_json::to_value(&cfg) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Provider {id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.delete" => {
                if self.providers.is_none() {
                    return ApiResponse::error(req.id, "ProviderRepository not configured");
                }
                let id = req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider id parameter");
                }
                match self.remove_provider_credential(id, true).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "id": id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.routing.get" | "v1.providers.routing.save" | "v1.providers.route" => {
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let service = req
                    .params
                    .get("service_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if service.is_empty() {
                    return ApiResponse::error(req.id, "Missing provider service");
                }
                let result = match req.method.as_str() {
                    "v1.providers.route" => {
                        let model = req
                            .params
                            .get("model")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        crate::runtime::reserve_available_provider_account(
                            repo,
                            service,
                            model,
                            req.params.get("reasoning_effort").and_then(|v| v.as_str()),
                            &self.model_availability,
                        )
                        .map(|account| serde_json::json!({"account_id":account}))
                    }
                    "v1.providers.routing.save" => {
                        let enabled = match req.params.get("round_robin").and_then(|v| v.as_bool())
                        {
                            Some(v) => v,
                            None => return ApiResponse::error(req.id, "Missing round_robin mode"),
                        };
                        let order: Vec<String> = match req
                            .params
                            .get("account_order")
                            .cloned()
                            .map(serde_json::from_value)
                        {
                            Some(Ok(v)) => v,
                            _ => return ApiResponse::error(req.id, "Invalid account order"),
                        };
                        repo.save_routing_policy(service, enabled, &order)
                            .and_then(|()| repo.routing_policy(service))
                    }
                    _ => repo.routing_policy(service),
                };
                match result {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.save" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let id = req
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("p_custom")
                    .to_string();
                let existing_provider = match providers_repo.get_provider(&id) {
                    Ok(value) => value,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| {
                        existing_provider
                            .as_ref()
                            .map(|p| p.name.as_str())
                            .unwrap_or(&id)
                    })
                    .to_string();
                let service_type = req
                    .params
                    .get("service_type")
                    .or_else(|| req.params.get("provider_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| {
                        existing_provider
                            .as_ref()
                            .map(|p| p.service_type.as_str())
                            .unwrap_or("anthropic")
                    })
                    .to_string();
                let raw_api_key = req.params.get("api_key").and_then(|v| v.as_str());
                let masked = if raw_api_key.is_some_and(|key| !key.trim().is_empty()) {
                    "••••••••".to_string()
                } else {
                    providers_repo
                        .get_provider(&id)
                        .ok()
                        .flatten()
                        .map(|p| p.api_key_masked)
                        .unwrap_or_else(|| "none".into())
                };
                let status = req
                    .params
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or(if masked != "none" {
                        "configured"
                    } else {
                        "unconfigured"
                    })
                    .to_string();
                let endpoint_url = req
                    .params
                    .get("endpoint_url")
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let default_model = req
                    .params
                    .get("default_model")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let context_window = req.params.get("context_window").and_then(|v| v.as_u64());
                let fast_mode = req.params.get("fast_mode").and_then(|v| v.as_bool());

                let now = chrono::Utc::now().timestamp_millis();

                let cfg = custos_domain::ProviderConfig {
                    id,
                    name,
                    service_type,
                    api_key_masked: masked,
                    status,
                    endpoint_url,
                    default_model,
                    context_window,
                    fast_mode,
                    created_at: now,
                    updated_at: now,
                };
                match self.credential_service() {
                    Ok(service) => match service.save_provider(&cfg, raw_api_key).await {
                        Ok(()) => ApiResponse::success(
                            req.id,
                            serde_json::json!({ "saved": true, "id": cfg.id }),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.models.probe" => {
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string());

                let provider_record = if let (Some(repo), Some(pid)) =
                    (self.providers.as_ref(), provider_id.as_deref())
                {
                    repo.get_provider(pid).ok().flatten()
                } else {
                    None
                };

                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| provider_record.as_ref().map(|p| p.service_type.as_str()))
                    .unwrap_or("local");

                let base_url = req
                    .params
                    .get("base_url")
                    .or_else(|| req.params.get("endpoint_url"))
                    .or_else(|| req.params.get("endpoint"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        provider_record
                            .as_ref()
                            .and_then(|p| p.endpoint_url.as_deref())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    })
                    .or_else(|| default_model_probe_base_url(provider_type).map(str::to_string))
                    .unwrap_or_default();

                if base_url.trim().is_empty() {
                    return ApiResponse::error(
                        req.id,
                        "Missing base_url parameter for models probe",
                    );
                }

                let token_record = if req
                    .params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .is_some_and(|key| !key.trim().is_empty())
                {
                    None
                } else if self.credentials.is_some() {
                    match self
                        .read_provider_token(provider_id.as_deref().unwrap_or(provider_type))
                        .await
                    {
                        Ok(token) => token,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    None
                };
                let explicit_key = req
                    .params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty());
                let oauth = explicit_key.is_none()
                    && token_record.as_ref().is_some_and(|tok| {
                        tok.refresh_token.is_some()
                            || tok.scope.as_deref().is_some_and(|s| !s.trim().is_empty())
                    });
                let mut api_key_owned = explicit_key
                    .map(str::to_string)
                    .or_else(|| token_record.as_ref().map(|tok| tok.access_token.clone()));
                if oauth {
                    if let Some(tok) = token_record.as_ref().filter(|tok| tok.is_expired(120)) {
                        let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(
                            &tok.service_type,
                        ) {
                            Ok(v) => v,
                            Err(e) => return ApiResponse::error(req.id, e.to_string()),
                        };
                        let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                            &settings.0,
                            &settings.1,
                        );
                        match manager.refresh_token(&settings.2, tok).await {
                            Ok(refreshed) => {
                                if self.credentials.is_some() {
                                    if let Err(e) =
                                        self.refresh_provider_token(tok, &refreshed).await
                                    {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                }
                                api_key_owned = Some(refreshed.access_token);
                            }
                            Err(_) => {
                                return ApiResponse::error(
                                    req.id,
                                    "OAuth refresh failed; reconnect the account",
                                )
                            }
                        }
                    }
                }
                let probe = if oauth && provider_type == "copilot" {
                    match api_key_owned.as_deref() {
                        Some(token) => {
                            custos_adapters::providers::probe::probe_copilot_models(token).await
                        }
                        None => Err(custos_domain::DomainError::Validation(
                            "Copilot OAuth credentials missing".into(),
                        )),
                    }
                } else if provider_type == "antigravity" || (oauth && provider_type == "gemini") {
                    match api_key_owned.as_deref() {
                        Some(token) => {
                            let mut combined = Vec::new();
                            let mut seen_ids = std::collections::HashSet::new();
                            if let Ok(models) =
                                custos_adapters::providers::antigravity::probe_codeassist_models(
                                    "antigravity",
                                    "https://cloudcode-pa.googleapis.com",
                                    token,
                                )
                                .await
                            {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) {
                                        combined.push(m);
                                    }
                                }
                            }
                            if let Ok(models) =
                                custos_adapters::providers::antigravity::probe_codeassist_models(
                                    "gemini",
                                    "https://cloudcode-pa.googleapis.com",
                                    token,
                                )
                                .await
                            {
                                for m in models {
                                    if seen_ids.insert(m.id.clone()) {
                                        combined.push(m);
                                    }
                                }
                            }
                            if combined.is_empty() {
                                custos_adapters::providers::antigravity::probe_codeassist_models(
                                    "antigravity",
                                    "https://cloudcode-pa.googleapis.com",
                                    token,
                                )
                                .await
                            } else {
                                Ok(combined)
                            }
                        }
                        None => Err(custos_domain::DomainError::Validation(
                            "Google OAuth credentials missing".into(),
                        )),
                    }
                } else if oauth && matches!(provider_type, "openai" | "codex") {
                    match api_key_owned.as_deref() {
                        Some(token) => {
                            custos_adapters::providers::probe::probe_oauth_models(token).await
                        }
                        None => Err(custos_domain::DomainError::Validation(
                            "OAuth credentials missing".into(),
                        )),
                    }
                } else {
                    custos_adapters::providers::probe::probe_endpoint_models(
                        &base_url,
                        api_key_owned.as_deref(),
                        if oauth && provider_type == "anthropic" {
                            "anthropic-oauth"
                        } else {
                            provider_type
                        },
                    )
                    .await
                };
                match probe {
                    Ok(mut models) => {
                        models.retain(|model| {
                            custos_adapters::providers::probe::conversation_model_is_visible(
                                &model.id,
                                model.name.as_deref(),
                            )
                        });
                        let mut unavailable_models = std::collections::HashMap::new();
                        if !matches!(
                            provider_type,
                            "openai"
                                | "codex"
                                | "anthropic"
                                | "gemini"
                                | "antigravity"
                                | "copilot"
                                | "deepseek"
                                | "local"
                                | "ollama"
                                | "groq"
                                | "openrouter"
                                | "together"
                                | "cerebras"
                                | "perplexity"
                                | "mistral"
                                | "lmstudio"
                                | "custom"
                        ) {
                            models.clear();
                        }
                        if !matches!(provider_type, "openai" | "codex" | "copilot") {
                            for model in &mut models {
                                if provider_type == "antigravity" && custos_adapters::providers::antigravity::supports_tiered_reasoning(&model.id) { continue; }
                                model.efforts.clear();
                                model.default_effort = None;
                            }
                        }
                        if let (Some(repo), Some(pid)) =
                            (self.providers.as_ref(), provider_id.as_deref())
                        {
                            let catalog_result = self
                                .model_catalog
                                .merge_probed_models(provider_type, models.clone());
                            let cache_key = custos_adapters::providers::probe::verified_catalog_key(
                                pid,
                                &base_url,
                                oauth,
                                token_record.as_ref().map(|t| t.created_at).unwrap_or(0),
                                provider_type,
                            );
                            if let Err(e) =
                                repo.save_catalog_models(&cache_key, &catalog_result.models)
                            {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            if let Err(e) = self.model_availability.refresh(&cache_key) {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            unavailable_models = match self.model_availability.snapshot(&cache_key)
                            {
                                Ok(value) => value,
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            };
                            models.retain(|model| {
                                !unavailable_models.contains_key("*")
                                    && !unavailable_models.contains_key(&model.id)
                            });
                        }
                        match serde_json::to_value(&models) {
                            Ok(val) => ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "models": val,
                                    "count": models.len(),
                                    "unavailable_models": unavailable_models,
                                    "base_url": base_url,
                                }),
                            ),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Probe failed: {e}")),
                }
            }
            "v1.models.catalog" => {
                let provider_type = req
                    .params
                    .get("provider_type")
                    .or_else(|| req.params.get("service_type"))
                    .and_then(|v| v.as_str());
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                if let (Some(repo), Some(pid)) = (self.providers.as_ref(), provider_id) {
                    let record = repo.get_provider(pid).ok().flatten();
                    let service = record
                        .as_ref()
                        .map(|p| p.service_type.as_str())
                        .or(provider_type)
                        .unwrap_or("local");
                    let endpoint = record
                        .as_ref()
                        .and_then(|p| p.endpoint_url.as_deref())
                        .filter(|s| !s.trim().is_empty())
                        .or_else(|| default_model_probe_base_url(service))
                        .unwrap_or("");
                    let cache_key = if record.is_some() {
                        match crate::runtime::provider_catalog_scope(repo, pid) {
                            Ok(scope) => scope,
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        }
                    } else {
                        custos_adapters::providers::probe::verified_catalog_key(
                            pid, endpoint, false, 0, service,
                        )
                    };
                    let fetched_at = match repo.catalog_fetched_at(&cache_key) {
                        Ok(timestamp) => timestamp,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    return match repo.list_catalog_models(Some(&cache_key)) {
                        Ok(mut cached) => {
                            let unavailable = match self.model_availability.snapshot(&cache_key) {
                                Ok(value) => value,
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            };
                            cached.retain(|model| {
                                custos_adapters::providers::probe::conversation_model_is_visible(
                                    &model.id,
                                    Some(&model.label),
                                ) && !unavailable.contains_key("*")
                                    && !unavailable.contains_key(&model.id)
                            });
                            for model in &mut cached {
                                model.provider_type = service.to_string();
                            }
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({ "origin": "cached", "models": cached, "fetched_at": fetched_at, "unavailable_models": unavailable }),
                            )
                        }
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let catalog = self.model_catalog.get_catalog(provider_type);
                // Scanned files are inventory, not composed inference routes.
                // Exposing them as `local/<file_name>` sends them to the HTTP/Ollama adapter.
                match serde_json::to_value(&catalog) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.folders.list" => {
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
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
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
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
                let hardware =
                    custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let discovered =
                    custos_adapters::local_inference::folder_scanner::scan_folder_for_models(
                        &folder, &hardware,
                    );
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
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
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
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let folder_id = req.params.get("folder_id").and_then(|v| v.as_str());
                let folders = match repo.list_local_folders() {
                    Ok(f) => f,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let hardware =
                    custos_adapters::local_inference::metal_probe::probe_local_hardware();
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
                        custos_adapters::local_inference::folder_scanner::scan_folder_for_models(
                            f, &hardware,
                        );
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
                let repo = match self.providers.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
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
                let hardware =
                    custos_adapters::local_inference::metal_probe::probe_local_hardware();
                match serde_json::to_value(&hardware) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.profile.get" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => {
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        let hardware =
                            custos_adapters::local_inference::metal_probe::probe_local_hardware();
                        let profile = custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: if hardware.is_apple_silicon {
                                Some(true)
                            } else {
                                None
                            },
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        };
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.profile.save" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let descriptor = match repo.get_cached_model_by_path(model_path) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return ApiResponse::error(req.id, "Model artifact is not registered")
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let context_size = req
                    .params
                    .get("context_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096);
                if context_size < 512 || context_size > descriptor.context_length {
                    return ApiResponse::error(
                        req.id,
                        format!(
                            "context_size must be between 512 and {} for this artifact",
                            descriptor.context_length
                        ),
                    );
                }
                let gpu_mode = req
                    .params
                    .get("gpu_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                if !matches!(gpu_mode, "auto" | "max" | "off" | "manual") {
                    return ApiResponse::error(
                        req.id,
                        "gpu_mode must be auto, max, off, or manual",
                    );
                }
                let gpu_layers = req
                    .params
                    .get("gpu_layers")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(u32::MAX as u64) as u32);
                if gpu_mode == "manual" && gpu_layers.is_none() {
                    return ApiResponse::error(req.id, "manual GPU mode requires gpu_layers");
                }
                let revision = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(current)) => current.revision.saturating_add(1),
                    Ok(None) => 1,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let profile = custos_domain::LocalModelLoadProfile {
                    model_path: model_path.to_string(),
                    revision,
                    context_size,
                    gpu_mode: gpu_mode.to_string(),
                    gpu_layers,
                    eval_batch_size: req
                        .params
                        .get("eval_batch_size")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    cpu_threads: req
                        .params
                        .get("cpu_threads")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    flash_attention: req.params.get("flash_attention").and_then(|v| v.as_bool()),
                    mmap: req
                        .params
                        .get("mmap")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    mlock: req
                        .params
                        .get("mlock")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    idle_timeout_seconds: req
                        .params
                        .get("idle_timeout_seconds")
                        .and_then(|v| v.as_u64()),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                match repo.save_local_model_load_profile(&profile) {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.preflight" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let path = std::path::Path::new(model_path);
                if !path.exists() || !path.is_file() {
                    return ApiResponse::error(
                        req.id,
                        format!("Model file does not exist: {}", model_path),
                    );
                }
                let hardware =
                    custos_adapters::local_inference::metal_probe::probe_local_hardware();
                let descriptor =
                    match custos_adapters::local_inference::gguf_inspector::inspect_gguf_file(
                        path,
                        "preflight",
                        &hardware,
                    ) {
                        Ok(d) => d,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to parse GGUF file: {}", e),
                            )
                        }
                    };
                let requested_context = req
                    .params
                    .get("requested_context")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096)
                    .min(descriptor.context_length);
                let inspected_context = descriptor.context_length.clamp(1, 8192);
                let inspected_kv_bytes = descriptor
                    .total_memory_required
                    .saturating_sub(descriptor.file_size_bytes);
                let estimated_kv_bytes = inspected_kv_bytes
                    .saturating_mul(requested_context)
                    .checked_div(inspected_context)
                    .unwrap_or(inspected_kv_bytes);
                let estimated_memory_bytes = descriptor
                    .file_size_bytes
                    .saturating_add(estimated_kv_bytes);
                let can_load = estimated_memory_bytes < hardware.total_ram_bytes;
                let mut issues = Vec::new();
                if !can_load {
                    issues.push(format!(
                        "Estimated model and KV-cache memory ({} bytes) is at least the detected physical RAM ({} bytes); loading is blocked by the current preflight policy",
                        estimated_memory_bytes, hardware.total_ram_bytes
                    ));
                }
                if descriptor.compatibility == custos_domain::ModelCompatibility::Red {
                    issues.push("Model memory requirements with large context may exceed memory. Offloading or reduced context recommended.".to_string());
                } else if descriptor.compatibility == custos_domain::ModelCompatibility::Yellow {
                    issues.push(
                        "Model may require partial CPU offloading and run with reduced speed"
                            .to_string(),
                    );
                }
                let response = custos_domain::LocalModelPreflightResponse {
                    model_path: model_path.to_string(),
                    can_load,
                    architecture: descriptor.architecture,
                    quantization: descriptor.quantization,
                    file_size_bytes: descriptor.file_size_bytes,
                    context_limit: requested_context,
                    estimated_memory_bytes,
                    free_ram_bytes: hardware.free_ram_bytes,
                    metal_vram_limit_bytes: hardware.metal_vram_limit_bytes,
                    compatibility: descriptor.compatibility,
                    recommended_gpu_layers: None,
                    issues,
                };
                match serde_json::to_value(&response) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.load" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match custos_adapters::local_inference::preload_model_with_profile(
                    model_path, &profile,
                )
                .await
                {
                    Ok(instance) => match serde_json::to_value(&instance) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to load local model: {e}"))
                    }
                }
            }
            "v1.local_models.ensure" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let force_reload = req
                    .params
                    .get("force_reload")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = match self.providers.as_ref() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };

                let instances = custos_adapters::local_inference::list_loaded_instances()
                    .await
                    .unwrap_or_default();
                let clean_model = model_path.strip_prefix("native:").unwrap_or(model_path);
                let existing = instances
                    .into_iter()
                    .find(|inst| inst.model_path == clean_model || inst.model_name == clean_model);

                if let Some(inst) = existing {
                    if !force_reload && inst.profile_revision == profile.revision {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "instance": inst,
                                "drift_detected": false,
                                "reloaded": false
                            }),
                        );
                    }
                }

                match custos_adapters::local_inference::preload_model_with_profile(
                    model_path, &profile,
                )
                .await
                {
                    Ok(instance) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "instance": instance,
                            "drift_detected": false,
                            "reloaded": true
                        }),
                    ),
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to ensure local model: {e}"))
                    }
                }
            }
            "v1.local_models.verify" => {
                let model_opt = req
                    .params
                    .get("model_path")
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str());

                let target_model = match model_opt {
                    Some(m) if !m.trim().is_empty() => m.trim().to_string(),
                    _ => {
                        let instances = custos_adapters::local_inference::list_loaded_instances()
                            .await
                            .unwrap_or_default();
                        match instances.first() {
                            Some(inst) => inst.model_name.clone(),
                            None => {
                                return ApiResponse::error(
                                    req.id,
                                    "No active local model instance is loaded to verify",
                                )
                            }
                        }
                    }
                };

                let clean_model = target_model
                    .strip_prefix("native:")
                    .unwrap_or(&target_model);
                let start = std::time::Instant::now();
                match custos_adapters::local_inference::verify_sidecar_generation(clean_model).await
                {
                    Ok(output) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "verified": true,
                                "model": clean_model,
                                "test_output": output,
                                "latency_ms": latency_ms
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Generation probe failed: {e}")),
                }
            }
            "v1.local_models.unload" => {
                let model_identifier = match req
                    .params
                    .get("model_path")
                    .or_else(|| req.params.get("instance_id"))
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing model_path or instance_id parameter",
                        )
                    }
                };
                let clean_model = model_identifier
                    .strip_prefix("native:")
                    .unwrap_or(model_identifier);
                match custos_adapters::local_inference::evict_model(clean_model).await {
                    Ok(evicted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "evicted": evicted, "model": clean_model }),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to unload model: {}", e)),
                }
            }
            "v1.local_models.instances" => {
                match custos_adapters::local_inference::list_loaded_instances().await {
                    Ok(instances) => {
                        ApiResponse::success(req.id, serde_json::json!({ "instances": instances }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.usage" => {
                let Some(store) = &self.usage_runs else {
                    return ApiResponse::error(req.id, "Usage store unavailable");
                };
                let tasks = match self.task_service.list_tasks().await {
                    Ok(tasks) => tasks,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut records = Vec::new();
                for task in tasks {
                    let runs = match store.list_runs_for_task(&task.id).await {
                        Ok(runs) => runs,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    for run in runs {
                        if run.metadata.get("actual_mode").and_then(|v| v.as_str()) != Some("model")
                        {
                            continue;
                        }
                        if run
                            .metadata
                            .get("model_id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|id| id.starts_with("custos-"))
                        {
                            continue;
                        }
                        let usage = run
                            .metadata
                            .get("usage")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let failure = run
                            .metadata
                            .get("failure_reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        records.push(serde_json::json!({"id":run.id,"account_id":run.metadata.get("provider_account_id"),"model":run.metadata.get("model_id").or_else(||run.metadata.get("requested_model")),"status":run.status.to_string(),"recorded_at":run.started_at,"input_tokens":usage.get("input_tokens"),"output_tokens":usage.get("output_tokens"),"total_tokens":usage.get("total_tokens"),"rate_limited":failure.contains("429") || failure.contains("RESOURCE_EXHAUSTED")}));
                    }
                }
                let mut quotas = Vec::new();
                if req.params.get("include_quota").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(repo) = &self.providers {
                        let providers = match repo.list_providers() {
                            Ok(p) => p,
                            Err(e) => return ApiResponse::error(req.id, e.to_string()),
                        };
                        for provider in providers {
                            let mut snapshot = serde_json::json!({"account_id":provider.id,"status":if provider.service_type == "local" {"not_applicable"} else {"unsupported"},"windows":[],"fetched_at":chrono::Utc::now()});
                            if provider.service_type != "local" {
                                match self.read_provider_token(&provider.id).await {
                                    Ok(Some(mut token))
                                        if token.refresh_token.is_some()
                                            || provider.service_type == "copilot" =>
                                    {
                                        if token.is_expired(120) && token.refresh_token.is_some() {
                                            let refreshed = match custos_adapters::providers::oauth_pkce::oauth_settings(&token.service_type) {
                                                Ok(settings) => custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(&settings.0,&settings.1).refresh_token(&settings.2,&token).await,
                                                Err(e) => Err(e),
                                            };
                                            match refreshed {
                                                Ok(value) => {
                                                    if let Err(e) = self
                                                        .refresh_provider_token(&token, &value)
                                                        .await
                                                    {
                                                        return ApiResponse::error(
                                                            req.id,
                                                            e.to_string(),
                                                        );
                                                    }
                                                    token = value;
                                                }
                                                Err(_) => {
                                                    snapshot["status"] =
                                                        serde_json::json!("unavailable");
                                                    snapshot["error"] = serde_json::json!(
                                                        "Reconnect account to refresh quota"
                                                    );
                                                    quotas.push(snapshot);
                                                    continue;
                                                }
                                            }
                                        }
                                        match custos_adapters::providers::probe::probe_account_quota(&provider.service_type,&token.access_token).await {
                                            Ok(value) => {
                                                if value["status"] == "available" && value["windows"].as_array().is_some_and(|windows| !windows.is_empty() && windows.iter().all(|window| window["used_percent"].as_f64().is_some_and(|used| (0.0..100.0).contains(&used)))) {
                                                    if let Ok(scope) = crate::runtime::provider_catalog_scope(repo, &provider.id) { if let Err(error) = self.model_availability.quota_recovered(&scope) { tracing::debug!(%error, "Quota recovery projection failed"); } }
                                                }
                                                snapshot["status"]=value["status"].clone(); snapshot["windows"]=value["windows"].clone(); snapshot["source"]=value["source"].clone(); },
                                            Err(e) => { snapshot["status"]=serde_json::json!("unavailable"); snapshot["error"]=serde_json::json!(e.to_string()); }
                                        }
                                    }
                                    Err(_) => {
                                        snapshot["status"] = serde_json::json!("unavailable");
                                    }
                                    _ => {}
                                }
                            }
                            quotas.push(snapshot);
                        }
                    }
                }
                tracing::debug!(request_id = %req.id, records = records.len(), quota_accounts = quotas.len(), "Provider usage snapshot loaded");
                ApiResponse::success(
                    req.id,
                    serde_json::json!({"records":records,"quotas":quotas,"fetched_at":chrono::Utc::now()}),
                )
            }
            "v1.models.pricing" => {
                let model_id = req
                    .params
                    .get("model_id")
                    .or_else(|| req.params.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if model_id.trim().is_empty() {
                    return ApiResponse::error(req.id, "Missing model_id parameter");
                }

                let pricing = custos_adapters::providers::pricing::lookup_model_pricing(model_id);
                let input_tokens = req
                    .params
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let output_tokens = req
                    .params
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let cache_read_tokens =
                    req.params.get("cache_read_tokens").and_then(|v| v.as_u64());
                let cache_write_tokens = req
                    .params
                    .get("cache_write_tokens")
                    .and_then(|v| v.as_u64());

                let estimated_cost = pricing.as_ref().map(|p| {
                    custos_adapters::providers::pricing::calculate_token_cost(
                        p,
                        input_tokens,
                        output_tokens,
                        cache_read_tokens,
                        cache_write_tokens,
                    )
                });

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "model_id": model_id,
                        "pricing": pricing,
                        "estimated_cost_usd": estimated_cost,
                    }),
                )
            }
            "v1.keys.list" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                match providers_repo.list_client_keys() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.generate" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Client API Token");
                let key = custos_domain::ClientApiKeyRecord {
                    id: custos_domain::new_id("key"),
                    name: name.to_string(),
                    token: format!("custos_live_sec_{}", custos_domain::new_id("tok")),
                    created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    revoked: false,
                };
                match providers_repo.create_client_key(&key) {
                    Ok(()) => match serde_json::to_value(&key) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.keys.revoke" => {
                let providers_repo = match self.providers.as_ref() {
                    Some(p) => p,
                    None => return ApiResponse::error(req.id, "ProviderRepository not configured"),
                };
                let key_id = match req.params.get("key_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing key_id parameter"),
                };
                match providers_repo.revoke_client_key(key_id) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "revoked": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.llm.status" => {
                let has_env_key = std::env::var("ANTHROPIC_API_KEY").is_ok()
                    || std::env::var("OPENAI_API_KEY").is_ok()
                    || std::env::var("GEMINI_API_KEY").is_ok()
                    || std::env::var("CUSTOS_PROVIDER").is_ok();

                let oauth_tokens = if self.credentials.is_some() {
                    match self.provider_tokens().await {
                        Ok(tokens) => tokens,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    Vec::new()
                };

                let active_oauth = oauth_tokens
                    .into_iter()
                    .find(|t| !t.access_token.trim().is_empty());

                let configured_provider = if let Some(ref repo) = self.providers {
                    repo.list_providers().ok().and_then(|list| {
                        list.into_iter().find(|p| {
                            (p.status == "configured"
                                || p.status == "active"
                                || p.status == "primary")
                                && p.api_key_masked != "none"
                        })
                    })
                } else {
                    None
                };

                if let Some(tok) = active_oauth {
                    let display_name = match tok.provider_id.as_str() {
                        "openai" => "OpenAI (OAuth PKCE)".to_string(),
                        "github-copilot" | "copilot" => "GitHub Copilot (OAuth)".to_string(),
                        "google" | "gemini" => "Google Gemini (OAuth)".to_string(),
                        "antigravity" => "Antigravity Sovereign (OAuth)".to_string(),
                        other => format!("{other} (OAuth)"),
                    };
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": display_name,
                            "message": format!("{display_name} authentication active"),
                            "expires_at": tok.expires_at,
                            "provider_id": tok.provider_id,
                        }),
                    )
                } else if has_env_key || configured_provider.is_some() {
                    let prov_name = configured_provider
                        .map(|p| p.name)
                        .unwrap_or_else(|| "Environment API Key".to_string());
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": prov_name,
                            "message": "LLM sẵn sàng thực thi",
                        }),
                    )
                } else {
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "configured": true,
                            "active_provider": "Custos Sovereign (Local / Offline)",
                            "message": "Custos Offline Sovereign runtime sẵn sàng",
                        }),
                    )
                }
            }
            METHOD_OAUTH_AUTHORIZE => {
                let params: OAuthAuthorizeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let raw_service = params
                    .service_type
                    .as_deref()
                    .or(params.provider_id.as_deref())
                    .unwrap_or("openai");
                let service = raw_service.split('-').next().unwrap_or(raw_service);
                if service == "copilot" {
                    let server = match self.oauth_callback_server.as_ref() {
                        Some(v) => v,
                        None => {
                            return ApiResponse::error(
                                req.id,
                                "OAuth callback server not configured",
                            )
                        }
                    };
                    return match custos_adapters::providers::oauth_pkce::start_copilot_device()
                        .await
                    {
                        Ok(mut device) => {
                            if device.get("device_code").and_then(|v| v.as_str()).is_none()
                                || device.get("user_code").and_then(|v| v.as_str()).is_none()
                            {
                                return ApiResponse::error(
                                    req.id,
                                    "Invalid GitHub device authorization response",
                                );
                            }
                            server.start_copilot_device(
                                device.clone(),
                                params
                                    .provider_id
                                    .clone()
                                    .unwrap_or_else(|| "copilot".into()),
                            );
                            if let Some(object) = device.as_object_mut() {
                                object.remove("device_code");
                            }
                            device["flow_type"] = serde_json::json!("device_code");
                            device["authorization_url"] = device
                                .get("verification_uri")
                                .cloned()
                                .unwrap_or(serde_json::json!("https://github.com/login/device"));
                            ApiResponse::success(req.id, device)
                        }
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    };
                }
                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(service)
                {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let auth_url = params.auth_endpoint.as_deref().unwrap_or(&settings.0);
                let token_url = params.token_endpoint.as_deref().unwrap_or(&settings.1);
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let service_type_or_default = service;
                let redirect_uri = params.redirect_uri.as_deref().unwrap_or(
                    if service_type_or_default == "anthropic" {
                        "https://platform.claude.com/oauth/code/callback"
                    } else if service_type_or_default == "antigravity" {
                        "http://localhost:51121/oauth-callback"
                    } else {
                        custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI
                    },
                );
                let scope = params.scope.as_deref().or(Some(settings.3.as_str()));

                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    auth_url, token_url,
                );
                let challenge = custos_adapters::providers::oauth_pkce::OAuthPkceManager::generate_pkce_challenge();

                match manager.build_authorization_url(client_id, redirect_uri, scope, &challenge) {
                    Ok(res) => {
                        // Start the loopback callback server on port 1455 if available
                        if let Some(ref cb_server) = self.oauth_callback_server {
                            let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                            let service_type = service;
                            let pending = crate::oauth_callback_server::PendingOAuthContext {
                                provider_id: provider_id.to_string(),
                                service_type: service_type.to_string(),
                                client_id: client_id.to_string(),
                                code_verifier: challenge.code_verifier.clone(),
                                state: challenge.state.clone(),
                                redirect_uri: redirect_uri.to_string(),
                                expires_at: chrono::Utc::now().timestamp_millis() + 600_000,
                            };
                            if let Err(e) = cb_server.start_listening(pending).await {
                                tracing::warn!(
                                    provider_id = %provider_id,
                                    "Notice: OAuth loopback callback listener failed to bind: {e}"
                                );
                            }
                        }

                        match serde_json::to_value(&res) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_EXCHANGE => {
                if self.providers.is_none() {
                    return ApiResponse::error(req.id, "ProviderRepository not configured");
                }
                let params: OAuthExchangeParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let provider_id = params.provider_id.as_deref().unwrap_or("openai");
                let raw_service = params
                    .service_type
                    .as_deref()
                    .or(params.provider_id.as_deref())
                    .unwrap_or("openai");
                let service_type = raw_service.split('-').next().unwrap_or(raw_service);
                let settings =
                    match custos_adapters::providers::oauth_pkce::oauth_settings(service_type) {
                        Ok(v) => v,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let service_type_or_default = service_type;
                let redirect_uri = params.redirect_uri.as_deref().unwrap_or(
                    if service_type_or_default == "anthropic" {
                        "https://platform.claude.com/oauth/code/callback"
                    } else if service_type_or_default == "antigravity" {
                        "http://localhost:51121/oauth-callback"
                    } else {
                        custos_adapters::providers::oauth_pkce::DEFAULT_REDIRECT_URI
                    },
                );
                let token_url = params.token_endpoint.as_deref().unwrap_or(&settings.1);

                let callback_server = match self.oauth_callback_server.as_ref() {
                    Some(v) => v,
                    None => {
                        return ApiResponse::error(req.id, "OAuth callback server not configured")
                    }
                };
                if let Err(e) = callback_server.validate_manual_callback(
                    provider_id,
                    &params.code_verifier,
                    redirect_uri,
                    &params.code_or_url,
                ) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager
                    .exchange_code_with_identity(
                        provider_id,
                        service_type,
                        client_id,
                        &params.code_or_url,
                        redirect_uri,
                        &params.code_verifier,
                    )
                    .await
                {
                    Ok((token_record, identity)) => {
                        // Upsert matching provider config so it appears in UI as active
                        let now = chrono::Utc::now().timestamp_millis();
                        let masked = "oauth".to_string();
                        let prov_cfg = custos_domain::ProviderConfig {
                            id: provider_id.to_string(),
                            name: identity.unwrap_or_else(|| format!("{provider_id} (OAuth)")),
                            service_type: service_type.to_string(),
                            api_key_masked: masked,
                            status: "active".to_string(),
                            endpoint_url: if service_type == "gemini" {
                                Some("https://cloudcode-pa.googleapis.com".into())
                            } else {
                                default_model_probe_base_url(service_type).map(str::to_string)
                            },
                            default_model: None,
                            context_window: None,
                            fast_mode: Some(false),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = self
                            .write_provider_connection(&token_record, &prov_cfg)
                            .await
                        {
                            return ApiResponse::error(req.id, e.to_string());
                        }

                        match serde_json::to_value(
                            serde_json::json!({"provider_id":token_record.provider_id,"service_type":token_record.service_type,"connected":true,"expires_at":token_record.expires_at,"scope":token_record.scope,"has_refresh_token":token_record.refresh_token.is_some()}),
                        ) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_REFRESH => {
                if self.providers.is_none() {
                    return ApiResponse::error(req.id, "ProviderRepository not configured");
                }
                let params: OAuthRefreshParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                let existing = match self.read_provider_token(&params.provider_id).await {
                    Ok(Some(tok)) => tok,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("No OAuth token found for {}", params.provider_id),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let settings = match custos_adapters::providers::oauth_pkce::oauth_settings(
                    &existing.service_type,
                ) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let token_url = params.token_endpoint.as_deref().unwrap_or(&settings.1);
                let manager = custos_adapters::providers::oauth_pkce::OAuthPkceManager::new(
                    custos_adapters::providers::oauth_pkce::DEFAULT_OPENAI_AUTH_URL,
                    token_url,
                );

                match manager.refresh_token(client_id, &existing).await {
                    Ok(refreshed) => {
                        if let Err(e) = self.refresh_provider_token(&existing, &refreshed).await {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to update token: {e}"),
                            );
                        }
                        match serde_json::to_value(
                            serde_json::json!({"provider_id":refreshed.provider_id,"service_type":refreshed.service_type,"connected":true,"expires_at":refreshed.expires_at,"scope":refreshed.scope,"has_refresh_token":refreshed.refresh_token.is_some()}),
                        ) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_GET => {
                if self.providers.is_none() {
                    return ApiResponse::error(req.id, "ProviderRepository not configured");
                }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.read_provider_token(provider_id).await {
                    Ok(Some(tok)) => {
                        let sanitized = serde_json::json!({
                            "provider_id": tok.provider_id,
                            "service_type": tok.service_type,
                            "connected": !tok.access_token.is_empty(),
                            "expires_at": tok.expires_at,
                            "token_type": tok.token_type,
                            "scope": tok.scope,
                            "has_refresh_token": tok.refresh_token.is_some(),
                        });
                        ApiResponse::success(req.id, sanitized)
                    }
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_DELETE => {
                if self.providers.is_none() {
                    return ApiResponse::error(req.id, "ProviderRepository not configured");
                }
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai");
                match self.remove_provider_credential(provider_id, false).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "deleted": true, "provider_id": provider_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_OAUTH_CANCEL => {
                if let Some(server) = self.oauth_callback_server.as_ref() {
                    server.cancel();
                }
                ApiResponse::success(req.id, serde_json::json!({"cancelled":true}))
            }
            METHOD_OAUTH_STATUS => {
                if let Some(ref cb_server) = self.oauth_callback_server {
                    let status_val = match cb_server.status() {
                        crate::oauth_callback_server::CallbackServerStatus::Idle => {
                            serde_json::json!({ "status": "idle" })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Listening { port } => {
                            serde_json::json!({ "status": "listening", "port": port })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Exchanging {
                            ..
                        } => {
                            serde_json::json!({ "status": "exchanging" })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Completed {
                            provider_id,
                        } => {
                            serde_json::json!({ "status": "completed", "provider_id": provider_id })
                        }
                        crate::oauth_callback_server::CallbackServerStatus::Failed { error } => {
                            serde_json::json!({ "status": "failed", "error": error })
                        }
                    };
                    ApiResponse::success(req.id, status_val)
                } else {
                    ApiResponse::success(req.id, serde_json::json!({ "status": "idle" }))
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
