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
    async fn uses_oauth(&self, _provider_id: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
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
    codex_endpoint: String,
}

impl RouterModelProvider {
    pub fn new(resolver: Arc<dyn CredentialResolver>) -> Self {
        Self {
            resolver,
            codex_endpoint: "https://chatgpt.com/backend-api/codex/responses".into(),
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
            return Err(DomainError::Validation(format_provider_error(status, &body, provider_label)));
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

    async fn execute_chatgpt_codex(
        &self,
        token: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        let endpoint = self.codex_endpoint.as_str();

        // Map generic/openai model names to Codex flagship model
        let target_model = if model.is_empty()
            || model == "default"
            || model == "openai"
            || model == "codex"
            || model == "openai-codex"
        {
            "gpt-5.6-sol"
        } else {
            model
        };

        let payload = serde_json::json!({
            "model": target_model,
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": req.prompt}]
                }
            ],
            "instructions": "You are a helpful assistant.",
            "store": false,
            "stream": true
        });

        info!(
            model = %target_model,
            endpoint = %endpoint,
            prompt_len = req.prompt.len(),
            "Dispatching live ChatGPT Codex response via subscription quota"
        );

        let mut req_builder = self
            .http_client
            .post(endpoint)
            .bearer_auth(token)
            .header("User-Agent", "codex-cli/0.144.0")
            .header("Originator", "codex_cli_rs")
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .json(&payload);

        if let Some(account_id) = crate::providers::oauth_pkce::extract_chatgpt_account_id(token) {
            req_builder = req_builder.header("ChatGPT-Account-ID", account_id);
        }

        let resp = req_builder.send().await.map_err(|e| {
            DomainError::Validation(format!("ChatGPT Codex transport error: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, "ChatGPT Codex returned error response");
            return Err(DomainError::Validation(format_provider_error(status, &body, "ChatGPT Codex")));
        }

        let text_body = resp.text().await.map_err(|e| {
            DomainError::Validation(format!("Failed reading Codex response stream: {e}"))
        })?;

        let (content, tokens_used) = parse_codex_sse_stream(&text_body)?;

        debug!(
            model = %target_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from ChatGPT Codex"
        );

        Ok(ModelResponse::text(content, target_model.to_string(), tokens_used))
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
        debug!(task_id = %req.task_id, request_id = %req.request_id, model = %requested_model, "Routing model request");
        let target_provider = Self::classify_provider(requested_model);

        match target_provider {
            "openai" => {
                let oauth = self.resolver.uses_oauth("openai").await?;
                let token = self.resolver.get_token("openai").await?.ok_or_else(|| {
                    DomainError::Validation(
                        "OpenAI credentials not configured. Please log in with OpenAI via OAuth 2.0 PKCE or enter an API key in Settings > Providers.".into(),
                    )
                })?;
                let custom_url = self.resolver.get_endpoint_url("openai").await.ok().flatten();
                if oauth {
                    return self.execute_chatgpt_codex(&token, requested_model, req).await;
                }
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

/// Formats non-success provider API errors, decorating known credit/quota balance issues
/// with actionable instructions.
pub fn format_provider_error(status: reqwest::StatusCode, body: &str, provider_label: &str) -> String {
    if provider_label == "OpenAI" && (body.contains("credit_balance_exhausted") || body.contains("insufficient_quota")) {
        "OpenAI API error 429 (credit_balance_exhausted): You have no prepaid API credits remaining on your OpenAI Platform organization.\n\n\
        Note: ChatGPT Plus subscription ($20/mo) only covers web/app ChatGPT and does NOT cover OpenAI Platform Developer API usage. \
        Please add credits ($5+) at https://platform.openai.com/settings/organization/billing/ or switch to Google Gemini (free tier) / Ollama.".to_string()
    } else {
        format!("{provider_label} API error {status}: {body}")
    }
}

/// Parses SSE event stream returned by ChatGPT Codex responses endpoint
/// (`https://chatgpt.com/backend-api/codex/responses`).
pub fn parse_codex_sse_stream(raw: &str) -> Result<(String, usize), DomainError> {
    let mut content = String::new();
    let mut tokens_used: Option<usize> = None;
    let mut completed = false;

    for line in raw.lines() {
        let line = line.trim();
        if let Some(json_str) = line.strip_prefix("data: ") {
            if json_str == "[DONE]" {
                break;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if matches!(val.get("type").and_then(|t| t.as_str()), Some("error" | "response.failed" | "response.incomplete")) {
                    let message = val.pointer("/error/message").or_else(|| val.pointer("/response/error/message")).or_else(|| val.get("message")).and_then(|m| m.as_str()).unwrap_or("Response failed or incomplete");
                    return Err(DomainError::Validation(format!("ChatGPT Codex stream error: {message}")));
                }
                completed |= val.get("type").and_then(|t| t.as_str()) == Some("response.completed");
                if let Some(delta) = val.get("delta").and_then(|d| d.as_str()) {
                    content.push_str(delta);
                }
                if let Some(usage) = val.get("response").and_then(|r| r.get("usage")) {
                    if let Some(total) = usage.get("total_tokens").and_then(|t| t.as_u64()) {
                        tokens_used = Some(total as usize);
                    }
                } else if let Some(total) = val.get("usage").and_then(|u| u.get("total_tokens")).and_then(|t| t.as_u64()) {
                    tokens_used = Some(total as usize);
                }
            }
        }
    }

    if !completed {
        return Err(DomainError::Validation("ChatGPT Codex stream ended before completion".into()));
    }
    if content.is_empty() {
        return Err(DomainError::Validation(
            "ChatGPT Codex returned an empty stream response".into(),
        ));
    }

    let resolved_tokens = tokens_used.unwrap_or_else(|| (content.len() / 4).max(1));
    Ok((content, resolved_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCredentialResolver {
        oauth: bool,
        endpoint: String,
        fail: bool,
    }
    #[async_trait]
    impl CredentialResolver for TestCredentialResolver {
        async fn uses_oauth(&self, _: &str) -> Result<bool, DomainError> {
            Ok(self.oauth)
        }
        async fn get_token(&self, _: &str) -> Result<Option<String>, DomainError> {
            if self.fail {
                return Err(DomainError::Validation("OAuth refresh failed".into()));
            }
            Ok(Some(
                if self.oauth {
                    "opaque-credential"
                } else {
                    "eyJ-api-key"
                }
                .into(),
            ))
        }
        async fn get_endpoint_url(&self, _: &str) -> Result<Option<String>, DomainError> {
            Ok(Some(self.endpoint.clone()))
        }
    }

    #[tokio::test]
    async fn oauth_with_configured_endpoint_uses_codex_and_preserves_model() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let n = socket.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&request);
                if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                    let len: usize = headers
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.parse().ok())
                        })
                        .unwrap_or(0);
                    if body.len() >= len {
                        break;
                    }
                }
            }
            let body = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"OAuth answer\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":5}}}\n\n";
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth: true,
            endpoint: format!("{base}/v1"),
            fail: false,
        }));
        provider.codex_endpoint = format!("{base}/codex/responses");
        let mut req = ProviderRequest::simple("hello OAuth");
        req.model = "gpt-4o".into();
        let result = provider.generate(&req).await.unwrap();
        assert_eq!(result.content, "OAuth answer");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /codex/responses "));
        let payload: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(payload["model"], "gpt-4o");
        assert_eq!(payload["input"][0]["content"][0]["text"], "hello OAuth");
    }

    #[tokio::test]
    async fn api_key_uses_custom_api_endpoint_even_for_jwt_shaped_keys() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut buf = [0; 4096];
            let n = socket.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = r#"{"model":"gpt-4o","choices":[{"message":{"content":"API answer"}}],"usage":{"total_tokens":5}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            request
        });
        let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth: false,
            endpoint: format!("{base}/custom/v1"),
            fail: false,
        }));
        let mut req = ProviderRequest::simple("hello API");
        req.model = "gpt-4o".into();
        assert_eq!(provider.generate(&req).await.unwrap().content, "API answer");
        assert!(server
            .join()
            .unwrap()
            .starts_with("POST /custom/v1/chat/completions "));
    }

    #[tokio::test]
    async fn oauth_refresh_failure_never_falls_back() {
        let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth: true,
            endpoint: "http://127.0.0.1:1/v1".into(),
            fail: true,
        }));
        let mut req = ProviderRequest::simple("hello");
        req.model = "gpt-4o".into();
        assert!(provider
            .generate(&req)
            .await
            .unwrap_err()
            .to_string()
            .contains("OAuth refresh failed"));
    }

    #[test]
    fn codex_errors_keep_subscription_identity_and_reject_partial_streams() {
        let formatted = format_provider_error(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "insufficient_quota",
            "ChatGPT Codex",
        );
        assert!(!formatted.contains("platform.openai.com"));
        assert!(formatted.contains("ChatGPT Codex"));
        for terminal in ["error", "response.failed", "response.incomplete"] {
            let stream = format!("data: {{\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}}\n\ndata: {{\"type\":\"{terminal}\"}}\n\n");
            assert!(parse_codex_sse_stream(&stream).is_err());
        }
        assert!(parse_codex_sse_stream("data: {\"delta\":\"partial\"}\n").is_err());
    }
    #[test]
    fn test_format_provider_error_credit_balance_exhausted() {
        let err_body = r#"{ "error": { "message": "You have no credits remaining.", "type": "insufficient_quota", "code": "credit_balance_exhausted" } }"#;
        let formatted = format_provider_error(reqwest::StatusCode::TOO_MANY_REQUESTS, err_body, "OpenAI");
        assert!(formatted.contains("credit_balance_exhausted"));
        assert!(formatted.contains("ChatGPT Plus subscription ($20/mo) only covers web/app ChatGPT"));
        assert!(formatted.contains("https://platform.openai.com/settings/organization/billing/"));
    }

    #[test]
    fn test_format_provider_error_standard() {
        let err_body = r#"{"error": "invalid model"}"#;
        let formatted = format_provider_error(reqwest::StatusCode::BAD_REQUEST, err_body, "OpenAI");
        assert_eq!(formatted, "OpenAI API error 400 Bad Request: {\"error\": \"invalid model\"}");
    }

    #[test]
    fn test_classify_provider() {
        assert_eq!(RouterModelProvider::classify_provider("gpt-4o"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("gpt-4o-mini"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("gpt-5.6-sol"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("gpt-5.6-terra"), "openai");
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

    #[test]
    fn test_parse_codex_sse_stream() {
        let stream = "event: response.output_text.delta\n\
                      data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\n\
                      event: response.output_text.delta\n\
                      data: {\"type\":\"response.output_text.delta\",\"delta\":\" world!\"}\n\n\
                      event: response.completed\n\
                      data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":18}}}\n\n";
        let (content, tokens) = parse_codex_sse_stream(stream).expect("valid parse");
        assert_eq!(content, "Hello world!");
        assert_eq!(tokens, 18);
    }

    #[test]
    fn test_parse_codex_sse_stream_empty() {
        let stream = "event: ping\ndata: {}\n\n";
        let err = parse_codex_sse_stream(stream);
        assert!(err.is_err());
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
