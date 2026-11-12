//! Provider account CRUD, routing policies, API keys, and LLM status.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_providers_crud(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.providers.list" => {
                let providers_repo = match self.providers.repo.as_ref() {
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
                let providers_repo = match self.providers.repo.as_ref() {
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
                if self.providers.repo.is_none() {
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
                let repo = match self.providers.repo.as_ref() {
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
                            &self.providers.model_availability,
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
                let providers_repo = match self.providers.repo.as_ref() {
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
            "v1.keys.list" => {
                let providers_repo = match self.providers.repo.as_ref() {
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
                let providers_repo = match self.providers.repo.as_ref() {
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
                let providers_repo = match self.providers.repo.as_ref() {
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

                let oauth_tokens = if self.providers.credentials.is_some() {
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

                let configured_provider = if let Some(ref repo) = self.providers.repo {
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
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
