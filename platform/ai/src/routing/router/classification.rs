use custos_domain::DomainError;
use custos_provider::ProviderRequest;

use super::catalog::find_by_model;

/// Determines the provider service type from a given model identifier.
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

pub(crate) fn responses_payload(req: &ProviderRequest, model: &str) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": model,
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": req.prompt
            }]
        }],
        "instructions": "You are a helpful assistant.",
        "store": false,
        "stream": true
    });
    if let Some(effort) = &req.reasoning_effort {
        payload["reasoning"] = serde_json::json!({ "effort": effort });
    }
    payload
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
