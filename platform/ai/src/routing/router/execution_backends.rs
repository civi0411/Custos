use custos_domain::DomainError;
use custos_provider::{ModelResponse, ProviderRequest};
use tracing::{debug, error, info};

use super::classification::{format_provider_error, parse_codex_sse_stream, responses_payload};
use super::provider::RouterModelProvider;
use super::wire_types::{
    AnthropicMessageItem, AnthropicMessagesRequest, AnthropicMessagesResponse, OpenAiChatRequest,
    OpenAiChatRequestMessage, OpenAiChatResponse,
};

impl RouterModelProvider {
    pub(crate) async fn execute_openai_compatible(
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

    pub(crate) async fn execute_ollama(
        &self,
        base: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        let root = crate::routing::probe::ollama_root(base)?;
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

    pub(crate) async fn execute_chatgpt_codex(
        &self,
        token: &str,
        model: &str,
        req: &ProviderRequest,
    ) -> Result<ModelResponse, DomainError> {
        self.execute_responses(token, model, req, &self.codex_endpoint, true)
            .await
    }

    pub(crate) async fn execute_responses(
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

        let payload = responses_payload(req, target_model);

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
                .header("User-Agent", crate::routing::probe::codex_user_agent())
                .header("Originator", "codex_cli_rs");
        }
        if let Some(account_id) = codex
            .then(|| crate::cloud::transport::extract_chatgpt_account_id(token))
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

    pub(crate) async fn execute_anthropic(
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
}
