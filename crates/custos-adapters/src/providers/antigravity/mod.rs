//! AntigravityProvider Adapter

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};

pub struct AntigravityProvider {
    provider_id: String,
}

impl AntigravityProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "google-antigravity".to_string(),
        }
    }
}

impl Default for AntigravityProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for AntigravityProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, _req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Err(DomainError::Validation(
            "Antigravity transport is not configured; this placeholder cannot perform inference"
                .into(),
        ))
    }
}

async fn cloud_call(
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

async fn cloud_call_with_fallback(
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

pub async fn discover_project(
    service: &str,
    base: &str,
    token: &str,
) -> Result<String, DomainError> {
    // Antigravity's bootstrap endpoint expects the native minimal metadata object.
    let metadata = if service == "gemini" {
        serde_json::json!({"ideType":"GEMINI_CLI","platform":"WINDOWS_AMD64","pluginType":"GEMINI"})
    } else {
        serde_json::json!({"ideType":"ANTIGRAVITY"})
    };
    let mut value = cloud_call(
        service,
        base,
        token,
        "loadCodeAssist",
        serde_json::json!({"metadata":metadata}),
    )
    .await?;
    let project = |v: &serde_json::Value| {
        v.get("cloudaicompanionProject")
            .and_then(|v| v.as_str().or_else(|| v.get("id").and_then(|v| v.as_str())))
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    if let Some(id) = project(&value) {
        return Ok(id);
    }
    if service != "antigravity" {
        return Err(DomainError::Validation(
            "Gemini account did not return a managed project".into(),
        ));
    }
    let tier = value
        .pointer("/paidTier/id")
        .or_else(|| value.pointer("/currentTier/id"))
        .or_else(|| {
            value
                .get("allowedTiers")
                .and_then(|v| v.as_array())
                .and_then(|tiers| {
                    tiers
                        .iter()
                        .find(|v| v.get("isDefault").and_then(|v| v.as_bool()) == Some(true))
                })
                .and_then(|v| v.get("id"))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("legacy-tier")
        .to_string();
    tracing::debug!(
        provider_id = "antigravity",
        "Resolving Antigravity IDE managed project through native onboarding"
    );
    let onboard = cloud_call(
        service,
        base,
        token,
        "onboardUser",
        serde_json::json!({"tier_id":tier,"metadata":metadata}),
    )
    .await?;
    if let Some(id) = project(&onboard).or_else(|| onboard.get("response").and_then(project)) {
        return Ok(id);
    }
    // An accepted onboarding can precede asynchronous assignment; bound discovery retries.
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        value = cloud_call(
            service,
            base,
            token,
            "loadCodeAssist",
            serde_json::json!({"metadata":metadata}),
        )
        .await?;
        if let Some(id) = project(&value) {
            return Ok(id);
        }
    }
    Err(DomainError::Validation("Antigravity IDE backend did not assign a managed project after onboarding; retry the request later. No CLI installation is required".into()))
}

pub async fn probe_models(
    base: &str,
    token: &str,
) -> Result<Vec<custos_domain::ProbedModel>, DomainError> {
    probe_codeassist_models("antigravity", base, token).await
}

pub async fn probe_codeassist_models(
    service: &str,
    base: &str,
    token: &str,
) -> Result<Vec<custos_domain::ProbedModel>, DomainError> {
    let body = if service == "antigravity" {
        serde_json::json!({})
    } else {
        serde_json::json!({"project":discover_project(service, base, token).await?})
    };
    let value = cloud_call(service, base, token, "fetchAvailableModels", body).await?;
    let payload = value
        .get("models")
        .ok_or_else(|| DomainError::Validation("Invalid Antigravity model catalog".into()))?;
    let models: Vec<(String, &serde_json::Value)> = if let Some(map) = payload.as_object() {
        map.iter().map(|(id, model)| (id.clone(), model)).collect()
    } else if let Some(array) = payload.as_array() {
        array
            .iter()
            .filter_map(|model| {
                model
                    .get("id")
                    .or_else(|| model.get("name"))
                    .or_else(|| model.get("model"))
                    .and_then(|v| v.as_str())
                    .filter(|id| !id.is_empty())
                    .map(|id| (id.to_string(), model))
            })
            .collect()
    } else {
        return Err(DomainError::Validation(
            "Invalid Antigravity model catalog".into(),
        ));
    };
    Ok(models
        .into_iter()
        .filter(|(_, model)| model.get("isInternal").and_then(|v| v.as_bool()) != Some(true))
        .map(|(id, model)| custos_domain::ProbedModel {
            id: format!(
                "{}/{id}",
                if service == "gemini" {
                    "gemini-oauth"
                } else {
                    "antigravity"
                }
            ),
            name: model
                .get("displayName")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            context_window: model.get("maxInputTokens").and_then(|v| v.as_u64()),
            owned_by: Some(service.into()),
            description: None,
        })
        .collect())
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
    let project = discover_project(service, base, token).await?;
    let model = req
        .model
        .strip_prefix(if service == "gemini" {
            "gemini-oauth/"
        } else {
            "antigravity/"
        })
        .ok_or_else(|| DomainError::Validation("Antigravity requires a discovered model".into()))?;
    let value = cloud_call(service, base, token, "generateContent", serde_json::json!({"project":project,"model":model,"userAgent":"antigravity","requestType":"agent","requestId":req.request_id,"request":{"contents":[{"role":"user","parts":[{"text":req.prompt}]}],"generationConfig":{"temperature":req.temperature,"maxOutputTokens":req.max_tokens}}})).await?;
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
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn project(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v1internal:loadCodeAssist"))
            .and(header("authorization", "Bearer oauth-token"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({"metadata":{"ideType":"ANTIGRAVITY"}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"cloudaicompanionProject":{"id":"account-project"}}),
            ))
            .mount(server)
            .await;
    }
    #[tokio::test]
    async fn bootstrap_metadata_and_error_detail_match_provider_contract() {
        let server = MockServer::start().await;
        Mock::given(path("/v1internal:loadCodeAssist"))
            .and(wiremock::matchers::body_json(serde_json::json!({"metadata":{"ideType":"GEMINI_CLI","platform":"WINDOWS_AMD64","pluginType":"GEMINI"}})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"cloudaicompanionProject":"gemini-project"})))
            .expect(1).mount(&server).await;
        assert_eq!(
            discover_project("gemini", &server.uri(), "oauth-token")
                .await
                .unwrap(),
            "gemini-project"
        );
        server.reset().await;
        Mock::given(path("/v1internal:loadCodeAssist"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error":{"message":"Invalid enum value WINDOWS oauth-token"}}),
            ))
            .mount(&server)
            .await;
        let error = discover_project("antigravity", &server.uri(), "oauth-token")
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("Invalid enum value WINDOWS"));
        assert!(!error.contains("oauth-token"));
        assert!(!error.contains("reconnect"));
    }

    #[tokio::test]
    async fn ide_project_onboarding_recovers_missing_initial_assignment() {
        let server = MockServer::start().await;
        Mock::given(path("/v1internal:loadCodeAssist"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"currentTier":{"id":"paid-tier"}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/v1internal:onboardUser"))
            .and(wiremock::matchers::body_json(serde_json::json!({"tier_id":"paid-tier","metadata":{"ideType":"ANTIGRAVITY"}})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"done":true,"response":{"cloudaicompanionProject":{"id":"ide-managed-project"}}})))
            .expect(1).mount(&server).await;
        assert_eq!(
            discover_project("antigravity", &server.uri(), "oauth-token")
                .await
                .unwrap(),
            "ide-managed-project"
        );
    }

    #[tokio::test]
    async fn ide_catalog_without_project_supports_array_and_hides_internal_models() {
        let server = MockServer::start().await;
        Mock::given(path("/v1internal:fetchAvailableModels"))
            .and(wiremock::matchers::body_json(serde_json::json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"models":[{"id":"gemini-flash","displayName":"Flash"},{"id":"internal-model","isInternal":true}]})))
            .expect(1).mount(&server).await;
        let models = probe_models(&server.uri(), "oauth-token").await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "antigravity/gemini-flash");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn generic_runtime_429_can_use_alternate_endpoint_without_changing_request() {
        let primary = MockServer::start().await;
        let alternate = MockServer::start().await;
        let body = serde_json::json!({"project":"same-project","model":"gemini-3.8-flash-tiered","request":{"contents":[{"role":"user","parts":[{"text":"same prompt"}]}]}});
        Mock::given(path("/v1internal:generateContent"))
            .and(wiremock::matchers::body_json(body.clone()))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({"error":{"status":"RESOURCE_EXHAUSTED","message":"Resource has been exhausted"}})))
            .expect(1).mount(&primary).await;
        Mock::given(path("/v1internal:generateContent"))
            .and(header("authorization", "Bearer oauth-token"))
            .and(wiremock::matchers::body_json(body.clone()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"response":{"candidates":[]}})),
            )
            .expect(1)
            .mount(&alternate)
            .await;
        assert!(cloud_call_with_fallback(
            "antigravity",
            &primary.uri(),
            "oauth-token",
            "generateContent",
            body,
            Some(&alternate.uri())
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn explicit_quota_retry_delay_does_not_try_alternate_endpoint() {
        let primary = MockServer::start().await;
        let alternate = MockServer::start().await;
        Mock::given(path("/v1internal:generateContent"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({"error":{"status":"RESOURCE_EXHAUSTED","message":"Quota exhausted","details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"60s"}]}})))
            .expect(1).mount(&primary).await;
        let error = cloud_call_with_fallback(
            "antigravity",
            &primary.uri(),
            "oauth-token",
            "generateContent",
            serde_json::json!({}),
            Some(&alternate.uri()),
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("60s"));
        assert!(alternate.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn antigravity_catalog_is_live_and_namespaced() {
        let server = MockServer::start().await;
        Mock::given(path("/v1internal:fetchAvailableModels")).and(wiremock::matchers::body_json(serde_json::json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"models":{"claude-sonnet":{"displayName":"Sonnet"},"gemini-flash":{}}}))).expect(1).mount(&server).await;
        let models = probe_models(&server.uri(), "oauth-token").await.unwrap();
        assert_eq!(models.len(), 2);
        assert!(models.iter().all(|m| m.id.starts_with("antigravity/")));
        assert_eq!(
            super::super::router::RouterModelProvider::classify_provider(&models[0].id),
            "antigravity"
        );
    }
    #[tokio::test]
    async fn antigravity_prompt_and_usage_use_cloudcode_envelope() {
        let server = MockServer::start().await;
        project(&server).await;
        Mock::given(path("/v1internal:generateContent")).and(header("authorization", "Bearer oauth-token"))
            .and(body_partial_json(serde_json::json!({"project":"account-project","model":"gemini-flash","request":{"contents":[{"role":"user","parts":[{"text":"actual conversation prompt"}]}]}})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"response":{"candidates":[{"content":{"parts":[{"thought":true,"text":"private reasoning"},{"text":"answer"}]}}],"usageMetadata":{"totalTokenCount":7}}}))).expect(1).mount(&server).await;
        let mut req = ModelRequest::simple("actual conversation prompt");
        req.model = "antigravity/gemini-flash".into();
        let answer = generate_content(&server.uri(), "oauth-token", &req)
            .await
            .unwrap();
        assert_eq!(answer.content, "answer");
        assert_eq!(answer.tokens_used, 7);
    }
    #[tokio::test]
    async fn antigravity_forbidden_and_unprovisioned_accounts_fail_closed() {
        let server = MockServer::start().await;
        Mock::given(path("/v1internal:loadCodeAssist"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        assert!(
            discover_project("antigravity", &server.uri(), "oauth-token")
                .await
                .unwrap_err()
                .to_string()
                .contains("403")
        );
        server.reset().await;
        Mock::given(path("/v1internal:loadCodeAssist"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        assert!(
            discover_project("antigravity", &server.uri(), "oauth-token")
                .await
                .unwrap_err()
                .to_string()
                .contains("onboardUser")
        );
    }
}
