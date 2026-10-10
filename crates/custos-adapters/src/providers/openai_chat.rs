//! Live OpenAI Chat Completions Provider
//!
//! Implements ModelProvider / ModelPort for live chat execution using either
//! OAuth 2.0 PKCE tokens or API keys against OpenAI's /v1/chat/completions endpoint.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

pub type TokenResolver = Arc<dyn Fn() -> Result<String, DomainError> + Send + Sync>;

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o";

#[derive(Debug, Serialize)]
struct ChatCompletionRequestMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionResponseChoiceMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionResponseChoice {
    pub message: ChatCompletionResponseChoiceMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionResponseUsage {
    #[serde(default)]
    pub prompt_tokens: Option<usize>,
    #[serde(default)]
    pub completion_tokens: Option<usize>,
    #[serde(default)]
    pub total_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<ChatCompletionResponseChoice>,
    #[serde(default)]
    pub usage: Option<ChatCompletionResponseUsage>,
}

#[async_trait]
pub trait TokenProvider: Send + Sync {
    async fn get_access_token(&self) -> Result<String, DomainError>;
}

#[derive(Clone)]
pub struct OpenAiChatProvider {
    provider_id: String,
    base_url: String,
    default_model: String,
    http_client: reqwest::Client,
    static_token: Option<String>,
    token_resolver: Option<TokenResolver>,
    token_provider: Option<Arc<dyn TokenProvider>>,
}

impl OpenAiChatProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "openai".to_string(),
            base_url: DEFAULT_OPENAI_BASE_URL.to_string(),
            default_model: DEFAULT_OPENAI_MODEL.to_string(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            static_token: None,
            token_resolver: None,
            token_provider: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.static_token = Some(token.into());
        self
    }

    pub fn with_resolver(mut self, resolver: TokenResolver) -> Self {
        self.token_resolver = Some(resolver);
        self
    }

    pub fn with_token_provider(mut self, provider: Arc<dyn TokenProvider>) -> Self {
        self.token_provider = Some(provider);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Resolves the bearer token from configured resolver, static token, or OPENAI_API_KEY env.
    pub fn resolve_token(&self) -> Result<String, DomainError> {
        if let Some(ref resolver) = self.token_resolver {
            return resolver();
        }
        if let Some(ref tok) = self.static_token {
            if !tok.trim().is_empty() {
                return Ok(tok.trim().to_string());
            }
        }
        if let Ok(env_key) = std::env::var("OPENAI_API_KEY") {
            if !env_key.trim().is_empty() {
                return Ok(env_key.trim().to_string());
            }
        }
        Err(DomainError::Validation(
            "OpenAI credentials not configured. Please authenticate via OAuth 2.0 PKCE or provide an API key in Settings.".into(),
        ))
    }
}

impl Default for OpenAiChatProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for OpenAiChatProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        let token = if let Some(ref tp) = self.token_provider {
            tp.get_access_token().await?
        } else {
            self.resolve_token()?
        };

        let model = if req.model.is_empty()
            || req.model == "default"
            || req.model == "openai"
            || req.model == "openai-codex"
        {
            self.default_model.clone()
        } else {
            req.model.clone()
        };

        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let payload = ChatCompletionRequest {
            model: model.clone(),
            messages: vec![ChatCompletionRequestMessage {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }],
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        info!(
            model = %model,
            endpoint = %endpoint,
            prompt_len = req.prompt.len(),
            "Dispatching live OpenAI chat completion"
        );

        let resp = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("OpenAI transport error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, body = %body, "OpenAI API returned non-success error");
            return Err(DomainError::Validation(
                crate::providers::router::format_provider_error(status, &body, "OpenAI"),
            ));
        }

        let res_body: ChatCompletionResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!("Failed to parse OpenAI JSON response: {e}"))
        })?;

        let choice = res_body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::Validation("OpenAI returned no completion choices".into()))?;

        let content = choice.message.content.unwrap_or_default();
        let tokens_used = res_body.usage.and_then(|u| u.total_tokens).unwrap_or(0);
        let resolved_model = res_body.model.unwrap_or(model);

        debug!(
            model = %resolved_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from OpenAI"
        );

        Ok(ModelResponse::text(content, resolved_model, tokens_used))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_openai_provider_fails_token_resolution() {
        // Clear OPENAI_API_KEY during test
        let prev = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");

        let provider = OpenAiChatProvider::new();
        assert!(provider.resolve_token().is_err());

        if let Some(p) = prev {
            std::env::set_var("OPENAI_API_KEY", p);
        }
    }

    #[test]
    fn static_token_resolves_correctly() {
        let provider = OpenAiChatProvider::new().with_token("test-token-123");
        assert_eq!(provider.resolve_token().unwrap(), "test-token-123");
    }

    #[test]
    fn custom_resolver_takes_precedence() {
        let resolver = Arc::new(|| Ok("resolved-token-xyz".to_string()));
        let provider = OpenAiChatProvider::new()
            .with_token("static-token")
            .with_resolver(resolver);
        assert_eq!(provider.resolve_token().unwrap(), "resolved-token-xyz");
    }
}
