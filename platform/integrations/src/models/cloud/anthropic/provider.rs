use async_trait::async_trait;
use reqwest::StatusCode;
use serde_json::Value;

use super::types::{
    AnthropicProvider, ANTHROPIC_DEFAULT_MODEL, ANTHROPIC_DOC_URL,
};
use crate::base::{
    known_models_from_registry, ConfigKey, MessageStream, Provider, ProviderDescriptor,
    ProviderMetadata,
};
use crate::cloud::transport::openai_compatible::handle_status;
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::cloud::codecs::anthropic::ANTHROPIC_PROVIDER_NAME;
use crate::model::ModelConfig;
use rmcp::model::Tool;

impl AnthropicProvider {
    pub(crate) async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.api_client.request("v1/models").response_get().await?;

        if response.status() == StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|p| {
                    p.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "models endpoint not found".to_string());
            return Err(ProviderError::EndpointNotFound(msg));
        }

        let response = handle_status(response).await?;

        let body = response.bytes().await.map_err(|e| {
            ProviderError::NetworkError(format!("Failed to read response body: {}", e))
        })?;
        let json: Value = serde_json::from_slice(&body).map_err(|e| {
            ProviderError::EndpointNotFound(format!("Response body is not valid JSON: {}", e))
        })?;

        if let Some(err_obj) = json.get("error").filter(|error| !error.is_null()) {
            let message = err_obj
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string();
            let error_type = err_obj.get("type").and_then(Value::as_str);
            return Err(match error_type {
                Some("authentication_error" | "permission_error") => {
                    ProviderError::Authentication(message)
                }
                Some("rate_limit_error") => ProviderError::RateLimitExceeded {
                    details: message,
                    retry_delay: None,
                },
                Some("billing_error") => ProviderError::CreditsExhausted {
                    details: message,
                    top_up_url: None,
                },
                Some("api_error" | "overloaded_error") => ProviderError::ServerError(message),
                _ => ProviderError::RequestFailed(message),
            });
        }

        let arr = match json.get("data").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                return Err(ProviderError::RequestFailed(
                    "response is not a models payload (missing 'data' array)".into(),
                ));
            }
        };

        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }
}

impl ProviderDescriptor for AnthropicProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            ANTHROPIC_PROVIDER_NAME,
            "Anthropic",
            "Claude and other models from Anthropic",
            ANTHROPIC_DEFAULT_MODEL,
            known_models_from_registry(ANTHROPIC_PROVIDER_NAME),
            ANTHROPIC_DOC_URL,
            vec![
                ConfigKey::new("ANTHROPIC_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "ANTHROPIC_HOST",
                    true,
                    false,
                    Some("https://api.anthropic.com"),
                    false,
                ),
            ],
        )
        .with_setup_steps(vec![
            "Go to https://platform.claude.com/settings/keys",
            "Click 'Create Key'",
            "Copy the key and paste it above",
        ])
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        self.api_client
            .refresh_credentials()
            .await
            .map_err(|error| ProviderError::Authentication(error.to_string()))
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        let configured_limits = self
            .custom_models
            .iter()
            .flatten()
            .filter_map(|model| model.context_limit.map(|limit| (model.name.clone(), limit)));
        crate::context_limit::ContextLimitResolver::new(&self.name)
            .with_configured_limits(configured_limits)
            .resolve(model, override_limit, || async { Ok(None) })
            .await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(custom_models) = &self.custom_models {
            if self.dynamic_models == Some(false) {
                return Ok(custom_models
                    .iter()
                    .map(|model| model.name.clone())
                    .collect());
            }
            match self.fetch_models_from_api().await {
                Ok(models) => return Ok(models),
                Err(e) if e.is_endpoint_not_found() => {
                    tracing::debug!(
                        "Models endpoint not implemented for provider '{}' ({}), using predefined list",
                        self.name,
                        e
                    );
                    return Ok(custom_models
                        .iter()
                        .map(|model| model.name.clone())
                        .collect());
                }
                Err(e) => return Err(e),
            }
        }

        self.fetch_models_from_api().await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.stream_for_model(
            model_config,
            &model_config.model_name,
            system,
            messages,
            tools,
        )
        .await
    }
}
