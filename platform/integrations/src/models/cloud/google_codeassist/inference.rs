use custos_domain::DomainError;
use custos_provider::{ModelRequest, ModelResponse};

use super::models::supports_tiered_reasoning;
use super::project::discover_project;
use super::transport::cloud_call;

pub(crate) fn codeassist_payload(
    service: &str,
    project: &str,
    req: &ModelRequest,
) -> Result<serde_json::Value, DomainError> {
    let model = req
        .model
        .strip_prefix(if service == "gemini" {
            "gemini-oauth/"
        } else {
            "antigravity/"
        })
        .ok_or_else(|| DomainError::Validation("Select a discovered Google model".into()))?;
    let mut generation =
        serde_json::json!({"temperature":req.temperature,"maxOutputTokens":req.max_tokens});
    if let Some(effort) = req.reasoning_effort.as_deref() {
        if service != "antigravity"
            || !supports_tiered_reasoning(&req.model)
            || !matches!(effort, "low" | "medium" | "high")
        {
            return Err(DomainError::Validation(
                "Reasoning effort is not supported by this Google transport/model".into(),
            ));
        }
        generation["thinkingConfig"] = serde_json::json!({"thinkingLevel":effort});
    }
    Ok(
        serde_json::json!({"project":project,"model":model,"userAgent":"antigravity","requestType":"agent","requestId":req.request_id,"request":{"contents":[{"role":"user","parts":[{"text":req.prompt}]}],"generationConfig":generation}}),
    )
}

pub async fn stream_codeassist(
    service: &str,
    base: &str,
    token: &str,
    req: &ModelRequest,
) -> Result<reqwest::Response, DomainError> {
    tracing::debug!(task_id=%req.task_id,request_id=%req.request_id,"Starting Google stream");
    if req.tools.as_ref().is_some_and(|tools| !tools.is_empty()) {
        return Err(DomainError::Validation(
            "Google flat streaming does not support tool schemas".into(),
        ));
    }
    let mut body = codeassist_payload(service, "", req)?;
    let project = discover_project(service, base, token).await?;
    body["project"] = serde_json::json!(project);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    let fallback = if service == "antigravity"
        && base.trim_end_matches('/') == "https://cloudcode-pa.googleapis.com"
    {
        Some("https://daily-cloudcode-pa.googleapis.com")
    } else {
        None
    };
    for (attempt, endpoint) in std::iter::once(base).chain(fallback).enumerate() {
        let response = client
            .post(format!(
                "{}/v1internal:streamGenerateContent?alt=sse",
                endpoint.trim_end_matches('/')
            ))
            .bearer_auth(token)
            .header(
                "User-Agent",
                if service == "antigravity" {
                    "antigravity/1.18.3 windows/amd64"
                } else {
                    "google-api-nodejs-client/9.15.1"
                },
            )
            .header(
                "X-Goog-Api-Client",
                "google-cloud-sdk vscode_cloudshelleditor/0.1",
            )
            .json(&body)
            .send()
            .await
            .map_err(|_| DomainError::Validation("Google stream connection failed".into()))?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let retry_after = response.headers().get("retry-after").is_some();
        let error = response.json::<serde_json::Value>().await.ok();
        let details = error
            .as_ref()
            .and_then(|v| v.pointer("/error/details"))
            .and_then(|v| v.as_array());
        let quota_limited = details.is_some_and(|values| {
            values.iter().any(|v| {
                v["@type"]
                    .as_str()
                    .is_some_and(|s| s.ends_with("QuotaFailure") || s.ends_with("RetryInfo"))
                    || v["reason"]
                        .as_str()
                        .is_some_and(|s| s.contains("QUOTA") || s.contains("RATE_LIMIT"))
            })
        });
        if attempt == 0
            && fallback.is_some()
            && status == reqwest::StatusCode::TOO_MANY_REQUESTS
            && error
                .as_ref()
                .and_then(|v| v.pointer("/error/status"))
                .and_then(|v| v.as_str())
                == Some("RESOURCE_EXHAUSTED")
            && !quota_limited
            && !retry_after
        {
            continue;
        }
        let code = error
            .as_ref()
            .and_then(|value| value.pointer("/error/status"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let message = error
            .as_ref()
            .and_then(|value| value.pointer("/error/message"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .replace(token, "[redacted]")
            .chars()
            .take(500)
            .collect::<String>();
        return Err(DomainError::Validation(format!(
            "Google streaming returned {status}: {code} {message}"
        )));
    }
    Err(DomainError::Validation(
        "Google streaming runtime unavailable".into(),
    ))
}

pub async fn generate_content(
    base: &str,
    token: &str,
    req: &ModelRequest,
) -> Result<ModelResponse, DomainError> {
    generate_codeassist_content("antigravity", base, token, req).await
}

pub async fn generate_codeassist_content(
    service: &str,
    base: &str,
    token: &str,
    req: &ModelRequest,
) -> Result<ModelResponse, DomainError> {
    tracing::debug!(task_id = %req.task_id, run_id = %req.request_id, "Antigravity inference");
    if req.tools.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(DomainError::Validation(
            "Antigravity tool calls are not supported by this conversation transport".into(),
        ));
    }
    let mut body = codeassist_payload(service, "", req)?;
    let project = discover_project(service, base, token).await?;
    body["project"] = serde_json::json!(project);
    let value = cloud_call(service, base, token, "generateContent", body).await?;
    let response = value.get("response").unwrap_or(&value);
    let parts = response
        .pointer("/candidates/0/content/parts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| DomainError::Validation("Antigravity returned no text candidate".into()))?;
    let content: String = parts
        .iter()
        .filter(|p| p.get("thought").and_then(|v| v.as_bool()) != Some(true))
        .filter_map(|p| p.get("text").and_then(|v| v.as_str()))
        .collect();
    if content.is_empty() {
        return Err(DomainError::Validation(
            "Antigravity returned an empty or blocked answer".into(),
        ));
    }
    Ok(ModelResponse::text(
        content,
        &req.model,
        response
            .pointer("/usageMetadata/totalTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
    )
    .with_usage(
        response
            .pointer("/usageMetadata/promptTokenCount")
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok()),
        response
            .pointer("/usageMetadata/candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok()),
        response
            .pointer("/usageMetadata/totalTokenCount")
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok()),
    ))
}
