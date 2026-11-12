use custos_domain::{DomainError, ProbedModel};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::time::Duration;

use super::metadata::{
    advertised_default_effort, advertised_efforts, codex_user_agent, model_is_selectable,
    CODEX_CLIENT_VERSION,
};
use super::ollama::{ollama_root, probe_ollama_endpoint};

const PROBE_TIMEOUT_SECS: u64 = 5;
const PROBE_USER_AGENT: &str = "Custos-Model-Probe/1.0";

/// Probes a live AI endpoint URL and returns the list of served models and their context window limits.
pub async fn probe_endpoint_models(
    base_url: &str,
    api_key: Option<&str>,
    provider_type: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    if provider_type == "antigravity" {
        return crate::cloud::google_codeassist::probe_models(
            base_url,
            api_key.ok_or_else(|| {
                DomainError::Validation("Antigravity OAuth credentials missing".into())
            })?,
        )
        .await;
    }
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(DomainError::Validation("Endpoint Base URL is empty".into()));
    }

    let client = reqwest::Client::builder()
        .user_agent(PROBE_USER_AGENT)
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| DomainError::Validation(format!("Failed to build probe HTTP client: {e}")))?;

    // Explicit Ollama discovery must preserve empty lists and transport failures.
    if provider_type == "ollama" || base.contains(":11434") {
        let root = ollama_root(base)?;
        return probe_ollama_endpoint(&client, &root).await;
    }
    if provider_type == "local" {
        let root = ollama_root(base)?;
        if let Ok(models) = probe_ollama_endpoint(&client, &root).await {
            return Ok(models);
        }
    }

    // 2. Route by provider type or fallback to OpenAI compatible
    match provider_type {
        "anthropic" => probe_anthropic_endpoint(&client, base, api_key, false).await,
        "anthropic-oauth" => probe_anthropic_endpoint(&client, base, api_key, true).await,
        "gemini" | "google" => probe_gemini_endpoint(&client, base, api_key).await,
        _ => {
            probe_openai_endpoint(
                &client,
                base,
                api_key,
                if provider_type == "openai" {
                    None
                } else {
                    Some("/chat/completions")
                },
                false,
            )
            .await
        }
    }
}

/// OpenAI-compatible: GET /models or /v1/models
async fn probe_openai_endpoint(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
    required_endpoint: Option<&str>,
    copilot_policy: bool,
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

    parse_provider_models(&body, required_endpoint, copilot_policy)
}

pub fn parse_openai_models(body: &str) -> Result<Vec<ProbedModel>, DomainError> {
    parse_openai_models_for_transport(body, None)
}

pub fn parse_openai_models_for_transport(
    body: &str,
    required_endpoint: Option<&str>,
) -> Result<Vec<ProbedModel>, DomainError> {
    parse_provider_models(body, required_endpoint, false)
}

pub(crate) fn parse_provider_models(
    body: &str,
    required_endpoint: Option<&str>,
    copilot_policy: bool,
) -> Result<Vec<ProbedModel>, DomainError> {
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
            // Copilot's picker flag is presentation metadata, not subscription policy.
            let mut normalized;
            let m = if copilot_policy {
                let policy = m.pointer("/policy/state").and_then(Value::as_str);
                if policy.is_some_and(|state| state != "enabled")
                    || m.pointer("/capabilities/type").and_then(Value::as_str) != Some("chat")
                {
                    return None;
                }
                if policy != Some("enabled")
                    && m.get("model_picker_enabled").and_then(Value::as_bool) != Some(true)
                {
                    return None;
                }
                normalized = m.clone();
                if let Some(object) = normalized.as_object_mut() {
                    object.remove("model_picker_enabled");
                }
                &normalized
            } else {
                m
            };
            if !model_is_selectable(m) {
                return None;
            }
            let id = m.get("id")?.as_str()?.to_string();
            let endpoint = required_endpoint.unwrap_or_else(|| {
                if custos_provider::model::is_openai_responses_model(&id) {
                    "/responses"
                } else {
                    "/chat/completions"
                }
            });
            if m.get("supported_endpoints")
                .and_then(Value::as_array)
                .is_some_and(|endpoints| {
                    !endpoints
                        .iter()
                        .any(|value| value.as_str() == Some(endpoint))
                })
            {
                return None;
            }
            let name = m
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            let owned_by = m
                .get("owned_by")
                .and_then(|o| o.as_str())
                .map(|s| s.to_string());
            let description = m
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let context = [
                "context_length",
                "max_model_len",
                "max_context_length",
                "context_window",
            ]
            .iter()
            .find_map(|k| m.get(*k).and_then(|c| c.as_u64()))
            .or_else(|| {
                m.pointer("/capabilities/limits/max_context_window_tokens")
                    .and_then(Value::as_u64)
            });

            Some(ProbedModel {
                id,
                name,
                context_window: context,
                owned_by,
                default_effort: advertised_default_effort(m),
                efforts: advertised_efforts(m),
                description,
            })
        })
        .collect();

    Ok(models)
}

pub async fn probe_oauth_models(token: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| DomainError::Validation(format!("Failed to build OAuth probe client: {e}")))?;
    probe_chatgpt_codex_endpoint(&client, token).await
}

/// Probes the ChatGPT Codex backend models endpoint for OAuth users:
/// GET https://chatgpt.com/backend-api/codex/models?client_version=0.162.0
pub async fn probe_chatgpt_codex_endpoint(
    client: &reqwest::Client,
    token: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = format!(
        "https://chatgpt.com/backend-api/codex/models?client_version={CODEX_CLIENT_VERSION}"
    );
    let mut req = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("User-Agent", codex_user_agent())
        .header("Originator", "codex_cli_rs");

    if let Some(account_id) = crate::cloud::transport::extract_chatgpt_account_id(token) {
        req = req.header("ChatGPT-Account-ID", account_id);
    }

    let resp = req.send().await.map_err(|e| {
        DomainError::Validation(format!("Failed to reach Codex models at {url}: {e}"))
    })?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Codex models endpoint returned HTTP status: {}",
            resp.status()
        )));
    }

    let body = resp.text().await.map_err(|e| {
        DomainError::Validation(format!("Failed reading Codex models response: {e}"))
    })?;

    parse_chatgpt_codex_models(&body)
}

pub fn parse_chatgpt_codex_models(body: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let v: Value = serde_json::from_str(body).map_err(|e| {
        DomainError::Validation(format!("Codex models response was not valid JSON: {e}"))
    })?;

    let list = v
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| DomainError::Validation("Codex response has no models array".into()))?;

    let models: Vec<ProbedModel> = list
        .iter()
        .filter_map(|m| {
            if m.get("visibility").and_then(Value::as_str) != Some("list")
                || !model_is_selectable(m)
            {
                return None;
            }
            let id = m.get("slug")?.as_str()?.to_string();
            let name = m
                .get("display_name")
                .or_else(|| m.get("title"))
                .or_else(|| m.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            let description = m
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let context_window = m
                .get("context_window")
                .or_else(|| m.get("max_tokens"))
                .and_then(|t| t.as_u64());
            Some(ProbedModel {
                id: id.clone(),
                name: name.or(Some(id)),
                context_window,
                owned_by: Some("openai-chatgpt".into()),
                default_effort: advertised_default_effort(m),
                efforts: advertised_efforts(m),
                description,
            })
        })
        .collect();

    Ok(models)
}

/// Anthropic: GET /models with x-api-key and anthropic-version
async fn probe_anthropic_endpoint(
    client: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
    oauth: bool,
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
            if oauth {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", key.trim()))
                        .map_err(|e| DomainError::Validation(e.to_string()))?,
                );
                headers.insert(
                    "anthropic-beta",
                    HeaderValue::from_static("oauth-2025-04-20"),
                );
            } else {
                headers.insert("x-api-key", val);
            }
        }
    }

    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| {
            DomainError::Validation(format!("Failed to reach Anthropic models at {url}: {e}"))
        })?;

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

    parse_openai_models_for_transport(&body, Some("/messages"))
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

    let resp = client.get(&url).send().await.map_err(|e| {
        DomainError::Validation(format!("Failed to reach Gemini models at {url}: {e}"))
    })?;

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
            if !model_is_selectable(m)
                || m.get("supportedGenerationMethods")
                    .and_then(Value::as_array)
                    .is_some_and(|methods| {
                        !methods
                            .iter()
                            .any(|v| v.as_str() == Some("generateContent"))
                    })
            {
                return None;
            }
            let full_name = m.get("name")?.as_str()?;
            // Format is "models/gemini-2.5-pro", strip prefix
            let id = full_name
                .strip_prefix("models/")
                .unwrap_or(full_name)
                .to_string();
            let display_name = m
                .get("displayName")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let description = m
                .get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let context = m.get("inputTokenLimit").and_then(|c| c.as_u64());

            Some(ProbedModel {
                id,
                name: display_name,
                context_window: context,
                owned_by: Some("google".into()),
                default_effort: advertised_default_effort(m),
                efforts: advertised_efforts(m),
                description,
            })
        })
        .collect();

    Ok(probed)
}

pub async fn probe_copilot_models(github_token: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let token = crate::cloud::transport::copilot_session_token(github_token).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("GitHubCopilotChat/0.35.0")
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert("Editor-Version", HeaderValue::from_static("vscode/1.107.0"));
            headers.insert(
                "Editor-Plugin-Version",
                HeaderValue::from_static("copilot-chat/0.35.0"),
            );
            headers.insert(
                "Copilot-Integration-Id",
                HeaderValue::from_static("vscode-chat"),
            );
            headers
        })
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    let mut models = probe_openai_endpoint(
        &client,
        "https://api.githubcopilot.com",
        Some(&token),
        Some("/chat/completions"),
        true,
    )
    .await?;
    for model in &mut models {
        model.id = format!("copilot/{}", model.id);
        model.owned_by = Some("copilot".into());
    }
    Ok(models)
}
