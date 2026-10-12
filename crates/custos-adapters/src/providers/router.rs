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
    async fn validate_account(&self, _account_id: &str, _service: &str) -> Result<(), DomainError> { Ok(()) }
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
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
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
        if m.starts_with("mock")
            || m.starts_with("offline")
            || m == "custos-mock"
            || m == "custos-offline"
            || m == "offline-sovereign"
        {
            return "mock";
        }
        if m.starts_with("ollama/") {
            return "local";
        }
        if m.starts_with("copilot/") {
            return "copilot";
        }
        if m.starts_with("gemini-oauth/") {
            return "gemini-oauth";
        }
        if m.starts_with("antigravity/") {
            return "antigravity";
        }
        if m.starts_with("groq/") || m == "groq" {
            return "groq";
        }
        if m.starts_with("openrouter/") || m == "openrouter" {
            return "openrouter";
        }
        if m.starts_with("together/") || m == "together" {
            return "together";
        }
        if m.starts_with("cerebras/") || m == "cerebras" {
            return "cerebras";
        }
        if m.starts_with("perplexity/") || m == "perplexity" {
            return "perplexity";
        }
        if m.starts_with("mistral/") {
            return "mistral";
        }
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

    async fn resolve_token_with_fallback(
        &self,
        provider_id: &str,
    ) -> Result<Option<String>, DomainError> {
        if let Some(tok) = self.resolver.get_token(provider_id).await? {
            if !tok.trim().is_empty() {
                return Ok(Some(tok.trim().to_string()));
            }
        }
        let env_var = match provider_id {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "groq" => "GROQ_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "together" => "TOGETHER_API_KEY",
            "mistral" => "MISTRAL_API_KEY",
            "cerebras" => "CEREBRAS_API_KEY",
            "perplexity" => "PERPLEXITY_API_KEY",
            _ => return Ok(None),
        };
        Ok(std::env::var(env_var).ok().filter(|s| !s.trim().is_empty()))
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

        if provider_label == "GitHub Copilot" {
            req_builder = req_builder
                .header("User-Agent", "GitHubCopilotChat/0.35.0")
                .header("Editor-Version", "vscode/1.107.0")
                .header("Editor-Plugin-Version", "copilot-chat/0.35.0")
                .header("Copilot-Integration-Id", "vscode-chat");
        }
        let resp = req_builder.send().await.map_err(|e| {
            DomainError::Validation(format!("{provider_label} transport error: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, body = %body, "{provider_label} API returned error response");
            return Err(DomainError::Validation(format_provider_error(
                status,
                &body,
                provider_label,
            )));
        }

        let res_body: OpenAiChatResponse = resp.json().await.map_err(|e| {
            DomainError::Validation(format!(
                "Failed to parse {provider_label} JSON response: {e}"
            ))
        })?;

        let choice = res_body.choices.into_iter().next().ok_or_else(|| {
            DomainError::Validation(format!("{provider_label} returned no completion choices"))
        })?;

        let content = choice.message.content.unwrap_or_default();
        let usage = res_body.usage;
        let input = usage.as_ref().and_then(|u| u.prompt_tokens);
        let output = usage.as_ref().and_then(|u| u.completion_tokens);
        let total = usage.as_ref().and_then(|u| u.total_tokens);
        let tokens_used = total.unwrap_or(0);
        let resolved_model = res_body.model.unwrap_or_else(|| model.to_string());

        debug!(
            provider = %provider_label,
            model = %resolved_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from {provider_label}"
        );

        Ok(ModelResponse::text(content, resolved_model, tokens_used).with_usage(input, output, total))
    }

    async fn execute_ollama(
        &self,
        base: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        let root = super::probe::ollama_root(base)?;
        if model.is_empty() || matches!(model, "default" | "local" | "ollama") {
            return Err(DomainError::Validation(
                "Select an installed Ollama model in Settings > Providers".into(),
            ));
        }
        let mut options = serde_json::Map::new();
        if let Some(value) = req.temperature {
            options.insert("temperature".into(), serde_json::json!(value));
        }
        if let Some(value) = req.max_tokens {
            options.insert("num_predict".into(), serde_json::json!(value));
        }
        info!(task_id = %req.task_id, request_id = %req.request_id, model, "Dispatching Ollama chat");
        let response = self.http_client.post(format!("{root}/api/chat")).json(&serde_json::json!({
            "model": model, "messages": [{"role": "user", "content": req.prompt}], "stream": false, "options": options
        })).send().await.map_err(|e| DomainError::Validation(format!("Ollama connection failed; ensure Ollama is running: {e}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| DomainError::Validation(format!("Ollama response read failed: {e}")))?;
        if !status.is_success() {
            return Err(DomainError::Validation(format!("Ollama chat returned {status}: {body}. Refresh models if the selected model was removed.")));
        }
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| DomainError::Validation(format!("Invalid Ollama chat response: {e}")))?;
        let content = value
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::Validation("Ollama response has no message content".into())
            })?;
        if value.get("done").and_then(|v| v.as_bool()) != Some(true) {
            return Err(DomainError::Validation(
                "Ollama response did not complete".into(),
            ));
        }
        let tokens = value
            .get("prompt_eval_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .saturating_add(
                value
                    .get("eval_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            );
        Ok(ModelResponse::text(
            content,
            format!("ollama/{model}"),
            usize::try_from(tokens).unwrap_or(usize::MAX),
        ).with_usage(value.get("prompt_eval_count").and_then(|v| v.as_u64()).and_then(|v| usize::try_from(v).ok()), value.get("eval_count").and_then(|v| v.as_u64()).and_then(|v| usize::try_from(v).ok()), None))
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

        let resp = req_builder
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("ChatGPT Codex transport error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            error!(status = %status, "ChatGPT Codex returned error response");
            return Err(DomainError::Validation(format_provider_error(
                status,
                &body,
                "ChatGPT Codex",
            )));
        }

        let text_body = resp.text().await.map_err(|e| {
            DomainError::Validation(format!("Failed reading Codex response stream: {e}"))
        })?;

        let (content, tokens_used) = parse_codex_sse_stream(&text_body)?;
        let reported = text_body.lines().filter_map(|line| line.strip_prefix("data:")).filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok()).find(|v| v.get("type").and_then(|v| v.as_str()) == Some("response.completed"));
        let usage = reported.as_ref().and_then(|v| v.pointer("/response/usage"));
        let count = |key: &str| usage.and_then(|v| v.get(key)).and_then(|v| v.as_u64()).and_then(|v| usize::try_from(v).ok());

        debug!(
            model = %target_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from ChatGPT Codex"
        );

        Ok(ModelResponse::text(
            content,
            target_model.to_string(),
            tokens_used,
        )
        .with_usage(
            count("input_tokens"),
            count("output_tokens"),
            count("total_tokens"),
        ))
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

        let oauth = self
            .resolver
            .uses_oauth(req.provider_account_id.as_deref().unwrap_or("anthropic"))
            .await?;
        let mut builder = self
            .http_client
            .post(&endpoint)
            .header("anthropic-version", "2023-06-01")
            .json(&payload);
        builder = if oauth {
            builder
                .bearer_auth(token)
                .header("anthropic-beta", "oauth-2025-04-20")
                .header("User-Agent", "claude-cli/2.1.280 (external, cli)")
                .header("X-App", "cli")
        } else {
            builder.header("x-api-key", token)
        };
        let resp = builder
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("Anthropic transport error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
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

        let input = res_body.usage.as_ref().and_then(|u| u.input_tokens);
        let output = res_body.usage.as_ref().and_then(|u| u.output_tokens);
        let tokens_used = input.unwrap_or(0).saturating_add(output.unwrap_or(0));
        let resolved_model = res_body.model.unwrap_or_else(|| target_model.to_string());

        Ok(ModelResponse::text(content, resolved_model, tokens_used).with_usage(input, output, None))
    }
}

#[async_trait]
impl ModelProvider for RouterModelProvider {
    fn provider_id(&self) -> &str {
        "unified-router"
    }

    async fn stream(&self, req: &ProviderRequest) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        use super::streaming::{Wire, stream_response};
        let model = req.model.trim();
        let service = Self::classify_provider(model);
        let account = req.provider_account_id.as_deref().unwrap_or(if service == "gemini-oauth" { "gemini" } else { service });
        if req.provider_account_id.is_some() { self.resolver.validate_account(account, if service == "gemini-oauth" { "gemini" } else { service }).await?; }
        if matches!(service, "antigravity" | "gemini-oauth") {
            let token = self.resolver.get_token(account).await?.ok_or_else(||DomainError::Validation("Connect Google OAuth first".into()))?;
            let base = self.resolver.get_endpoint_url(account).await?.unwrap_or_else(||"https://cloudcode-pa.googleapis.com".into());
            let response = super::antigravity::stream_codeassist(if service == "gemini-oauth" {"gemini"} else {"antigravity"}, &base, &token, req).await?;
            return Ok(stream_response(response,req.request_id.clone(),Wire::Google));
        }
        if service == "mock" {
            let response=self.generate(req).await?;
            let (tx,rx)=tokio::sync::mpsc::channel(1);
            let _=tx.send(custos_provider::ProviderEvent::completed(&req.request_id,Some(response.content),None)).await;
            return Ok(rx);
        }
        let local_base = if service == "local" { self.resolver.get_endpoint_url(account).await?.unwrap_or_else(||"http://localhost:11434".into()) } else {String::new()};
        if service == "local" && (model.starts_with("ollama/") || local_base.contains(":11434")) {
            let root=super::probe::ollama_root(&local_base)?;
            let model=model.strip_prefix("ollama/").unwrap_or(model);
            if model.is_empty() || matches!(model,"default"|"local"|"ollama") { return Err(DomainError::Validation("Select an installed Ollama model".into())); }
            let mut options=serde_json::Map::new();
            if let Some(v)=req.temperature { options.insert("temperature".into(),serde_json::json!(v)); }
            if let Some(v)=req.max_tokens { options.insert("num_predict".into(),serde_json::json!(v)); }
            let response=self.http_client.post(format!("{root}/api/chat")).json(&serde_json::json!({"model":model,"messages":[{"role":"user","content":req.prompt}],"stream":true,"options":options})).send().await.map_err(|error|DomainError::Validation(format!("Ollama connection failed: {error}")))?;
            if !response.status().is_success() { return Err(DomainError::Validation(format!("Ollama chat returned {}",response.status()))); }
            return Ok(stream_response(response,req.request_id.clone(),Wire::Ollama));
        }
        let oauth=service=="openai" && self.resolver.uses_oauth(account).await?;
        let token=if req.provider_account_id.is_some() || oauth || service=="copilot" { self.resolver.get_token(account).await? } else { self.resolve_token_with_fallback(account).await? };
        if token.is_none() && service!="local" {
            return Err(DomainError::Validation("Provider credentials not configured".into()));
        }
        let custom=self.resolver.get_endpoint_url(account).await?;
        let mut request;
        let wire;
        if service=="openai" && oauth {
            wire=Wire::Codex;
            let model=if matches!(model,""|"default"|"openai"|"codex"|"openai-codex") {"gpt-5.6-sol"} else {model};
            request=self.http_client.post(&self.codex_endpoint).bearer_auth(token.as_deref().unwrap_or("")).header("User-Agent","codex-cli/0.144.0").header("Originator","codex_cli_rs").header("Accept","text/event-stream").json(&serde_json::json!({"model":model,"input":[{"role":"user","content":[{"type":"input_text","text":req.prompt}]}],"instructions":"You are a helpful assistant.","store":false,"stream":true}));
            if let Some(id)=super::oauth_pkce::extract_chatgpt_account_id(token.as_deref().unwrap_or("")) { request=request.header("ChatGPT-Account-ID",id); }
        } else if service=="anthropic" {
            wire=Wire::Claude;
            let base=custom.unwrap_or_else(||"https://api.anthropic.com/v1".into());
            let target=if matches!(model,""|"default"|"anthropic") {"claude-3-7-sonnet-20250219"} else {model};
            let mut payload=serde_json::json!({"model":target,"messages":[{"role":"user","content":req.prompt}],"max_tokens":req.max_tokens.unwrap_or(4096),"stream":true});
            if let Some(temperature)=req.temperature {payload["temperature"]=serde_json::json!(temperature);}
            request=self.http_client.post(format!("{}/messages",base.trim_end_matches('/'))).header("anthropic-version","2023-06-01").json(&payload);
            if self.resolver.uses_oauth(account).await? { request=request.bearer_auth(token.as_deref().unwrap_or("")).header("anthropic-beta","oauth-2025-04-20").header("User-Agent","claude-cli/2.1.280 (external, cli)").header("X-App","cli"); } else { request=request.header("x-api-key",token.as_deref().unwrap_or("")); }
        } else {
            wire=Wire::OpenAi;
            let (base, model, bearer)=match service {
                "copilot" => ("https://api.githubcopilot.com".into(),model.strip_prefix("copilot/").unwrap_or(model).to_string(),Some(super::oauth_pkce::copilot_session_token(token.as_deref().unwrap_or("")).await?)),
                "openai" => (custom.unwrap_or_else(||"https://api.openai.com/v1".into()),if matches!(model,""|"default"|"openai") {"gpt-4o".into()} else {model.into()},token),
                "gemini" => (custom.unwrap_or_else(||"https://generativelanguage.googleapis.com/v1beta/openai".into()),model.into(),token),
                "deepseek" => (custom.unwrap_or_else(||"https://api.deepseek.com/v1".into()),model.into(),token),
                "local" => (local_base,model.into(),token),
                _ => return Err(DomainError::Validation("Unsupported streaming provider".into())),
            };
            let mut payload=serde_json::json!({"model":model,"messages":[{"role":"user","content":req.prompt}],"stream":true,"stream_options":{"include_usage":true}});
            if let Some(v)=req.temperature {payload["temperature"]=serde_json::json!(v);}
            if let Some(v)=req.max_tokens {payload["max_tokens"]=serde_json::json!(v);}
            request=self.http_client.post(format!("{}/chat/completions",base.trim_end_matches('/'))).json(&payload);
            if let Some(token)=bearer { request=request.bearer_auth(token); }
        }
        if service=="copilot" {
            request=request.header("User-Agent","GitHubCopilotChat/0.35.0").header("Editor-Version","vscode/1.107.0").header("Editor-Plugin-Version","copilot-chat/0.35.0").header("Copilot-Integration-Id","vscode-chat");
        }
        tracing::debug!(task_id = %req.task_id, request_id = %req.request_id, service, "Starting provider stream");
        let response=request.send().await.map_err(|e|DomainError::Validation(format!("Provider stream connection failed: {e}")))?;
        if !response.status().is_success() { return Err(DomainError::Validation(format!("Provider stream returned {}",response.status()))); }
        Ok(stream_response(response,req.request_id.clone(),wire))
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let requested_model = req.model.trim();
        debug!(task_id = %req.task_id, request_id = %req.request_id, model = %requested_model, "Routing model request");
        let target_provider = Self::classify_provider(requested_model);
        if let Some(account) = &req.provider_account_id {
            let service = if target_provider == "gemini-oauth" { "gemini" } else { target_provider };
            self.resolver.validate_account(account, service).await?;
        }

        match target_provider {
            "copilot" => {
                let github = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("copilot")).await?.ok_or_else(|| DomainError::Validation("Connect GitHub Copilot first".into()))?;
                let token = super::oauth_pkce::copilot_session_token(&github).await?;
                let model = requested_model.strip_prefix("copilot/").ok_or_else(|| DomainError::Validation("Invalid Copilot model".into()))?;
                self.execute_openai_compatible("https://api.githubcopilot.com", Some(&token), model, req, "GitHub Copilot").await
            }
            "gemini-oauth" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("gemini")).await?.ok_or_else(|| DomainError::Validation("Connect Gemini OAuth first".into()))?;
                super::antigravity::generate_codeassist_content("gemini", "https://cloudcode-pa.googleapis.com", &token, req).await
            }
            "antigravity" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("antigravity")).await?.ok_or_else(|| DomainError::Validation("Connect Antigravity with OAuth first".into()))?;
                let base = self.resolver.get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("antigravity")).await?.unwrap_or_else(|| "https://cloudcode-pa.googleapis.com".into());
                super::antigravity::generate_content(&base, &token, req).await
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
                self.execute_openai_compatible(
                    &base_url,
                    token.as_deref(),
                    requested_model,
                    req,
                    "Local Inference",
                )
                .await
            }
            "groq" | "openrouter" | "together" | "mistral" | "cerebras" | "perplexity" => {
                let default_base = match target_provider {
                    "groq" => "https://api.groq.com/openai/v1",
                    "openrouter" => "https://openrouter.ai/api/v1",
                    "together" => "https://api.together.xyz/v1",
                    "mistral" => "https://api.mistral.ai/v1",
                    "cerebras" => "https://api.cerebras.ai/v1",
                    "perplexity" => "https://api.perplexity.ai",
                    _ => unreachable!(),
                };
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
                let base_url = custom_url.unwrap_or_else(|| default_base.to_string());
                let prefix = format!("{target_provider}/");
                let model = requested_model.strip_prefix(&prefix).unwrap_or(requested_model);
                self.execute_openai_compatible(&base_url, Some(&token), model, req, target_provider)
                    .await
            }
            "mock" => {
                let content = format!(
                    "Custos Sovereign (Offline Mode): Yêu cầu của bạn đã được tiếp nhận thành công ({} ký tự prompt). Hệ thống backend đang hoạt động ổn định ở chế độ Offline.",
                    req.prompt.chars().count()
                );
                Ok(ModelResponse::text(
                    content,
                    if requested_model.is_empty() {
                        "custos-offline".to_string()
                    } else {
                        requested_model.to_string()
                    },
                    42,
                ))
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
pub fn format_provider_error(
    status: reqwest::StatusCode,
    body: &str,
    provider_label: &str,
) -> String {
    if provider_label == "OpenAI"
        && (body.contains("credit_balance_exhausted") || body.contains("insufficient_quota"))
    {
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
                if matches!(
                    val.get("type").and_then(|t| t.as_str()),
                    Some("error" | "response.failed" | "response.incomplete")
                ) {
                    let message = val
                        .pointer("/error/message")
                        .or_else(|| val.pointer("/response/error/message"))
                        .or_else(|| val.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Response failed or incomplete");
                    return Err(DomainError::Validation(format!(
                        "ChatGPT Codex stream error: {message}"
                    )));
                }
                completed |= val.get("type").and_then(|t| t.as_str()) == Some("response.completed");
                if let Some(delta) = val.get("delta").and_then(|d| d.as_str()) {
                    content.push_str(delta);
                }
                if let Some(usage) = val.get("response").and_then(|r| r.get("usage")) {
                    if let Some(total) = usage.get("total_tokens").and_then(|t| t.as_u64()) {
                        tokens_used = Some(total as usize);
                    }
                } else if let Some(total) = val
                    .get("usage")
                    .and_then(|u| u.get("total_tokens"))
                    .and_then(|t| t.as_u64())
                {
                    tokens_used = Some(total as usize);
                }
            }
        }
    }

    if !completed {
        return Err(DomainError::Validation(
            "ChatGPT Codex stream ended before completion".into(),
        ));
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

    #[test]
    fn ollama_namespace_wins_over_cloud_model_names() {
        for model in [
            "ollama/gpt-custom:latest",
            "ollama/claude-local",
            "ollama/gemini",
            "ollama/custom:tag",
        ] {
            assert_eq!(RouterModelProvider::classify_provider(model), "local");
        }
    }

    #[tokio::test]
    async fn ollama_native_request_and_errors() {
        use std::io::{Read, Write};
        for (status, body) in [
            (
                "200 OK",
                r#"{"message":{"content":"Local answer"},"done":true,"prompt_eval_count":7,"eval_count":3}"#,
            ),
            ("404 Not Found", r#"{"error":"model not found"}"#),
            ("200 OK", r#"{"message":{},"done":true}"#),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut buf = [0; 4096];
                    let n = socket.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some((headers, payload)) = text.split_once("\r\n\r\n") {
                        let length: usize = headers
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length: ")
                                    .and_then(|v| v.parse().ok())
                            })
                            .unwrap_or(0);
                        if payload.len() >= length {
                            break;
                        }
                    }
                }
                write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                String::from_utf8(request).unwrap()
            });
            let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
                oauth: false,
                endpoint: format!("{base}/v1/"),
                fail: true,
            }));
            let mut req = ProviderRequest::simple("hello local");
            req.model = "ollama/gpt-custom:tag".into();
            let result = provider.generate(&req).await;
            if body.contains("Local answer") {
                let result = result.unwrap();
                assert_eq!(result.content, "Local answer");
                assert_eq!(result.tokens_used, 10);
                assert_eq!(result.model_id, req.model);
            } else {
                assert!(result.is_err());
            }
            let request = server.join().unwrap();
            assert!(request.starts_with("POST /api/chat "));
            assert!(!request.to_lowercase().contains("authorization:"));
            let payload: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(payload["model"], "gpt-custom:tag");
            assert_eq!(payload["stream"], false);
            assert_eq!(payload["options"]["num_predict"], 4096);
        }
    }

    #[tokio::test]
    async fn ollama_stream_payload_routes_custom_tag_without_credentials() {
        use wiremock::{Mock,MockServer,ResponseTemplate};
        use wiremock::matchers::{method,path,body_partial_json};
        let server=MockServer::start().await;
        Mock::given(method("POST")).and(path("/api/chat")).and(body_partial_json(serde_json::json!({"model":"gemini-custom:tag","stream":true,"messages":[{"role":"user","content":"hello local"}],"options":{"temperature":0.25,"num_predict":32}})))
            .respond_with(ResponseTemplate::new(200).set_body_string("{\"message\":{\"content\":\"hello\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":7,\"eval_count\":2}\n").insert_header("content-type","application/x-ndjson"))
            .expect(1).mount(&server).await;
        let provider=RouterModelProvider::new(Arc::new(TestCredentialResolver{oauth:false,endpoint:format!("{}/v1/",server.uri()),fail:true}));
        let mut req=ProviderRequest::simple("hello local");req.model="ollama/gemini-custom:tag".into();req.temperature=Some(0.25);req.max_tokens=Some(32);
        let mut receiver=provider.stream(&req).await.unwrap();assert_eq!(receiver.recv().await.unwrap().text.as_deref(),Some("hello"));
        let mut completed=false;while let Some(event)=receiver.recv().await {if event.event_type==custos_provider::ProviderEventType::Completed {assert_eq!(event.usage.unwrap().output_tokens,Some(2));completed=true;}}
        assert!(completed);assert!(!server.received_requests().await.unwrap()[0].headers.contains_key("authorization"));
    }

    #[tokio::test]
    async fn ollama_request_timeout_is_reported() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(150));
        });
        let mut provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth: false,
            endpoint: base,
            fail: true,
        }));
        provider.http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(30))
            .build()
            .unwrap();
        let mut req = ProviderRequest::simple("hello");
        req.model = "ollama/custom".into();
        assert!(provider
            .generate(&req)
            .await
            .unwrap_err()
            .to_string()
            .contains("Ollama connection failed"));
        server.join().unwrap();
    }

    #[tokio::test]
    #[ignore = "requires a running local Ollama with qwen2.5:0.5b installed"]
    async fn ollama_live_smoke() {
        let models =
            super::super::probe::probe_endpoint_models("http://localhost:11434/v1", None, "ollama")
                .await
                .unwrap();
        assert!(models.iter().any(|m| m.id == "ollama/qwen2.5:0.5b"));
        let provider = RouterModelProvider::new(Arc::new(EnvCredentialResolver));
        let mut req = ProviderRequest::simple("Reply with the word hello.");
        req.model = "ollama/qwen2.5:0.5b".into();
        req.max_tokens = Some(32);
        let response = provider.generate(&req).await.unwrap();
        assert!(!response.content.trim().is_empty());
        assert!(response.tokens_used > 0);
    }

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

    struct AccountResolver { endpoint: String }
    #[async_trait]
    impl CredentialResolver for AccountResolver {
        async fn get_token(&self, id: &str) -> Result<Option<String>,DomainError> { Ok(Some(format!("{id}-key"))) }
        async fn get_endpoint_url(&self, id: &str) -> Result<Option<String>,DomainError> { assert!(id.starts_with("account-")); Ok(Some(self.endpoint.clone())) }
    }
    #[tokio::test]
    async fn explicitly_selected_accounts_use_their_own_credentials_and_usage() {
        use wiremock::{Mock,MockServer,ResponseTemplate};
        use wiremock::matchers::{method,path,header};
        let server=MockServer::start().await;
        for account in ["account-A","account-B"] {
            Mock::given(method("POST")).and(path("/chat/completions")).and(header("authorization",format!("Bearer {account}-key"))).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"model":"gpt-test","choices":[{"message":{"content":"answer"}}],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12}}))).expect(1).mount(&server).await;
        }
        let provider=RouterModelProvider::new(Arc::new(AccountResolver { endpoint:server.uri() }));
        for account in ["account-A","account-B"] {
            let mut req=ProviderRequest::simple("hello"); req.model="gpt-test".into(); req.provider_account_id=Some(account.into());
            let response=provider.generate(&req).await.unwrap();
            assert_eq!(response.input_tokens,Some(7)); assert_eq!(response.output_tokens,Some(5)); assert_eq!(response.total_tokens,Some(12));
        }
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
        let formatted =
            format_provider_error(reqwest::StatusCode::TOO_MANY_REQUESTS, err_body, "OpenAI");
        assert!(formatted.contains("credit_balance_exhausted"));
        assert!(
            formatted.contains("ChatGPT Plus subscription ($20/mo) only covers web/app ChatGPT")
        );
        assert!(formatted.contains("https://platform.openai.com/settings/organization/billing/"));
    }

    #[test]
    fn test_format_provider_error_standard() {
        let err_body = r#"{"error": "invalid model"}"#;
        let formatted = format_provider_error(reqwest::StatusCode::BAD_REQUEST, err_body, "OpenAI");
        assert_eq!(
            formatted,
            "OpenAI API error 400 Bad Request: {\"error\": \"invalid model\"}"
        );
    }

    #[test]
    fn test_classify_provider() {
        assert_eq!(RouterModelProvider::classify_provider("gpt-4o"), "openai");
        assert_eq!(
            RouterModelProvider::classify_provider("gpt-4o-mini"),
            "openai"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("gpt-5.6-sol"),
            "openai"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("gpt-5.6-terra"),
            "openai"
        );
        assert_eq!(RouterModelProvider::classify_provider("o3-mini"), "openai");
        assert_eq!(RouterModelProvider::classify_provider("o1"), "openai");
        assert_eq!(
            RouterModelProvider::classify_provider("claude-3-7-sonnet"),
            "anthropic"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("claude-3-5-haiku"),
            "anthropic"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("gemini-2.5-flash"),
            "gemini"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("gemini-2.5-pro"),
            "gemini"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("deepseek-chat"),
            "deepseek"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("deepseek-reasoner"),
            "deepseek"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("llama3.3:70b"),
            "local"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("qwen2.5-coder:32b"),
            "local"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("groq/llama-3.3-70b"),
            "groq"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("openrouter/auto"),
            "openrouter"
        );
        assert_eq!(
            RouterModelProvider::classify_provider("together/deepseek-r1"),
            "together"
        );
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
        async fn get_endpoint_url(
            &self,
            _provider_id: &str,
        ) -> Result<Option<String>, DomainError> {
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
