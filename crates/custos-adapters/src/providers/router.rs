//! Unified Router Model Provider
//!
//! Routes model inference requests to live provider endpoints (OpenAI, Anthropic,
//! Gemini, DeepSeek, Local/Ollama) based on requested model name and stored credentials.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelResponse, ProviderRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

#[async_trait]
pub trait CredentialResolver: Send + Sync {
    /// Return access token or API key for a given provider
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError>;
    /// Return custom endpoint URL for a given provider
    async fn get_endpoint_url(&self, provider_id: &str) -> Result<Option<String>, DomainError>;
}

/// Fallback resolver that reads environment variables
pub struct EnvCredentialResolver;

#[async_trait]
impl CredentialResolver for EnvCredentialResolver {
    async fn get_token(&self, provider_id: &str) -> Result<Option<String>, DomainError> {
        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            _ => return Ok(None),
        };
        Ok(std::env::var(env_var).ok().filter(|s| !s.trim().is_empty()))
    }

    async fn get_endpoint_url(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequestMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiChatRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatChoice {
    message: OpenAiChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatUsage {
    #[serde(default)]
    total_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<OpenAiChatChoice>,
    #[serde(default)]
    usage: Option<OpenAiChatUsage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessageItem {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicMessagesRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessageItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: Option<usize>,
    #[serde(default)]
    output_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessagesResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Clone)]
pub struct RouterModelProvider {
    resolver: Arc<dyn CredentialResolver>,
    http_client: reqwest::Client,
}

impl RouterModelProvider {
    pub fn new(resolver: Arc<dyn CredentialResolver>) -> Self {
        Self {
            resolver,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_env_fallback() -> Self {
        Self::new(Arc::new(EnvCredentialResolver))
    }

    /// Determines the provider service type from a given model identifier
    pub fn classify_provider(model: &str) -> &'static str {
        let m = model.trim().to_lowercase();
        if m.starts_with("gpt-")
            || m.starts_with("o1")
            || m.starts_with("o3")
            || m.starts_with("o4")
            || m.starts_with("chatgpt")
            || m.starts_with("text-embedding")
            || m == "openai"
            || m == "openai-codex"
            || m == "codex"
        {
            "openai"
        } else if m.starts_with("claude-") || m == "anthropic" {
            "anthropic"
        } else if m.starts_with("gemini-") || m == "google" || m == "gemini" {
            "gemini"
        } else if m.starts_with("deepseek-") || m == "deepseek" {
            "deepseek"
        } else if m.starts_with("llama")
            || m.starts_with("qwen")
            || m.starts_with("mistral")
            || m == "local"
            || m == "ollama"
        {
            "local"
        } else if m.contains("claude") {
            "anthropic"
        } else if m.contains("gemini") {
            "gemini"
        } else if m.contains("deepseek") {
            "deepseek"
        } else if m.contains("gpt") {
            "openai"
        } else {
            "openai"
        }
    }

    async fn resolve_token_with_fallback(&self, provider_id: &str) -> Option<String> {
        if let Ok(Some(tok)) = self.resolver.get_token(provider_id).await {
            if !tok.trim().is_empty() {
                return Some(tok.trim().to_string());
            }
        }
        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            _ => return None,
        };
        std::env::var(env_var).ok().filter(|s| !s.trim().is_empty())
    }

    async fn execute_openai_compatible(
        &self,
        base_url: &str,
        token: Option<&str>,
        model: &str,
        req: &ProviderRequest,
        provider_label: &str,
    ) -> Result<ModelResponse, DomainError> {
        let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let payload = OpenAiChatRequest {
            model: model.to_string(),
            messages: vec![OpenAiChatRequestMessage {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }],
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        info!(
            provider = %provider_label,
            model = %model,
            endpoint = %endpoint,
            prompt_len = req.prompt.len(),
            "Dispatching live chat completion"
        );

        let mut req_builder = self.http_client.post(&endpoint).json(&payload);
        if let Some(t) = token {
            req_builder = req_builder.bearer_auth(t);
        }

        let resp = req_builder.send().await.map_err(|e| {
            DomainError::Validation(format!("{provider_label} transport error: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, body = %body, "{provider_label} API returned error response");
            return Err(DomainError::Validation(format!(
                "{provider_label} API error {status}: {body}"
            )));
        }

        let res_body: OpenAiChatResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!("Failed to parse {provider_label} JSON response: {e}"))
        })?;

        let choice = res_body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::Validation(format!("{provider_label} returned no completion choices")))?;

        let content = choice.message.content.unwrap_or_default();
        let tokens_used = res_body.usage.and_then(|u| u.total_tokens).unwrap_or(0);
        let resolved_model = res_body.model.unwrap_or_else(|| model.to_string());

        debug!(
            provider = %provider_label,
            model = %resolved_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from {provider_label}"
        );

        Ok(ModelResponse::text(content, resolved_model, tokens_used))
    }

    async fn execute_anthropic(
        &self,
        base_url: &str,
        token: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        let endpoint = format!("{}/messages", base_url.trim_end_matches('/'));
        let target_model = if model.is_empty() || model == "default" || model == "anthropic" {
            "claude-3-7-sonnet-20250219"
        } else {
            model
        };

        let payload = AnthropicMessagesRequest {
            model: target_model.to_string(),
            max_tokens: req.max_tokens.unwrap_or(4096),
            messages: vec![AnthropicMessageItem {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }],
            temperature: req.temperature,
        };

        info!(
            model = %target_model,
            endpoint = %endpoint,
            prompt_len = req.prompt.len(),
            "Dispatching live Anthropic Claude completion"
        );

        let resp = self
            .http_client
            .post(&endpoint)
            .header("x-api-key", token)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("Anthropic transport error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, body = %body, "Anthropic API returned error response");
            return Err(DomainError::Validation(format!(
                "Anthropic API error {status}: {body}"
            )));
        }

        let res_body: AnthropicMessagesResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!("Failed to parse Anthropic JSON response: {e}"))
        })?;

        let content = res_body
            .content
            .into_iter()
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        let tokens_used = res_body
            .usage
            .map(|u| u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0))
            .unwrap_or(0);
        let resolved_model = res_body.model.unwrap_or_else(|| target_model.to_string());

        Ok(ModelResponse::text(content, resolved_model, tokens_used))
    }
}

#[async_trait]
impl ModelProvider for RouterModelProvider {
    fn provider_id(&self) -> &str {
        "unified-router"
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let requested_model = req.model.trim();
        let target_provider = Self::classify_provider(requested_model);

        match target_provider {
            "openai" => {
                let token = self.resolve_token_with_fallback("openai").await.ok_or_else(|| {
                    DomainError::Validation(
                        "OpenAI credentials not configured. Please log in with OpenAI via OAuth 2.0 PKCE or enter an API key in Settings > Providers.".into(),
                    )
                })?;
                let custom_url = self.resolver.get_endpoint_url("openai").await.ok().flatten();
                let base_url = custom_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let model = if requested_model.is_empty() || requested_model == "default" || requested_model == "openai" {
                    "gpt-4o"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(&base_url, Some(&token), model, req, "OpenAI").await
            }
            "anthropic" => {
                let token = self.resolve_token_with_fallback("anthropic").await.ok_or_else(|| {
                    DomainError::Validation(
                        "Anthropic Claude API key not configured. Please enter your API key in Settings > Providers to chat with Claude models.".into(),
                    )
                })?;
                let custom_url = self.resolver.get_endpoint_url("anthropic").await.ok().flatten();
                let base_url = custom_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                self.execute_anthropic(&base_url, &token, requested_model, req).await
            }
            "gemini" => {
                let token = self.resolve_token_with_fallback("gemini").await.ok_or_else(|| {
                    DomainError::Validation(
                        "Google Gemini API key not configured. Please enter your API key in Settings > Providers to chat with Gemini models.".into(),
                    )
                })?;
                let custom_url = self.resolver.get_endpoint_url("gemini").await.ok().flatten();
                let base_url = custom_url.unwrap_or_else(|| {
                    "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
                });
                let model = if requested_model.is_empty() || requested_model == "default" || requested_model == "gemini" {
                    "gemini-2.5-flash"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(&base_url, Some(&token), model, req, "Google Gemini").await
            }
            "deepseek" => {
                let token = self.resolve_token_with_fallback("deepseek").await.ok_or_else(|| {
                    DomainError::Validation(
                        "DeepSeek API key not configured. Please enter your API key in Settings > Providers to chat with DeepSeek models.".into(),
                    )
                })?;
                let custom_url = self.resolver.get_endpoint_url("deepseek").await.ok().flatten();
                let base_url = custom_url.unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
                let model = if requested_model.is_empty() || requested_model == "default" || requested_model == "deepseek" {
                    "deepseek-chat"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(&base_url, Some(&token), model, req, "DeepSeek").await
            }
            "local" => {
                let custom_url = self.resolver.get_endpoint_url("local").await.ok().flatten();
                let base_url = custom_url.unwrap_or_else(|| "http://localhost:11434/v1".to_string());
                let token = self.resolve_token_with_fallback("local").await;
                let model = if requested_model.is_empty() || requested_model == "default" || requested_model == "local" {
                    "llama3.3:70b"
                } else {
                    requested_model
                };
                self.execute_openai_compatible(&base_url, token.as_deref(), model, req, "Local Inference").await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_provider() {
        assert_eq!(RouterModelProvider::classify_provider("gpt-4o"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("gpt-4o-mini"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("o3-mini"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("o1"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("claude-3-7-sonnet"), "anthropic");
        assert_eq!(RouterModelProvider::classify_provider("claude-3-5-haiku"), "anthropic");
        assert_eq!(RouterModelProvider::classify_provider("gemini-2.5-flash"), "gemini");
        assert_eq!(RouterModelProvider::classify_provider("gemini-2.5-pro"), "gemini");
        assert_eq!(RouterModelProvider::classify_provider("deepseek-chat"), "deepseek");
        assert_eq!(RouterModelProvider::classify_provider("deepseek-reasoner"), "deepseek");
        assert_eq!(RouterModelProvider::classify_provider("llama3.3:70b"), "local");
        assert_eq!(RouterModelProvider::classify_provider("qwen2.5-coder:32b"), "local");
    }

    struct MockEmptyResolver;

    #[async_trait]
    impl CredentialResolver for MockEmptyResolver {
        async fn get_token(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
            Ok(None)
        }
        async fn get_endpoint_url(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_unconfigured_model_fails_closed_without_panic() {
        // Clear environment variables
        let _ = std::env::remove_var("OPENAI_API_KEY");
        let _ = std::env::remove_var("ANTHROPIC_API_KEY");

        let router = RouterModelProvider::new(Arc::new(MockEmptyResolver));
        let req = ProviderRequest::simple("Hello world");
        let result = router.generate(&req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
