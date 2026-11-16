use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::inference::{codeassist_payload, generate_content, stream_codeassist};
use super::models::probe_models;
use super::project::discover_project;
use super::transport::cloud_call_with_fallback;
use custos_provider::ModelRequest;

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
async fn tiered_catalog_and_both_transports_preserve_exact_effort_and_id() {
    let server = MockServer::start().await;
    Mock::given(path("/v1internal:fetchAvailableModels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"models":{"gemini-3.8-flash-tiered":{},"gemini-other":{}}}),
        ))
        .mount(&server)
        .await;
    let models = probe_models(&server.uri(), "oauth-token").await.unwrap();
    let model = models
        .iter()
        .find(|model| model.id == "antigravity/gemini-3.8-flash-tiered")
        .unwrap();
    assert_eq!(model.efforts, vec!["low", "medium", "high"]);
    assert_eq!(model.default_effort.as_deref(), Some("medium"));
    assert!(models
        .iter()
        .find(|model| model.id == "antigravity/gemini-other")
        .unwrap()
        .efforts
        .is_empty());
    project(&server).await;
    for effort in ["low", "medium", "high"] {
        for streaming in [false, true] {
            let path_suffix = if streaming {
                "streamGenerateContent"
            } else {
                "generateContent"
            };
            Mock::given(path(format!("/v1internal:{path_suffix}")))
                .and(wiremock::matchers::body_partial_json(serde_json::json!({"model":"gemini-3.8-flash-tiered","request":{"generationConfig":{"thinkingConfig":{"thinkingLevel":effort}}}})))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"response":{"candidates":[{"content":{"parts":[{"text":"fixture answer"}]}}]}})))
                .expect(1).mount(&server).await;
            let mut request = ModelRequest::simple("Fixture prompt");
            request.model = model.id.clone();
            request.reasoning_effort = Some(effort.into());
            if streaming {
                assert!(stream_codeassist(
                    "antigravity",
                    &server.uri(),
                    "oauth-token",
                    &request
                )
                .await
                .unwrap()
                .status()
                .is_success());
            } else {
                assert_eq!(
                    generate_content(&server.uri(), "oauth-token", &request)
                        .await
                        .unwrap()
                        .content,
                    "fixture answer"
                );
            }
        }
    }
}

#[tokio::test]
async fn unsupported_effort_fails_before_project_or_inference_requests() {
    let server = MockServer::start().await;
    let mut request = ModelRequest::simple("Fixture prompt");
    for (model, effort) in [
        ("antigravity/gemini-3.8-flash-tiered", "ultra"),
        ("antigravity/claude-sonnet-4-6", "low"),
    ] {
        request.model = model.into();
        request.reasoning_effort = Some(effort.into());
        assert!(
            stream_codeassist("antigravity", &server.uri(), "fixture-token", &request)
                .await
                .is_err()
        );
        assert!(generate_content(&server.uri(), "fixture-token", &request)
            .await
            .is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    request.model = "antigravity/gemini-3.8-flash-tiered".into();
    request.reasoning_effort = None;
    assert!(codeassist_payload("antigravity", "project", &request)
        .unwrap()
        .pointer("/request/generationConfig/thinkingConfig")
        .is_none());
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"allowedTiers":[{"id":"tier-enterprise","isDefault":true}]})))
        .expect(1).mount(&server).await;
    Mock::given(path("/v1internal:onboardUser"))
        .and(wiremock::matchers::body_json(serde_json::json!({"tier_id":"tier-enterprise","metadata":{"ideType":"ANTIGRAVITY"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"cloudaicompanionProject":{"id":"onboarded-project"}})))
        .expect(1).mount(&server).await;
    assert_eq!(
        discover_project("antigravity", &server.uri(), "oauth-token")
            .await
            .unwrap(),
        "onboarded-project"
    );
}

#[tokio::test]
async fn production_endpoint_quota_fallback_recovers_transient_failures() {
    let primary = MockServer::start().await;
    let alternate = MockServer::start().await;
    let body = serde_json::json!({"prompt":[{"role":"user","parts":[{"text":"fixture prompt"}]}]});
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
        crate::routing::router::RouterModelProvider::classify_provider(&models[0].id),
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
