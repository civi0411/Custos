//! Model Endpoint Probe
//!
//! Discovers the models and context window sizes a custom or remote AI endpoint serves.
//! Runs in Rust daemon/adapters to avoid webview CORS limitations and enforce strict timeout bounds.
//! Adapted from Open Science Desktop model probe and extended for Custos provider protocol.

use custos_domain::{DomainError, ProbedModel};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::time::Duration;

const PROBE_TIMEOUT_SECS: u64 = 5;
const PROBE_USER_AGENT: &str = "Custos-Model-Probe/1.0";

/// Probes a live AI endpoint URL and returns the list of served models and their context window limits.
pub async fn probe_endpoint_models(
    base_url: &str,
    api_key: Option<&str>,
    provider_type: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(DomainError::Validation("Endpoint Base URL is empty".into()));
    }

    let client = reqwest::Client::builder()
        .user_agent(PROBE_USER_AGENT)
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| DomainError::Validation(format!("Failed to build probe HTTP client: {e}")))?;

    // 1. If provider_type is ollama or url hints at ollama (:11434), try Ollama native probe first
    if provider_type == "ollama" || provider_type == "local" || base.contains(":11434") {
        let ollama_root = base.trim_end_matches("/v1");
        if let Ok(models) = probe_ollama_endpoint(&client, ollama_root).await {
            if !models.is_empty() {
                return Ok(models);
            }
        }
    }

    // 2. Route by provider type or fallback to OpenAI compatible
    match provider_type {
        "anthropic" => probe_anthropic_endpoint(&client, base, api_key).await,
        "gemini" | "google" => probe_gemini_endpoint(&client, base, api_key).await,
        _ => probe_openai_endpoint(&client, base, api_key).await,
    }
}

/// Ollama native API:
/// GET /api/tags -> lists tags/models
/// POST /api/show with {"model": name} -> inspects model_info["<arch>.context_length"]
async fn probe_ollama_endpoint(
    client: &reqwest::Client,
    root: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = format!("{root}/api/tags");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Ollama probe connect error: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Ollama /api/tags returned status: {}",
            resp.status()
        )));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to read Ollama body: {e}")))?;

    let names = parse_ollama_tags(&text).ok_or_else(|| {
        DomainError::Validation("Invalid Ollama /api/tags response schema".into())
    })?;

    let mut probed = Vec::new();
    for name in names {
        // Query show endpoint for context length (best-effort)
        let show_url = format!("{root}/api/show");
        let context = match client
            .post(&show_url)
            .json(&serde_json::json!({ "model": &name }))
            .send()
            .await
        {
            Ok(show_resp) if show_resp.status().is_success() => {
                if let Ok(show_text) = show_resp.text().await {
                    parse_ollama_show_context(&show_text)
                } else {
                    None
                }
            }
            _ => None,
        };

        probed.push(ProbedModel {
            id: name.clone(),
            name: Some(name),
            context_window: context,
            owned_by: Some("ollama".into()),
            description: None,
        });
    }

    Ok(probed)
}

pub fn parse_ollama_tags(body: &str) -> Option<Vec<String>> {
    let v: Value = serde_json::from_str(body).ok()?;
    let models = v.get("models")?.as_array()?;
    let names: Vec<String> = models
        .iter()
        .filter_map(|m| m.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    Some(names)
}

pub fn parse_ollama_show_context(body: &str) -> Option<u64> {
    let v: Value = serde_json::from_str(body).ok()?;
    let info = v.get("model_info")?.as_object()?;
    info.iter()
        .find(|(k, _)| k.ends_with(".context_length"))
        .and_then(|(_, v)| v.as_u64())
}

/// OpenAI-compatible: GET /models or /v1/models
async fn probe_openai_endpoint(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{base}/models")
    };

    let mut req = client.get(&url);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        req = req.header(AUTHORIZATION, format!("Bearer {}", key.trim()));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to reach {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "OpenAI-compatible models probe returned HTTP status: {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed reading response from {url}: {e}")))?;

    parse_openai_models(&body)
}

pub fn parse_openai_models(body: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| DomainError::Validation(format!("Models response was not valid JSON: {e}")))?;

    // Standard shape is { "data": [...] }; some servers return a bare array
    let list = v
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| v.as_array())
        .ok_or_else(|| DomainError::Validation("Endpoint returned no model list array".into()))?;

    let models: Vec<ProbedModel> = list
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            let name = m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
            let owned_by = m.get("owned_by").and_then(|o| o.as_str()).map(|s| s.to_string());
            let description = m.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
            let context = ["context_length", "max_model_len", "max_context_length", "context_window"]
                .iter()
                .find_map(|k| m.get(*k).and_then(|c| c.as_u64()));

            Some(ProbedModel {
                id,
                name,
                context_window: context,
                owned_by,
                description,
            })
        })
        .collect();

    if models.is_empty() {
        return Err(DomainError::Validation("The endpoint listed zero models".into()));
    }

    Ok(models)
}

/// Anthropic: GET /models with x-api-key and anthropic-version
async fn probe_anthropic_endpoint(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{base}/models")
    };

    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        if let Ok(val) = HeaderValue::from_str(key.trim()) {
            headers.insert("x-api-key", val);
        }
    }

    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to reach Anthropic models at {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Anthropic models endpoint returned status: {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed reading Anthropic response: {e}")))?;

    parse_openai_models(&body)
}

/// Google Gemini: GET /v1beta/models
async fn probe_gemini_endpoint(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
) -> Result<Vec<ProbedModel>, DomainError> {
    let mut url = if base.ends_with("/models") {
        base.to_string()
    } else {
        format!("{base}/v1beta/models")
    };

    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        url = format!("{url}?key={}", key.trim());
    }

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to reach Gemini models at {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Gemini models endpoint returned status: {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed reading Gemini response: {e}")))?;

    parse_gemini_models(&body)
}

pub fn parse_gemini_models(body: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| DomainError::Validation(format!("Gemini response was not valid JSON: {e}")))?;

    let models = v
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| DomainError::Validation("Gemini response missing 'models' array".into()))?;

    let probed: Vec<ProbedModel> = models
        .iter()
        .filter_map(|m| {
            let full_name = m.get("name")?.as_str()?;
            // Format is "models/gemini-2.5-pro", strip prefix
            let id = full_name.strip_prefix("models/").unwrap_or(full_name).to_string();
            let display_name = m.get("displayName").and_then(|d| d.as_str()).map(|s| s.to_string());
            let description = m.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
            let context = m.get("inputTokenLimit").and_then(|c| c.as_u64());

            Some(ProbedModel {
                id,
                name: display_name,
                context_window: context,
                owned_by: Some("google".into()),
                description,
            })
        })
        .collect();

    if probed.is_empty() {
        return Err(DomainError::Validation("No Gemini models discovered".into()));
    }

    Ok(probed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ollama_tags() {
        let body = r#"{"models":[{"name":"llama3:8b","size":4000000},{"name":"qwen3:14b"}]}"#;
        let tags = parse_ollama_tags(body).unwrap();
        assert_eq!(tags, vec!["llama3:8b".to_string(), "qwen3:14b".to_string()]);
    }

    #[test]
    fn test_parse_ollama_show_context() {
        let body = r#"{
            "model_info": {
                "llama.context_length": 131072,
                "general.architecture": "llama"
            }
        }"#;
        assert_eq!(parse_ollama_show_context(body), Some(131072));
    }

    #[test]
    fn test_parse_openai_models() {
        let body = r#"{
            "data": [
                {
                    "id": "qwen2.5-coder-32b",
                    "owned_by": "vllm",
                    "max_model_len": 65536
                },
                {
                    "id": "gpt-4o",
                    "context_length": 128000
                }
            ]
        }"#;
        let models = parse_openai_models(body).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "qwen2.5-coder-32b");
        assert_eq!(models[0].context_window, Some(65536));
        assert_eq!(models[0].owned_by, Some("vllm".into()));
        assert_eq!(models[1].id, "gpt-4o");
        assert_eq!(models[1].context_window, Some(128000));
    }

    #[test]
    fn test_parse_gemini_models() {
        let body = r#"{
            "models": [
                {
                    "name": "models/gemini-2.5-pro",
                    "displayName": "Gemini 2.5 Pro",
                    "description": "State-of-the-art multimodal model",
                    "inputTokenLimit": 1048576
                }
            ]
        }"#;
        let models = parse_gemini_models(body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-2.5-pro");
        assert_eq!(models[0].name, Some("Gemini 2.5 Pro".into()));
        assert_eq!(models[0].context_window, Some(1048576));
    }
}
