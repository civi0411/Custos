//! Model catalog discovery, capability probing, pricing, and token usage records.

use super::default_model_probe_base_url;
use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_providers_models(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.models.probe" => {
                let provider_id = req
                    .params
                    .get("provider_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string());

                let provider_record = if let (Some(repo), Some(pid)) =
                    (self.providers.repo.as_ref(), provider_id.as_deref())
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
                } else if self.providers.credentials.is_some() {
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
                        let settings = match custos_adapters::connectors::oauth::oauth_settings(
                            &tok.service_type,
                        ) {
                            Ok(v) => v,
                            Err(e) => return ApiResponse::error(req.id, e.to_string()),
                        };
                        let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
                            &settings.0,
                            &settings.1,
                        );
                        match manager.refresh_token(&settings.2, tok).await {
                            Ok(refreshed) => {
                                if self.providers.credentials.is_some() {
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
                            custos_provider::routing::probe::probe_copilot_models(token).await
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
                                custos_provider::cloud::google_codeassist::probe_codeassist_models(
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
                                custos_provider::cloud::google_codeassist::probe_codeassist_models(
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
                                custos_provider::cloud::google_codeassist::probe_codeassist_models(
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
                            custos_provider::routing::probe::probe_oauth_models(token).await
                        }
                        None => Err(custos_domain::DomainError::Validation(
                            "OAuth credentials missing".into(),
                        )),
                    }
                } else {
                    custos_provider::routing::probe::probe_endpoint_models(
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
                            custos_provider::routing::probe::conversation_model_is_visible(
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
                                if provider_type == "antigravity" && custos_provider::cloud::google_codeassist::supports_tiered_reasoning(&model.id) { continue; }
                                model.efforts.clear();
                                model.default_effort = None;
                            }
                        }
                        if let (Some(repo), Some(pid)) =
                            (self.providers.repo.as_ref(), provider_id.as_deref())
                        {
                            let catalog_result = self
                                .providers.model_catalog
                                .merge_probed_models(provider_type, models.clone());
                            let cache_key = custos_provider::routing::probe::verified_catalog_key(
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
                            if let Err(e) = self.providers.model_availability.refresh(&cache_key) {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            unavailable_models = match self.providers.model_availability.snapshot(&cache_key)
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

                if let (Some(repo), Some(pid)) = (self.providers.repo.as_ref(), provider_id) {
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
                        custos_provider::routing::probe::verified_catalog_key(
                            pid, endpoint, false, 0, service,
                        )
                    };
                    let fetched_at = match repo.catalog_fetched_at(&cache_key) {
                        Ok(timestamp) => timestamp,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    return match repo.list_catalog_models(Some(&cache_key)) {
                        Ok(mut cached) => {
                            let unavailable = match self.providers.model_availability.snapshot(&cache_key) {
                                Ok(value) => value,
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            };
                            cached.retain(|model| {
                                custos_provider::routing::probe::conversation_model_is_visible(
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
                let catalog = self.providers.model_catalog.get_catalog(provider_type);
                // Scanned files are inventory, not composed inference routes.
                // Exposing them as `local/<file_name>` sends them to the HTTP/Ollama adapter.
                match serde_json::to_value(&catalog) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.providers.usage" => {
                let Some(store) = &self.kernel.usage_runs else {
                    return ApiResponse::error(req.id, "Usage store unavailable");
                };
                let tasks = match self.kernel.task_service.list_tasks().await {
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
                    if let Some(repo) = &self.providers.repo {
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
                                            let refreshed = match custos_adapters::connectors::oauth::oauth_settings(&token.service_type) {
                                                Ok(settings) => custos_adapters::connectors::oauth::OAuthPkceManager::new(&settings.0,&settings.1).refresh_token(&settings.2,&token).await,
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
                                        match custos_provider::routing::probe::probe_account_quota(&provider.service_type,&token.access_token).await {
                                            Ok(value) => {
                                                if value["status"] == "available" && value["windows"].as_array().is_some_and(|windows| !windows.is_empty() && windows.iter().all(|window| window["used_percent"].as_f64().is_some_and(|used| (0.0..100.0).contains(&used)))) {
                                                    if let Ok(scope) = crate::runtime::provider_catalog_scope(repo, &provider.id) { if let Err(error) = self.providers.model_availability.quota_recovered(&scope) { tracing::debug!(%error, "Quota recovery projection failed"); } }
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

                let pricing = custos_provider::catalog::lookup_model_pricing(model_id);
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
                    custos_provider::catalog::calculate_token_cost(
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
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
