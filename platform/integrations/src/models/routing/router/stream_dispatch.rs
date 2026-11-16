use custos_domain::DomainError;
use custos_provider::ProviderRequest;

use super::catalog::{find_by_service, strip_model_prefix};
use super::classification::classify_provider;
use super::classification::responses_payload;
use super::provider::RouterModelProvider;
use crate::routing::streaming::Wire;

impl RouterModelProvider {
    pub(crate) async fn send_stream_request(
        &self,
        req: &ProviderRequest,
    ) -> Result<(reqwest::Response, Wire, String, String), DomainError> {
        if let Some(account) = &req.provider_account_id {
            self.resolver
                .validate_model(account, &req.model, req.reasoning_effort.as_deref())
                .await?;
        }
        let model = req.model.trim();
        let service = classify_provider(model);
        if service == "native-local" {
            return Err(DomainError::Validation(
                "Native local does not use HTTP stream request".into(),
            ));
        }
        let service_override = match req.provider_account_id.as_deref() {
            Some(account) => self.resolver.service_for_account(account).await?,
            None => None,
        };
        let service = if model.starts_with("gemini-oauth/") {
            "gemini-oauth"
        } else {
            service_override
                .as_deref()
                .unwrap_or_else(|| classify_provider(model))
        };
        let account = req
            .provider_account_id
            .as_deref()
            .unwrap_or(if service == "gemini-oauth" {
                "gemini"
            } else {
                service
            });
        if req.provider_account_id.is_some() {
            self.resolver
                .validate_account(
                    account,
                    if service == "gemini-oauth" {
                        "gemini"
                    } else {
                        service
                    },
                )
                .await?;
        }
        if matches!(
            service,
            "google" | "antigravity" | "gemini-oauth" | "gemini"
        ) {
            let token = self
                .resolver
                .get_token(account)
                .await?
                .ok_or_else(|| DomainError::Validation("Connect Google OAuth first".into()))?;
            let base = self
                .resolver
                .get_endpoint_url(account)
                .await?
                .unwrap_or_else(|| "https://cloudcode-pa.googleapis.com".into());
            let codeassist_mode = if model.starts_with("gemini-oauth/") || service == "gemini" {
                "gemini"
            } else {
                "antigravity"
            };
            let response = self
                .observe_adapter_error(
                    crate::cloud::google_codeassist::stream_codeassist(codeassist_mode, &base, &token, req)
                        .await,
                    req,
                )
                .await?;
            return Ok((
                response,
                Wire::Google,
                service.to_string(),
                model.to_string(),
            ));
        }
        let local_base = if service == "local" {
            self.resolver
                .get_endpoint_url(account)
                .await?
                .unwrap_or_else(|| "http://localhost:11434".into())
        } else {
            String::new()
        };
        if service == "local" && (model.starts_with("ollama/") || local_base.contains(":11434")) {
            let root = crate::routing::probe::ollama_root(&local_base)?;
            let clean_model = model.strip_prefix("ollama/").unwrap_or(model);
            if clean_model.is_empty() || matches!(clean_model, "default" | "local" | "ollama") {
                return Err(DomainError::Validation(
                    "Select an installed Ollama model".into(),
                ));
            }
            let mut options = serde_json::Map::new();
            if let Some(v) = req.temperature {
                options.insert("temperature".into(), serde_json::json!(v));
            }
            if let Some(v) = req.max_tokens {
                options.insert("num_predict".into(), serde_json::json!(v));
            }
            let request = self.http_client.post(format!("{root}/api/chat")).json(&serde_json::json!({
                "model": clean_model,
                "messages": [{"role": "user", "content": req.prompt}],
                "stream": true,
                "options": options
            }));
            let response = self.checked_response(request.send().await, req).await?;
            if !response.status().is_success() {
                return Err(DomainError::Validation(format!(
                    "Ollama chat returned {}",
                    response.status()
                )));
            }
            return Ok((
                response,
                Wire::Ollama,
                "ollama".to_string(),
                clean_model.to_string(),
            ));
        }
        let oauth = service == "openai" && self.resolver.uses_oauth(account).await?;
        let token = if req.provider_account_id.is_some() || oauth || service == "copilot" {
            self.resolver.get_token(account).await?
        } else {
            self.resolve_token_with_fallback(account).await?
        };
        if token.is_none() && service != "local" && service != "lmstudio" {
            return Err(DomainError::Validation(
                "Provider credentials not configured".into(),
            ));
        }
        let custom = self.resolver.get_endpoint_url(account).await?;
        let mut request;
        let wire;
        let effective_model;
        if service == "openai" && oauth {
            wire = Wire::Codex;
            let m = if matches!(model, "" | "default" | "openai" | "codex" | "openai-codex") {
                "gpt-5.6-sol"
            } else {
                model
            };
            effective_model = m.to_string();
            request = self
                .http_client
                .post(&self.codex_endpoint)
                .bearer_auth(token.as_deref().unwrap_or(""))
                .header("User-Agent", crate::routing::probe::codex_user_agent())
                .header("Originator", "codex_cli_rs")
                .header("Accept", "text/event-stream")
                .json(&responses_payload(req, m));
            if let Some(id) =
                crate::cloud::transport::extract_chatgpt_account_id(token.as_deref().unwrap_or(""))
            {
                request = request.header("ChatGPT-Account-ID", id);
            }
        } else if service == "openai"
            && custos_provider::model::is_openai_responses_model(model)
        {
            wire = Wire::Codex;
            effective_model = model.to_string();
            let base = custom.unwrap_or_else(|| "https://api.openai.com/v1".into());
            request = self
                .http_client
                .post(format!("{}/responses", base.trim_end_matches('/')))
                .bearer_auth(token.as_deref().unwrap_or(""))
                .json(&responses_payload(req, model));
        } else if service == "anthropic" {
            wire = Wire::Claude;
            let base = custom.unwrap_or_else(|| "https://api.anthropic.com/v1".into());
            let target = if matches!(model, "" | "default" | "anthropic") {
                "claude-3-7-sonnet-20250219"
            } else {
                model
            };
            effective_model = target.to_string();
            let mut payload = serde_json::json!({
                "model": target,
                "messages": [{"role": "user", "content": req.prompt}],
                "max_tokens": req.max_tokens.unwrap_or(4096),
                "stream": true
            });
            if let Some(temperature) = req.temperature {
                payload["temperature"] = serde_json::json!(temperature);
            }
            request = self
                .http_client
                .post(format!("{}/messages", base.trim_end_matches('/')))
                .header("anthropic-version", "2023-06-01")
                .json(&payload);
            if self.resolver.uses_oauth(account).await? {
                request = request
                    .bearer_auth(token.as_deref().unwrap_or(""))
                    .header("anthropic-beta", "oauth-2025-04-20")
                    .header("User-Agent", "claude-cli/2.1.280 (external, cli)")
                    .header("X-App", "cli");
            } else {
                request = request.header("x-api-key", token.as_deref().unwrap_or(""));
            }
        } else {
            wire = Wire::OpenAi;
            let (base, m, bearer) = match service {
                "copilot" => (
                    "https://api.githubcopilot.com".into(),
                    model.strip_prefix("copilot/").unwrap_or(model).to_string(),
                    Some(
                        crate::cloud::transport::copilot_session_token(token.as_deref().unwrap_or(""))
                            .await?,
                    ),
                ),
                "openai" => (
                    custom.unwrap_or_else(|| "https://api.openai.com/v1".into()),
                    if matches!(model, "" | "default" | "openai") {
                        "gpt-4o".into()
                    } else {
                        model.into()
                    },
                    token,
                ),
                "gemini" => (
                    custom.unwrap_or_else(|| {
                        "https://generativelanguage.googleapis.com/v1beta/openai".into()
                    }),
                    model.into(),
                    token,
                ),
                "deepseek" => (
                    custom.unwrap_or_else(|| "https://api.deepseek.com/v1".into()),
                    model.into(),
                    token,
                ),
                "local" => {
                    let clean = model
                        .strip_prefix("local/")
                        .or_else(|| model.strip_prefix("local:"))
                        .unwrap_or(model);
                    (local_base, clean.into(), token)
                }
                "lmstudio" => {
                    let base = custom.unwrap_or_else(|| "http://127.0.0.1:1234/v1".into());
                    let clean = model.strip_prefix("lmstudio/").unwrap_or(model);
                    (base, clean.into(), token)
                }
                _ if let Some(endpoint) = find_by_service(service) => {
                    let base = custom.unwrap_or_else(|| endpoint.default_base_url.to_string());
                    let clean = strip_model_prefix(model, endpoint);
                    (base, clean.into(), token)
                }
                _ if custom.is_some() => {
                    let custom_endpoint = custom.unwrap();
                    let clean_model = if let Some((_, rem)) = model.split_once('/') {
                        rem
                    } else {
                        model
                    };
                    (custom_endpoint, clean_model.into(), token)
                }
                _ => {
                    return Err(DomainError::Validation(format!(
                        "Unsupported streaming provider '{service}'"
                    )))
                }
            };
            effective_model = m.clone();
            let mut payload = serde_json::json!({
                "model": m,
                "messages": [{"role": "user", "content": req.prompt}],
                "stream": true,
                "stream_options": {"include_usage": true}
            });
            if let Some(effort) = &req.reasoning_effort {
                payload["reasoning_effort"] = serde_json::json!(effort);
            }
            if let Some(v) = req.temperature {
                payload["temperature"] = serde_json::json!(v);
            }
            if let Some(v) = req.max_tokens {
                payload["max_tokens"] = serde_json::json!(v);
            }
            request = self
                .http_client
                .post(format!("{}/chat/completions", base.trim_end_matches('/')))
                .json(&payload);
            if let Some(token) = bearer {
                request = request.bearer_auth(token);
            }
        }
        if service == "copilot" {
            request = request
                .header("User-Agent", "GitHubCopilotChat/0.35.0")
                .header("Editor-Version", "vscode/1.107.0")
                .header("Editor-Plugin-Version", "copilot-chat/0.35.0")
                .header("Copilot-Integration-Id", "vscode-chat");
        }
        tracing::debug!(task_id = %req.task_id, request_id = %req.request_id, service, "Starting provider stream");
        let response = self.checked_response(request.send().await, req).await?;
        if !response.status().is_success() {
            return Err(DomainError::Validation(format!(
                "Provider stream returned {}",
                response.status()
            )));
        }
        Ok((response, wire, service.to_string(), effective_model))
    }

    pub(crate) async fn dispatch_stream_attempt(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let service = classify_provider(&req.model);
        if service == "native-local" {
            let local = self.native_local.as_ref().ok_or_else(|| {
                DomainError::Validation(
                    "Native local inference runtime is not configured".to_string(),
                )
            })?;
            return local.stream(req).await;
        }
        let (response, wire, _, _) = self.send_stream_request(req).await?;
        Ok(crate::routing::streaming::stream_response_observed(
            response,
            req.clone(),
            wire,
            self.resolver.clone(),
        ))
    }
}
