use custos_domain::DomainError;

pub(crate) async fn cloud_call(
    service: &str,
    base: &str,
    token: &str,
    method: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, DomainError> {
    let fallback = if service == "antigravity"
        && method == "generateContent"
        && base.trim_end_matches('/') == "https://cloudcode-pa.googleapis.com"
    {
        Some("https://daily-cloudcode-pa.googleapis.com")
    } else {
        None
    };
    cloud_call_with_fallback(service, base, token, method, body, fallback).await
}

pub(crate) async fn cloud_call_with_fallback(
    service: &str,
    base: &str,
    token: &str,
    method: &str,
    body: serde_json::Value,
    fallback: Option<&str>,
) -> Result<serde_json::Value, DomainError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| DomainError::Validation(e.to_string()))?;
    for (attempt, endpoint) in std::iter::once(base).chain(fallback).enumerate() {
        let response = client
            .post(format!(
                "{}/v1internal:{method}",
                endpoint.trim_end_matches('/')
            ))
            .bearer_auth(token)
            .header(
                "User-Agent",
                if service == "gemini" || method == "loadCodeAssist" {
                    "google-api-nodejs-client/9.15.1"
                } else {
                    "antigravity/1.18.3 windows/amd64"
                },
            )
            .header(
                "X-Goog-Api-Client",
                "google-cloud-sdk vscode_cloudshelleditor/0.1",
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Validation(format!("Antigravity {method}: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let error = response.json::<serde_json::Value>().await.ok();
            let details = error
                .as_ref()
                .and_then(|v| v.pointer("/error/details"))
                .and_then(|v| v.as_array());
            let quota_limited = details.is_some_and(|values| {
                values.iter().any(|v| {
                    v.get("@type")
                        .and_then(|v| v.as_str())
                        .is_some_and(|v| v.ends_with("QuotaFailure") || v.ends_with("RetryInfo"))
                        || v.get("reason")
                            .and_then(|v| v.as_str())
                            .is_some_and(|reason| {
                                reason.contains("QUOTA") || reason.contains("RATE_LIMIT")
                            })
                })
            });
            let exhausted = error
                .as_ref()
                .and_then(|v| v.pointer("/error/status"))
                .and_then(|v| v.as_str())
                == Some("RESOURCE_EXHAUSTED");
            if attempt == 0
                && fallback.is_some()
                && status == reqwest::StatusCode::TOO_MANY_REQUESTS
                && exhausted
                && !quota_limited
                && retry_after.is_none()
            {
                tracing::debug!(provider_id = service, method, "Production runtime returned generic RESOURCE_EXHAUSTED; trying the alternate Google runtime once");
                continue;
            }
            let delay = retry_after.or_else(|| {
                details.and_then(|values| {
                    values.iter().find_map(|v| {
                        v.get("retryDelay")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
                })
            });
            let wait = delay
                .map(|v| {
                    format!(
                        "; provider retry delay: {}",
                        v.chars().take(80).collect::<String>()
                    )
                })
                .unwrap_or_default();
            let detail = error
                .as_ref()
                .and_then(|v| v.pointer("/error/message"))
                .and_then(|v| v.as_str())
                .map(|v| {
                    v.replace(token, "[redacted]")
                        .chars()
                        .take(500)
                        .collect::<String>()
                })
                .unwrap_or_else(|| {
                    "Provider rejected the request; no structured error message returned".into()
                });
            return Err(DomainError::Validation(format!(
                "{service} {method} returned {status}: {detail}{wait}"
            )));
        }
        return response
            .json()
            .await
            .map_err(|e| DomainError::Validation(format!("Antigravity response: {e}")));
    }
    Err(DomainError::Validation(
        "Antigravity runtime endpoints did not accept the request".into(),
    ))
}
