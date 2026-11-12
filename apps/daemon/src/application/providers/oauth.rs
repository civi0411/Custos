//! OAuth PKCE authorization, exchange, token refresh, and loopback callback handling.

use super::default_model_probe_base_url;
use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_providers_oauth(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
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
                    let server = match self.providers.oauth_callback_server.as_ref() {
                        Some(v) => v,
                        None => {
                            return ApiResponse::error(
                                req.id,
                                "OAuth callback server not configured",
                            )
                        }
                    };
                    return match custos_adapters::connectors::oauth::start_copilot_device()
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
                let settings = match custos_adapters::connectors::oauth::oauth_settings(service)
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
                        custos_adapters::connectors::oauth::DEFAULT_REDIRECT_URI
                    },
                );
                let scope = params.scope.as_deref().or(Some(settings.3.as_str()));

                let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
                    auth_url, token_url,
                );
                let challenge = custos_adapters::connectors::oauth::OAuthPkceManager::generate_pkce_challenge();

                match manager.build_authorization_url(client_id, redirect_uri, scope, &challenge) {
                    Ok(res) => {
                        // Start the loopback callback server on port 1455 if available
                        if let Some(ref cb_server) = self.providers.oauth_callback_server {
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
                if self.providers.repo.is_none() {
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
                    match custos_adapters::connectors::oauth::oauth_settings(service_type) {
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
                        custos_adapters::connectors::oauth::DEFAULT_REDIRECT_URI
                    },
                );
                let token_url = params.token_endpoint.as_deref().unwrap_or(&settings.1);

                let callback_server = match self.providers.oauth_callback_server.as_ref() {
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
                let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
                    custos_adapters::connectors::oauth::DEFAULT_OPENAI_AUTH_URL,
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
                if self.providers.repo.is_none() {
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

                let settings = match custos_adapters::connectors::oauth::oauth_settings(
                    &existing.service_type,
                ) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let client_id = params.client_id.as_deref().unwrap_or(&settings.2);
                let token_url = params.token_endpoint.as_deref().unwrap_or(&settings.1);
                let manager = custos_adapters::connectors::oauth::OAuthPkceManager::new(
                    custos_adapters::connectors::oauth::DEFAULT_OPENAI_AUTH_URL,
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
                if self.providers.repo.is_none() {
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
                if self.providers.repo.is_none() {
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
                if let Some(server) = self.providers.oauth_callback_server.as_ref() {
                    server.cancel();
                }
                ApiResponse::success(req.id, serde_json::json!({"cancelled":true}))
            }
            METHOD_OAUTH_STATUS => {
                if let Some(ref cb_server) = self.providers.oauth_callback_server {
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
