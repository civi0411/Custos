//! Unified Router Model Provider
//!
//! Routes model inference requests to live provider endpoints (OpenAI, Anthropic,
//! Gemini, DeepSeek, Local/Ollama) based on requested model name and stored credentials.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelResponse, ProviderRequest};
use std::sync::Arc;
use tracing::{debug, error, info};

pub mod catalog;
pub mod credentials;
pub(crate) mod wire_types;

pub use catalog::*;
pub use credentials::*;
pub(crate) use wire_types::*;

#[derive(Clone)]
pub struct RouterModelProvider {
    resolver: Arc<dyn CredentialResolver>,
    http_client: reqwest::Client,
    codex_endpoint: String,
    native_local: Option<Arc<dyn custos_provider::ModelPort>>,
}

impl RouterModelProvider {
    async fn checked_response(
        &self,
        result: Result<reqwest::Response, reqwest::Error>,
        req: &ProviderRequest,
    ) -> Result<reqwest::Response, DomainError> {
        let account = req.provider_account_id.as_deref();
        let mut response = match result {
            Ok(response) => response,
            Err(_) => {
                if let Some(account) = account {
                    self.resolver
                        .observe_response(account, &req.model, 0, "", None)
                        .await?;
                }
                let label = if req.model.starts_with("ollama/") {
                    "Ollama"
                } else {
                    "Provider"
                };
                return Err(DomainError::Validation(format!(
                    "{label} connection failed; refresh models to reconnect"
                )));
            }
        };
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let retry_at = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|value| {
                value
                    .parse::<i64>()
                    .ok()
                    .map(|seconds| {
                        chrono::Utc::now()
                            .timestamp_millis()
                            .saturating_add(seconds.max(0).saturating_mul(1000))
                    })
                    .or_else(|| {
                        chrono::DateTime::parse_from_rfc2822(value)
                            .ok()
                            .map(|date| date.timestamp_millis())
                    })
            });
        let mut body = Vec::new();
        while body.len() < 16_384 {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let remaining = 16_384 - body.len();
                    body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                }
                _ => break,
            }
        }
        let body = String::from_utf8_lossy(&body);
        if let Some(account) = account {
            self.resolver
                .observe_response(account, &req.model, status, &body, retry_at)
                .await?;
        }
        tracing::debug!(task_id = %req.task_id, run_id = %req.request_id, status, "Provider request rejected");
        Err(DomainError::Validation(format!(
            "Provider request rejected (HTTP {status}); refresh models or inspect account limits"
        )))
    }

    async fn observe_adapter_error<T>(
        &self,
        result: Result<T, DomainError>,
        req: &ProviderRequest,
    ) -> Result<T, DomainError> {
        if let (Err(error), Some(account)) = (&result, req.provider_account_id.as_deref()) {
            let message = error.to_string();
            let status = message
                .split("returned ")
                .nth(1)
                .and_then(|part| part.split_whitespace().next())
                .and_then(|part| part.trim_end_matches(':').parse::<u16>().ok())
                .or_else(|| {
                    (message.contains("429") || message.contains("RESOURCE_EXHAUSTED"))
                        .then_some(429)
                })
                .or_else(|| message.contains("connection failed").then_some(0));
            if let Some(status) = status {
                self.resolver
                    .observe_response(account, &req.model, status, &message, None)
                    .await?;
            }
        }
        result
    }

    fn responses_payload(req: &ProviderRequest, model: &str) -> serde_json::Value {
        let mut payload = serde_json::json!({"model":model,"input":[{"role":"user","content":[{"type":"input_text","text":req.prompt}]}],"instructions":"You are a helpful assistant.","store":false,"stream":true});
        if let Some(effort) = &req.reasoning_effort {
            payload["reasoning"] = serde_json::json!({"effort":effort});
        }
        payload
    }

    pub fn new(resolver: Arc<dyn CredentialResolver>) -> Self {
        Self {
            resolver,
            codex_endpoint: "https://chatgpt.com/backend-api/codex/responses".into(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            native_local: None,
        }
    }

    pub fn with_native_local(mut self, native_local: Arc<dyn custos_provider::ModelPort>) -> Self {
        self.native_local = Some(native_local);
        self
    }

    pub fn with_env_fallback() -> Self {
        Self::new(Arc::new(EnvCredentialResolver))
    }

    /// Determines the provider service type from a given model identifier
    pub fn classify_provider(model: &str) -> &'static str {
        let m = model.trim().to_lowercase();
        if m.starts_with("native:") {
            return "native-local";
        }
        if m.starts_with("ollama/") || m.starts_with("local/") || m.starts_with("local:") {
            return "local";
        }
        if m.starts_with("lmstudio/") || m == "lmstudio" {
            return "lmstudio";
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
        if let Some((_, service)) = find_by_model(&m) {
            return service;
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
            "nvidia" => "NVIDIA_API_KEY",
            "moonshot" | "kimi" => "MOONSHOT_API_KEY",
            "zhipu" | "zai" => "ZHIPU_API_KEY",
            "minimax" => "MINIMAX_API_KEY",
            "alibaba" | "qwen" => "DASHSCOPE_API_KEY",
            "xai" => "XAI_API_KEY",
            "sambanova" => "SAMBANOVA_API_KEY",
            "fireworks" => "FIREWORKS_API_KEY",
            "siliconflow" => "SILICONFLOW_API_KEY",
            "nebius" => "NEBIUS_API_KEY",
            "hyperbolic" => "HYPERBOLIC_API_KEY",
            "cohere" => "COHERE_API_KEY",
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
            reasoning_effort: req.reasoning_effort.clone(),
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
        let resp = self.checked_response(req_builder.send().await, req).await?;

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

        Ok(ModelResponse::text(content, resolved_model, tokens_used)
            .with_usage(input, output, total))
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
        }));
        let response = self.checked_response(response.send().await, req).await?;
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
        )
        .with_usage(
            value
                .get("prompt_eval_count")
                .and_then(|v| v.as_u64())
                .and_then(|v| usize::try_from(v).ok()),
            value
                .get("eval_count")
                .and_then(|v| v.as_u64())
                .and_then(|v| usize::try_from(v).ok()),
            None,
        ))
    }

    async fn execute_chatgpt_codex(
        &self,
        token: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        self.execute_responses(token, model, req, &self.codex_endpoint, true)
            .await
    }

    async fn execute_responses(
        &self,
        token: &str,
        model: &str,
        req: &ProviderRequest,
        endpoint: &str,
        codex: bool,
    ) -> Result<ModelResponse, DomainError> {
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

        let payload = Self::responses_payload(req, target_model);

        info!(
            model = %target_model,
            endpoint = %endpoint,
            prompt_len = req.prompt.len(),
            codex_subscription = codex,
            "Dispatching live Responses request"
        );

        let mut req_builder = self
            .http_client
            .post(endpoint)
            .bearer_auth(token)
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .json(&payload);

        if codex {
            req_builder = req_builder
                .header("User-Agent", super::probe::codex_user_agent())
                .header("Originator", "codex_cli_rs");
        }
        if let Some(account_id) = codex
            .then(|| crate::providers::oauth_pkce::extract_chatgpt_account_id(token))
            .flatten()
        {
            req_builder = req_builder.header("ChatGPT-Account-ID", account_id);
        }

        let resp = self.checked_response(req_builder.send().await, req).await?;

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
        let reported = text_body
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
            .find(|v| v.get("type").and_then(|v| v.as_str()) == Some("response.completed"));
        let usage = reported.as_ref().and_then(|v| v.pointer("/response/usage"));
        let count = |key: &str| {
            usage
                .and_then(|v| v.get(key))
                .and_then(|v| v.as_u64())
                .and_then(|v| usize::try_from(v).ok())
        };

        debug!(
            model = %target_model,
            tokens_used = tokens_used,
            content_len = content.len(),
            "Received live response from ChatGPT Codex"
        );

        Ok(
            ModelResponse::text(content, target_model.to_string(), tokens_used).with_usage(
                count("input_tokens"),
                count("output_tokens"),
                count("total_tokens"),
            ),
        )
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
        let resp = self.checked_response(builder.send().await, req).await?;

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

        Ok(ModelResponse::text(content, resolved_model, tokens_used)
            .with_usage(input, output, None))
    }

    pub(crate) async fn send_stream_request(
        &self,
        req: &ProviderRequest,
    ) -> Result<(reqwest::Response, super::streaming::Wire, String, String), DomainError> {
        use super::streaming::Wire;
        if let Some(account) = &req.provider_account_id {
            self.resolver
                .validate_model(account, &req.model, req.reasoning_effort.as_deref())
                .await?;
        }
        let model = req.model.trim();
        let service = Self::classify_provider(model);
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
                .unwrap_or_else(|| Self::classify_provider(model))
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
                    super::antigravity::stream_codeassist(codeassist_mode, &base, &token, req)
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
            let root = super::probe::ollama_root(&local_base)?;
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
            let request=self.http_client.post(format!("{root}/api/chat")).json(&serde_json::json!({"model":clean_model,"messages":[{"role":"user","content":req.prompt}],"stream":true,"options":options}));
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
                .header("User-Agent", super::probe::codex_user_agent())
                .header("Originator", "codex_cli_rs")
                .header("Accept", "text/event-stream")
                .json(&Self::responses_payload(req, m));
            if let Some(id) =
                super::oauth_pkce::extract_chatgpt_account_id(token.as_deref().unwrap_or(""))
            {
                request = request.header("ChatGPT-Account-ID", id);
            }
        } else if service == "openai"
            && custos_provider::formats::openai::is_openai_responses_model(model)
        {
            wire = Wire::Codex;
            effective_model = model.to_string();
            let base = custom.unwrap_or_else(|| "https://api.openai.com/v1".into());
            request = self
                .http_client
                .post(format!("{}/responses", base.trim_end_matches('/')))
                .bearer_auth(token.as_deref().unwrap_or(""))
                .json(&Self::responses_payload(req, model));
        } else if service == "anthropic" {
            wire = Wire::Claude;
            let base = custom.unwrap_or_else(|| "https://api.anthropic.com/v1".into());
            let target = if matches!(model, "" | "default" | "anthropic") {
                "claude-3-7-sonnet-20250219"
            } else {
                model
            };
            effective_model = target.to_string();
            let mut payload = serde_json::json!({"model":target,"messages":[{"role":"user","content":req.prompt}],"max_tokens":req.max_tokens.unwrap_or(4096),"stream":true});
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
                        super::oauth_pkce::copilot_session_token(token.as_deref().unwrap_or(""))
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
            let mut payload = serde_json::json!({"model":m,"messages":[{"role":"user","content":req.prompt}],"stream":true,"stream_options":{"include_usage":true}});
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

    async fn dispatch_stream_attempt(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let service = Self::classify_provider(&req.model);
        if service == "native-local" {
            let local = self.native_local.as_ref().ok_or_else(|| {
                DomainError::Validation(
                    "Native local inference runtime is not configured".to_string(),
                )
            })?;
            return local.stream(req).await;
        }
        let (response, wire, _, _) = self.send_stream_request(req).await?;
        Ok(super::streaming::stream_response_observed(
            response,
            req.clone(),
            wire,
            self.resolver.clone(),
        ))
    }

    async fn dispatch_generate_attempt(
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
                .unwrap_or_else(|| Self::classify_provider(requested_model))
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
                let token = super::oauth_pkce::copilot_session_token(&github).await?;
                let model = requested_model.strip_prefix("copilot/").ok_or_else(|| DomainError::Validation("Invalid Copilot model".into()))?;
                self.execute_openai_compatible("https://api.githubcopilot.com", Some(&token), model, req, "GitHub Copilot").await
            }
            "gemini-oauth" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("gemini")).await?.ok_or_else(|| DomainError::Validation("Connect Gemini OAuth first".into()))?;
                self.observe_adapter_error(super::antigravity::generate_codeassist_content("gemini", "https://cloudcode-pa.googleapis.com", &token, req).await, req).await
            }
            "google" | "antigravity" => {
                let token = self.resolver.get_token(req.provider_account_id.as_deref().unwrap_or("google")).await?;
                let token = match token {
                    Some(t) => t,
                    None => self.resolver.get_token("antigravity").await?.ok_or_else(|| DomainError::Validation("Connect Google OAuth first".into()))?,
                };
                let base = self.resolver.get_endpoint_url(req.provider_account_id.as_deref().unwrap_or("google")).await?.unwrap_or_else(|| "https://cloudcode-pa.googleapis.com".into());
                self.observe_adapter_error(super::antigravity::generate_content(&base, &token, req).await, req).await
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
                if custos_provider::formats::openai::is_openai_responses_model(model) {
                    return self.execute_responses(&token, model, req, &format!("{}/responses",base_url.trim_end_matches('/')), false).await;
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

#[async_trait]
impl ModelProvider for RouterModelProvider {
    fn provider_id(&self) -> &str {
        "unified-router"
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let mut routed = req.clone();
        let service = Self::classify_provider(&routed.model);
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        if service == "native-local" {
            routed.provider_account_id = None;
        } else if routed.provider_account_id.is_none() {
            routed.provider_account_id = self
                .resolver
                .select_account(candidate_service, &routed.model)
                .await?;
        }

        let max_retries = 3;
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.dispatch_stream_attempt(&routed).await {
                Ok(rx) => return Ok(rx),
                Err(err) => {
                    let err_str = err.to_string();
                    let is_rate_or_quota = err_str.contains("429")
                        || err_str.contains("RESOURCE_EXHAUSTED")
                        || err_str.contains("quota exhausted")
                        || err_str.contains("rate limited")
                        || err_str.contains("temporary rate limit")
                        || err_str.contains("Model unavailable");

                    if is_rate_or_quota && attempts < max_retries && service != "native-local" {
                        if let Ok(Some(next_account)) = self
                            .resolver
                            .select_account(candidate_service, &routed.model)
                            .await
                        {
                            if routed.provider_account_id.as_deref() != Some(&next_account) {
                                tracing::info!(
                                    service,
                                    model = %routed.model,
                                    previous_account = ?routed.provider_account_id,
                                    failover_account = %next_account,
                                    "Rate limit/429 encountered; auto-failing over to next available account in pool"
                                );
                                routed.provider_account_id = Some(next_account);
                                continue;
                            }
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut routed = req.clone();
        let service = Self::classify_provider(&routed.model);
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        if service == "native-local" {
            routed.provider_account_id = None;
        } else if routed.provider_account_id.is_none() {
            routed.provider_account_id = self
                .resolver
                .select_account(candidate_service, &routed.model)
                .await?;
        }

        let max_retries = 3;
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.dispatch_generate_attempt(&routed).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let err_str = err.to_string();
                    let is_rate_or_quota = err_str.contains("429")
                        || err_str.contains("RESOURCE_EXHAUSTED")
                        || err_str.contains("quota exhausted")
                        || err_str.contains("rate limited")
                        || err_str.contains("temporary rate limit")
                        || err_str.contains("Model unavailable");

                    if is_rate_or_quota && attempts < max_retries && service != "native-local" {
                        if let Ok(Some(next_account)) = self
                            .resolver
                            .select_account(candidate_service, &routed.model)
                            .await
                        {
                            if routed.provider_account_id.as_deref() != Some(&next_account) {
                                tracing::info!(
                                    service,
                                    model = %routed.model,
                                    previous_account = ?routed.provider_account_id,
                                    failover_account = %next_account,
                                    "Rate limit/429 encountered; auto-failing over to next available account in pool"
                                );
                                routed.provider_account_id = Some(next_account);
                                continue;
                            }
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn execute_turn(
        &self,
        req: custos_provider::turn::ModelTurnRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::turn::ModelTurnEvent>, DomainError>
    {
        let service = Self::classify_provider(&req.requested_model);
        if service == "native-local" {
            let local = self.native_local.as_ref().ok_or_else(|| {
                DomainError::Validation(
                    "Native local inference runtime is not configured".to_string(),
                )
            })?;
            return local.execute_turn(req).await;
        }

        let flat_prompt = req
            .structured_messages
            .iter()
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|c| match c {
                        custos_provider::conversation::message::MessageContentBlock::Text(t) => {
                            Some(t.text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut provider_req = custos_provider::request::ProviderRequest::new(
            req.attempt_id.to_string(),
            req.task_id.to_string(),
            1,
            flat_prompt,
            req.requested_model.clone(),
        );
        let candidate_service = if service == "gemini-oauth" {
            "gemini"
        } else {
            service
        };
        provider_req.provider_account_id = self
            .resolver
            .select_account(candidate_service, &provider_req.model)
            .await?;

        match self.send_stream_request(&provider_req).await {
            Ok((resp, wire, actual_provider, actual_model)) => {
                Ok(super::streaming::stream_turn_response_observed(
                    resp,
                    req.attempt_id,
                    actual_provider,
                    actual_model,
                    wire,
                    std::time::Duration::from_secs(45),
                ))
            }
            Err(err) => {
                let (tx, rx) = tokio::sync::mpsc::channel(2);
                let attempt_id = req.attempt_id;
                let provider_id = self.provider_id().to_string();
                let requested_model = req.requested_model.clone();
                tokio::spawn(async move {
                    let _ = tx
                        .send(custos_provider::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 1,
                            actual_provider: Some(provider_id),
                            actual_model: Some(requested_model),
                            actual_transport: None,
                            delta: custos_provider::turn::TurnDelta::Failed {
                                error_class: "router_error".to_string(),
                                message: err.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        })
                        .await;
                });
                Ok(rx)
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
mod tests;
