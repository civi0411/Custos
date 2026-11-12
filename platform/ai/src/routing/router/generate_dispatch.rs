use custos_domain::DomainError;
use custos_provider::{ModelResponse, ProviderRequest};
use tracing::debug;

use super::catalog::{find_by_service, strip_model_prefix};
use super::classification::classify_provider;
use super::provider::RouterModelProvider;

impl RouterModelProvider {
    pub(crate) async fn dispatch_generate_attempt(
        &self,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        let requested_model = req.model.trim();
        debug!(task_id = %req.task_id, request_id = %req.request_id, model = %requested_model, "Routing model request");
        let service_override = match req.provider_account_id.as_deref() {
            Some(account) => self.resolver.service_for_account(account).await?,
            None => None,
        };
        let target_provider = if requested_model.starts_with("gemini-oauth/") {
            "gemini-oauth"
        } else {
            service_override
                .as_deref()
                .unwrap_or_else(|| classify_provider(requested_model))
        };
        if let Some(account) = &req.provider_account_id {
            let service = if target_provider == "gemini-oauth" {
                "gemini"
            } else {
                target_provider
            };
            self.resolver.validate_account(account, service).await?;
        }

        match target_provider {
            "native-local" => {
                let local = self.native_local.as_ref().ok_or_else(|| {
                    DomainError::Validation("Native local inference runtime is not configured".to_string())
                })?;
                local.generate(req).await
            }
            "copilot" => {
                let github = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("copilot")).await?.ok_or_else(|| DomainError::Validation("Connect GitHub Copilot first".into()))?;
                let token = crate::cloud::transport::copilot_session_token(&github).await?;
                let model = requested_model.strip_prefix("copilot/").ok_or_else(|| DomainError::Validation("Invalid Copilot model".into()))?;
                self.execute_openai_compatible("https://api.githubcopilot.com", Some(&token), model, req, "GitHub Copilot").await
            }
            "gemini-oauth" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("gemini")).await?.ok_or_else(|| DomainError::Validation("Connect Gemini OAuth first".into()))?;
                self.observe_adapter_error(crate::cloud::google_codeassist::generate_codeassist_content("gemini", "https://cloudcode-pa.googleapis.com", &token, req).await, req).await
            }
            "google" | "antigravity" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("google")).await?;
                let token = match token {
                    Some(t) => t,
                    None => self.resolver.get_token("antigravity").await?.ok_or_else(|| DomainError::Validation("Connect Google OAuth first".into()))?,
                };
                let base = self.resolver.get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("google")).await?.unwrap_or_else(|| "https://cloudcode-pa.googleapis.com".into());
                self.observe_adapter_error(crate::cloud::google_codeassist::generate_content(&base, &token, req).await, req).await
            }
            "openai" => {
                let oauth = self
                    .resolver
                    .uses_oauth(req.provider_account_id.as_deref().unwrap_or("openai"))
                    .await?;
                let account = req.provider_account_id.as_deref().unwrap_or("openai");
                let token_res = self.resolver.get_token(account).await;
                let token = if oauth || req.provider_account_id.is_some() {
                    token_res?.filter(|tok| !tok.trim().is_empty())
                } else {
                    match token_res {
                        Ok(Some(tok)) if !tok.trim().is_empty() => Some(tok),
                        _ => self.resolve_token_with_fallback(account).await?,
                    }
                };
                let token = match token {
                    Some(t) => t,
                    None => {
                        return Err(DomainError::Validation(
                            "OpenAI credentials not configured. Please log in with OpenAI via OAuth 2.0 PKCE or enter an API key in Settings > Providers.".into(),
                        ));
                    }
                };
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("openai"))
                    .await
                    .ok()
                    .flatten();
                if oauth {
                    return self.execute_chatgpt_codex(&token, requested_model, req).await;
                }
                let base_url =
                    custom_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let model = if requested_model.is_empty()
                    || requested_model == "default"
                    || requested_model == "openai"
                {
                    "gpt-4o"
                } else {
                    requested_model
                };
                if custos_provider::model::is_openai_responses_model(model) {
                    return self.execute_responses(&token, model, req, &format!("{}/responses", base_url.trim_end_matches('/')), false).await;
                }
                self.execute_openai_compatible(&base_url, Some(&token), model, req, "OpenAI")
                    .await
            }
            "anthropic" => {
                let token = (if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback("anthropic").await?
                })
                .ok_or_else(|| {
                    DomainError::Validation(
                        "Anthropic Claude API key not configured. Please enter your API key in Settings > Providers to chat with Claude models.".into(),
                    )
                })?;
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("anthropic"))
                    .await
                    .ok()
                    .flatten();
                let base_url =
                    custom_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                self.execute_anthropic(&base_url, &token, requested_model, req)
                    .await
            }
            "gemini" => {
                let token = (if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback("gemini").await?
                })
                .ok_or_else(|| {
                    DomainError::Validation(
                        "Google Gemini API key not configured. Please enter your API key in Settings > Providers to chat with Gemini models.".into(),
                    )
                })?;
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("gemini"))
                    .await
                    .ok()
                    .flatten();
                let base_url = custom_url.unwrap_or_else(|| {
                    "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
                });
                let model = if requested_model.is_empty()
                    || requested_model == "default"
                    || requested_model == "gemini"
                {
                    "gemini-2.5-flash"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(
                    &base_url,
                    Some(&token),
                    model,
                    req,
                    "Google Gemini",
                )
                .await
            }
            "deepseek" => {
                let token = (if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback("deepseek").await?
                })
                .ok_or_else(|| {
                    DomainError::Validation(
                        "DeepSeek API key not configured. Please enter your API key in Settings > Providers to chat with DeepSeek models.".into(),
                    )
                })?;
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("deepseek"))
                    .await
                    .ok()
                    .flatten();
                let base_url =
                    custom_url.unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
                let model = if requested_model.is_empty()
                    || requested_model == "default"
                    || requested_model == "deepseek"
                {
                    "deepseek-chat"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(&base_url, Some(&token), model, req, "DeepSeek")
                    .await
            }
            "local" => {
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("local"))
                    .await?;
                let base_url = custom_url.unwrap_or_else(|| "http://localhost:11434".to_string());
                if let Some(model) = requested_model.strip_prefix("ollama/") {
                    return self.execute_ollama(&base_url, model, req).await;
                }
                if base_url.contains(":11434") {
                    return self.execute_ollama(&base_url, requested_model, req).await;
                }
                let token = if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback("local").await?
                };
                let clean_model = requested_model
                    .strip_prefix("local/")
                    .or_else(|| requested_model.strip_prefix("local:"))
                    .unwrap_or(requested_model);
                self.execute_openai_compatible(
                    &base_url,
                    token.as_deref(),
                    clean_model,
                    req,
                    "Local Inference",
                )
                .await
            }
            "lmstudio" => {
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("lmstudio"))
                    .await
                    .ok()
                    .flatten();
                let base_url = custom_url.unwrap_or_else(|| "http://127.0.0.1:1234/v1".to_string());
                let token = if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback("lmstudio").await?
                };
                let clean_model = requested_model
                    .strip_prefix("lmstudio/")
                    .unwrap_or(requested_model);
                self.execute_openai_compatible(
                    &base_url,
                    token.as_deref(),
                    clean_model,
                    req,
                    "LM Studio",
                )
                .await
            }
            _ if find_by_service(target_provider).is_some() => {
                let endpoint = find_by_service(target_provider).unwrap();
                let token = (if let Some(account) = req.provider_account_id.as_deref() {
                    self.resolver.get_token(account).await?
                } else {
                    self.resolve_token_with_fallback(target_provider).await?
                })
                .ok_or_else(|| {
                    DomainError::Validation(format!(
                        "{target_provider} API key not configured. Please enter your API key in Settings > Providers."
                    ))
                })?;
                let custom_url = self
                    .resolver
                    .get_endpoint_url(req.provider_account_id.as_deref().unwrap_or(target_provider))
                    .await
                    .ok()
                    .flatten();
                let base_url = custom_url.unwrap_or_else(|| endpoint.default_base_url.to_string());
                let clean_model = strip_model_prefix(requested_model, endpoint);
                self.execute_openai_compatible(&base_url, Some(&token), clean_model, req, target_provider)
                    .await
            }
            _ if self.resolver.get_endpoint_url(req.provider_account_id.as_deref().unwrap_or(target_provider)).await.ok().flatten().is_some() => {
                let base_url = self.resolver.get_endpoint_url(req.provider_account_id.as_deref().unwrap_or(target_provider)).await.unwrap().unwrap();
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or(target_provider)).await.ok().flatten();
                let clean_model = if let Some((_, rem)) = requested_model.split_once('/') { rem } else { requested_model };
                self.execute_openai_compatible(&base_url, token.as_deref(), clean_model, req, target_provider).await
            }
            _ => {
                Err(DomainError::Validation(format!(
                    "Unsupported model or provider for '{}'. Please select a supported model in the Model Selector.",
                    requested_model
                )))
            }
        }
    }
}
