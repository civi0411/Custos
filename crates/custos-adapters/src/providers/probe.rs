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
    if provider_type == "antigravity" {
        return super::antigravity::probe_models(base_url, api_key.ok_or_else(|| DomainError::Validation("Antigravity OAuth credentials missing".into()))?).await;
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
        _ => probe_openai_endpoint(&client, base, api_key).await,
    }
}

/// Normalize an Ollama root without duplicating compatibility API suffixes.
pub(crate) fn ollama_root(base: &str) -> Result<String, DomainError> {
    let root = base.trim().trim_end_matches('/').trim_end_matches("/v1");
    let url = reqwest::Url::parse(root)
        .map_err(|e| DomainError::Validation(format!("Invalid Ollama URL: {e}")))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DomainError::Validation(
            "Ollama URL must be an HTTP(S) base URL without query or fragment".into(),
        ));
    }
    Ok(root.to_string())
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
            id: format!("ollama/{name}"),
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

pub async fn probe_oauth_models(token: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build().map_err(|e| DomainError::Validation(format!("Failed to build OAuth probe client: {e}")))?;
    probe_chatgpt_codex_endpoint(&client, token).await
}

/// Probes the ChatGPT Codex backend models endpoint for OAuth users:
/// GET https://chatgpt.com/backend-api/codex/models?client_version=0.144.0
pub async fn probe_chatgpt_codex_endpoint(
    client: &reqwest::Client,
    token: &str,
) -> Result<Vec<ProbedModel>, DomainError> {
    let url = "https://chatgpt.com/backend-api/codex/models?client_version=0.144.0";
    let mut req = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("User-Agent", "codex-cli/0.144.0")
        .header("Originator", "codex_cli_rs");

    if let Some(account_id) = crate::providers::oauth_pkce::extract_chatgpt_account_id(token) {
        req = req.header("ChatGPT-Account-ID", account_id);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed to reach Codex models at {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(DomainError::Validation(format!(
            "Codex models endpoint returned HTTP status: {}",
            resp.status()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| DomainError::Validation(format!("Failed reading Codex models response: {e}")))?;

    parse_chatgpt_codex_models(&body)
}

pub fn parse_chatgpt_codex_models(body: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| DomainError::Validation(format!("Codex models response was not valid JSON: {e}")))?;

    let list = v
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| DomainError::Validation("Codex response has no models array".into()))?;

    let models: Vec<ProbedModel> = list
        .iter()
        .filter_map(|m| {
            let id = m.get("slug")?.as_str()?.to_string();
            let name = m
                .get("title")
                .or_else(|| m.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            let description = m.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
            let context_window = m.get("context_window").or_else(|| m.get("max_tokens")).and_then(|t| t.as_u64());
            Some(ProbedModel {
                id: id.clone(),
                name: name.or(Some(id)),
                context_window,
                owned_by: Some("openai-chatgpt".into()),
                description,
            })
        })
        .collect();

    if models.is_empty() {
        return Err(DomainError::Validation("The Codex endpoint listed zero models".into()));
    }

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
                headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", key.trim())).map_err(|e| DomainError::Validation(e.to_string()))?);
                headers.insert("anthropic-beta", HeaderValue::from_static("oauth-2025-04-20"));
            } else { headers.insert("x-api-key", val); }
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

    #[test]
    fn test_parse_chatgpt_codex_models() {
        let body = r#"{
            "models": [
                {
                    "slug": "gpt-5.6-sol",
                    "title": "GPT-5.6 Sol",
                    "description": "Flagship Codex model",
                    "max_tokens": 200000,
                    "visibility": "list"
                },
                {
                    "slug": "gpt-5.6-terra",
                    "title": null,
                    "description": null,
                    "max_tokens": null,
                    "visibility": "list"
                }
            ]
        }"#;
        let models = parse_chatgpt_codex_models(body).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-5.6-sol");
        assert_eq!(models[0].name, Some("GPT-5.6 Sol".into()));
        assert_eq!(models[0].context_window, Some(200000));
        assert_eq!(models[0].owned_by, Some("openai-chatgpt".into()));
        assert_eq!(models[1].id, "gpt-5.6-terra");
        assert_eq!(models[1].name, Some("gpt-5.6-terra".into()));
        assert_eq!(models[1].context_window, None);
    }
}

pub async fn probe_copilot_models(github_token: &str) -> Result<Vec<ProbedModel>, DomainError> {
    let token = super::oauth_pkce::copilot_session_token(github_token).await?;
    let client = reqwest::Client::builder().timeout(Duration::from_secs(20)).user_agent("GitHubCopilotChat/0.35.0").default_headers({
        let mut headers = HeaderMap::new();
        headers.insert("Editor-Version", HeaderValue::from_static("vscode/1.107.0"));
        headers.insert("Editor-Plugin-Version", HeaderValue::from_static("copilot-chat/0.35.0"));
        headers.insert("Copilot-Integration-Id", HeaderValue::from_static("vscode-chat")); headers
    }).build().map_err(|e| DomainError::Validation(e.to_string()))?;
    let mut models = probe_openai_endpoint(&client, "https://api.githubcopilot.com", Some(&token)).await?;
    for model in &mut models { model.id = format!("copilot/{}", model.id); model.owned_by = Some("copilot".into()); }
    Ok(models)
}

#[cfg(test)]
mod ollama_connection_tests {
    use super::*;
    #[test]
    fn roots_and_empty_models() {
        assert_eq!(
            ollama_root(" http://localhost:11434/v1/ ").unwrap(),
            "http://localhost:11434"
        );
        assert!(ollama_root("file:///models").is_err());
        assert!(ollama_root("http://localhost:11434?key=x").is_err());
        assert_eq!(parse_ollama_tags(r#"{"models":[]}"#), Some(vec![]));
    }
    #[tokio::test]
    async fn empty_native_discovery_does_not_fall_back() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = socket.read(&mut buf).unwrap();
            let body = r#"{"models":[]}"#;
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            assert!(String::from_utf8_lossy(&buf[..n]).starts_with("GET /api/tags "));
        });
        assert!(probe_endpoint_models(&base, None, "ollama")
            .await
            .unwrap()
            .is_empty());
        server.join().unwrap();
    }
}

/// Fetch account quota independently of observed inference usage.
pub async fn probe_account_quota(service: &str, token: &str) -> Result<Value, DomainError> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(8)).build().map_err(|e| DomainError::Validation(e.to_string()))?;
    // Quota reads must not onboard or create a project.
    if matches!(service, "antigravity" | "gemini") {
        let metadata = if service == "antigravity" { serde_json::json!({"ideType":"ANTIGRAVITY"}) } else { serde_json::json!({"ideType":"GEMINI_CLI","platform":"WINDOWS_AMD64","pluginType":"GEMINI"}) };
        if let Ok(response) = client.post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist").bearer_auth(token).json(&serde_json::json!({"metadata":metadata})).send().await {
            if response.status().is_success() {
                if let Ok(value) = response.json::<Value>().await {
                    if let Some(project) = value.get("cloudaicompanionProject").and_then(|v| v.as_str().or_else(||v.get("id").and_then(Value::as_str))) {
                        if let Ok(response) = client.post("https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota").bearer_auth(token).json(&serde_json::json!({"project":project})).send().await {
                            if response.status().is_success() {
                                if let Ok(value) = response.json::<Value>().await {
                                    let parsed = parse_account_quota("google-live", &value);
                                    if parsed["status"] == "available" { return Ok(parsed); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let request = match service {
        "openai" => {
            let mut request = client.get("https://chatgpt.com/backend-api/wham/usage").bearer_auth(token);
            if let Some(id) = super::oauth_pkce::extract_chatgpt_account_id(token) { request = request.header("ChatGPT-Account-ID", id); }
            request
        }
        "anthropic" => client.get("https://api.anthropic.com/api/oauth/usage").bearer_auth(token).header("anthropic-beta", "oauth-2025-04-20"),
        "copilot" => client.get("https://api.github.com/copilot_internal/user").bearer_auth(token).header("User-Agent", "Custos").header("Accept", "application/json"),
        "antigravity" | "gemini" => client.post("https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels").bearer_auth(token).header("User-Agent", "antigravity/1.15.8 windows/amd64").json(&serde_json::json!({})),
        _ => return Ok(serde_json::json!({"status":"unsupported", "windows":[]})),
    };
    let response = request.send().await.map_err(|_| DomainError::Validation("Quota service unavailable".into()))?;
    if !response.status().is_success() { return Err(DomainError::Validation(format!("Quota service returned {}", response.status()))); }
    let body: Value = response.json().await.map_err(|_| DomainError::Validation("Invalid quota response".into()))?;
    Ok(parse_account_quota(service, &body))
}

pub fn parse_account_quota(service: &str, body: &Value) -> Value {
    let mut windows = Vec::new();
    let mut push = |name: &str, used: Option<f64>, reset: Value| {
        if let Some(used) = used.filter(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0) {
            windows.push(serde_json::json!({"name":name,"used_percent":used,"resets_at":reset}));
        }
    };
    match service {
        "openai" => for (name, key) in [("Session", "primary_window"), ("Weekly", "secondary_window")] {
            if let Some(window) = body.pointer(&format!("/rate_limit/{key}")) { push(name, window.get("used_percent").and_then(Value::as_f64), window.get("reset_at").cloned().unwrap_or(Value::Null)); }
        },
        "anthropic" => for key in ["five_hour", "seven_day", "seven_day_sonnet", "seven_day_opus"] {
            if let Some(window) = body.get(key) { push(key, window.get("utilization").and_then(Value::as_f64), window.get("resets_at").cloned().unwrap_or(Value::Null)); }
        },
        "copilot" => if let Some(snapshots) = body.get("quota_snapshots").and_then(Value::as_object) {
            for (key, window) in snapshots {
                if window.get("unlimited").and_then(Value::as_bool) != Some(true) { push(key, window.get("percent_remaining").and_then(Value::as_f64).map(|v| 100.0 - v), body.get("quota_reset_date").cloned().unwrap_or(Value::Null)); }
            }
        },
        "google-live" => if let Some(buckets) = body.get("buckets").and_then(Value::as_array) {
            for bucket in buckets { if let Some(name) = bucket.get("modelId").and_then(Value::as_str) { push(name, bucket.get("remainingFraction").and_then(Value::as_f64).map(|v| (1.0-v)*100.0), bucket.get("resetTime").cloned().unwrap_or(Value::Null)); } }
        },
        "antigravity" | "gemini" => if let Some(models) = body.get("models").and_then(Value::as_object) {
            for (key, model) in models { if let Some(quota) = model.get("quotaInfo") { push(key, quota.get("remainingFraction").and_then(Value::as_f64).map(|v| (1.0-v)*100.0), quota.get("resetTime").cloned().unwrap_or(Value::Null)); } }
        },
        _ => {}
    }
    serde_json::json!({"status": if windows.is_empty() {"unavailable"} else {"available"}, "source": if matches!(service, "antigravity" | "gemini") {"provider_catalog"} else {"provider"}, "windows": windows})
}

#[cfg(test)]
mod quota_monitor_tests {
    use super::*;
    #[test]
    fn missing_quota_is_unknown_and_windows_remain_distinct() {
        let empty = parse_account_quota("openai", &serde_json::json!({"rate_limit":{}}));
        assert_eq!(empty["status"], "unavailable");
        let parsed = parse_account_quota("openai", &serde_json::json!({"rate_limit":{"primary_window":{"used_percent":0,"reset_at":123},"secondary_window":{"used_percent":83}}}));
        assert_eq!(parsed["windows"].as_array().unwrap().len(),2);
        assert_eq!(parsed["windows"][0]["used_percent"],0.0);
        assert!(parsed["windows"][1]["resets_at"].is_null());
    }
    #[test]
    fn provider_quota_units_are_normalized_without_inventing_limits() {
        let google = parse_account_quota("google-live", &serde_json::json!({"buckets":[{"modelId":"gemini-flash","remainingFraction":0.25,"resetTime":"2026-10-12T00:00:00Z"},{"modelId":"unknown"}]}));
        assert_eq!(google["windows"].as_array().unwrap().len(),1);
        assert_eq!(google["windows"][0]["used_percent"],75.0);
        assert_eq!(google["source"],"provider");
        let claude = parse_account_quota("anthropic", &serde_json::json!({"five_hour":{"utilization":50},"seven_day":{"utilization":999}}));
        assert_eq!(claude["windows"].as_array().unwrap().len(),1);
        let copilot = parse_account_quota("copilot", &serde_json::json!({"quota_snapshots":{"premium_interactions":{"percent_remaining":20}}}));
        assert_eq!(copilot["windows"][0]["used_percent"],80.0);
    }
}
