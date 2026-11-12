use super::super::*;
use super::helpers::TestCredentialResolver;
use std::sync::Arc;
use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ProviderRequest};

#[test]
fn codex_errors_keep_subscription_identity_and_reject_partial_streams() {
    let formatted = format_provider_error(
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        "insufficient_quota",
        "ChatGPT Codex",
    );
    assert!(!formatted.contains("platform.openai.com"));
    assert!(formatted.contains("ChatGPT Codex"));
    for terminal in ["error", "response.failed", "response.incomplete"] {
        let stream = format!("data: {{\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}}\n\ndata: {{\"type\":\"{terminal}\"}}\n\n");
        assert!(parse_codex_sse_stream(&stream).is_err());
    }
    assert!(parse_codex_sse_stream("data: {\"delta\":\"partial\"}\n").is_err());
}

#[test]
fn test_format_provider_error_credit_balance_exhausted() {
    let err_body = r#"{ "error": { "message": "You have no credits remaining.", "type": "insufficient_quota", "code": "credit_balance_exhausted" } }"#;
    let formatted =
        format_provider_error(reqwest::StatusCode::TOO_MANY_REQUESTS, err_body, "OpenAI");
    assert!(formatted.contains("credit_balance_exhausted"));
    assert!(formatted.contains("ChatGPT Plus subscription ($20/mo) only covers web/app ChatGPT"));
    assert!(formatted.contains("https://platform.openai.com/settings/organization/billing/"));
}

#[test]
fn test_format_provider_error_standard() {
    let err_body = r#"{"error": "invalid model"}"#;
    let formatted = format_provider_error(reqwest::StatusCode::BAD_REQUEST, err_body, "OpenAI");
    assert_eq!(
        formatted,
        "OpenAI API error 400 Bad Request: {\"error\": \"invalid model\"}"
    );
}

#[test]
fn test_classify_provider() {
    assert_eq!(
        RouterModelProvider::classify_provider("native:/models/coder.gguf"),
        "native-local"
    );
    assert_eq!(RouterModelProvider::classify_provider("gpt-4o"), "openai");
    assert_eq!(
        RouterModelProvider::classify_provider("gpt-4o-mini"),
        "openai"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("gpt-5.6-sol"),
        "openai"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("gpt-5.6-terra"),
        "openai"
    );
    assert_eq!(RouterModelProvider::classify_provider("o3-mini"), "openai");
    assert_eq!(RouterModelProvider::classify_provider("o1"), "openai");
    assert_eq!(
        RouterModelProvider::classify_provider("claude-3-7-sonnet"),
        "anthropic"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("claude-3-5-haiku"),
        "anthropic"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("gemini-2.5-flash"),
        "gemini"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("gemini-2.5-pro"),
        "gemini"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("deepseek-chat"),
        "deepseek"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("deepseek-reasoner"),
        "deepseek"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("llama3.3:70b"),
        "local"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("qwen2.5-coder:32b"),
        "local"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("groq/llama-3.3-70b"),
        "groq"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("openrouter/auto"),
        "openrouter"
    );
    assert_eq!(
        RouterModelProvider::classify_provider("together/deepseek-r1"),
        "together"
    );
}

struct RejectingNativeAccountResolver;

#[async_trait]
impl CredentialResolver for RejectingNativeAccountResolver {
    async fn select_account(
        &self,
        _service: &str,
        _model: &str,
    ) -> Result<Option<String>, DomainError> {
        Err(DomainError::Validation(
            "Account lookup must not run for native models".into(),
        ))
    }

    async fn get_token(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }

    async fn get_endpoint_url(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

#[tokio::test]
async fn native_model_skips_external_account_lookup() {
    let router = RouterModelProvider::new(Arc::new(RejectingNativeAccountResolver));
    let mut request = ProviderRequest::simple("hello");
    request.model = "native:/models/coder.gguf".into();
    let generate_error = router
        .generate(&request)
        .await
        .expect_err("native backend is absent");
    let stream_error = router
        .stream(&request)
        .await
        .expect_err("native backend is absent");
    assert!(generate_error
        .to_string()
        .contains("Native local inference runtime is not configured"));
    assert!(stream_error
        .to_string()
        .contains("Native local inference runtime is not configured"));
}

#[test]
fn test_parse_codex_sse_stream() {
    let stream = "event: response.output_text.delta\n\
                      data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\n\
                      event: response.output_text.delta\n\
                      data: {\"type\":\"response.output_text.delta\",\"delta\":\" world!\"}\n\n\
                      event: response.completed\n\
                      data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":18}}}\n\n";
    let (content, tokens) = parse_codex_sse_stream(stream).expect("valid parse");
    assert_eq!(content, "Hello world!");
    assert_eq!(tokens, 18);
}

#[test]
fn test_parse_codex_sse_stream_empty() {
    let stream = "event: ping\ndata: {}\n\n";
    let err = parse_codex_sse_stream(stream);
    assert!(err.is_err());
}

#[tokio::test]
async fn conversation_effort_reaches_codex_and_api_responses_without_aliasing() {
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    for oauth in [true, false] {
        let server = MockServer::start().await;
        let endpoint = if oauth {
            "/codex/responses"
        } else {
            "/responses"
        };
        for effort in ["xhigh", "max", "ultra"] {
            let body = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"answer\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"model\":\"gpt-6.1-sol\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n";
            let mut mock = Mock::given(method("POST"))
                .and(path(endpoint))
                .and(body_partial_json(
                    serde_json::json!({"model":"gpt-6.1-sol","reasoning":{"effort":effort}}),
                ));
            if oauth {
                mock = mock.and(header(
                    "user-agent",
                    crate::routing::probe::codex_user_agent(),
                ));
            }
            mock.respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(body)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        }
        let mut provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth,
            endpoint: server.uri(),
            fail: false,
        }));
        provider.codex_endpoint = format!("{}/codex/responses", server.uri());
        for effort in ["xhigh", "max", "ultra"] {
            let mut request = ProviderRequest::simple("hello");
            request.model = "gpt-6.1-sol".into();
            request.reasoning_effort = Some(effort.into());
            request.provider_account_id = Some("account-A".into());
            assert_eq!(provider.generate(&request).await.unwrap().content, "answer");
            let mut receiver = provider.stream(&request).await.unwrap();
            let mut completed = false;
            while let Some(event) = receiver.recv().await {
                if event.event_type == custos_provider::ProviderEventType::Completed {
                    completed = true;
                }
                assert_ne!(event.event_type, custos_provider::ProviderEventType::Failed);
            }
            assert!(completed);
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 6);
    }
    let payload =
        RouterModelProvider::responses_payload(&ProviderRequest::simple("legacy"), "gpt-6-sol");
    assert!(payload.get("reasoning").is_none());
}

struct ObservingResolver {
    endpoint: String,
    state: crate::routing::availability::ModelAvailability,
}
#[async_trait]
impl CredentialResolver for ObservingResolver {
    async fn get_token(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some("fixture".into()))
    }
    async fn get_endpoint_url(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(self.endpoint.clone()))
    }
    async fn service_for_account(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some("openai".into()))
    }
    async fn validate_model(
        &self,
        account: &str,
        model: &str,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        self.state.check(account, model)
    }
    async fn observe_response(
        &self,
        account: &str,
        model: &str,
        status: u16,
        body: &str,
        retry_at: Option<i64>,
    ) -> Result<(), DomainError> {
        self.state.observe(account, model, status, body, retry_at)
    }
}
#[tokio::test]
async fn rejected_http_and_stream_frames_update_only_the_selected_account() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(serde_json::json!({"error":{"code":"insufficient_quota"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let state = crate::routing::availability::ModelAvailability::default();
    let provider = RouterModelProvider::new(Arc::new(ObservingResolver {
        endpoint: server.uri(),
        state: state.clone(),
    }));
    let mut request = ProviderRequest::simple("hello");
    request.model = "account-specific-alias".into();
    request.provider_account_id = Some("A".into());
    assert!(provider.generate(&request).await.is_err());
    assert!(state.check("A", &request.model).is_err());
    assert!(state.check("B", &request.model).is_ok());
    assert!(provider.stream(&request).await.is_err());
    state.refresh("A").unwrap();
    assert!(state.check("A", &request.model).is_err());
    state.quota_recovered("A").unwrap();
    server.reset().await;
    Mock::given(method("POST")).and(path("/chat/completions")).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"error\":{\"code\":\"model_not_found\",\"message\":\"unavailable\"}}\n\n").insert_header("content-type","text/event-stream")).expect(1).mount(&server).await;
    let mut receiver = provider.stream(&request).await.unwrap();
    assert_eq!(
        receiver.recv().await.unwrap().event_type,
        custos_provider::ProviderEventType::Failed
    );
    assert!(state.check("A", &request.model).is_err());
    assert!(state.check("B", &request.model).is_ok());
}

struct MockEmptyResolver;

#[async_trait]
impl CredentialResolver for MockEmptyResolver {
    async fn get_token(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn get_endpoint_url(&self, _provider_id: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

#[tokio::test]
async fn test_unconfigured_model_fails_closed_without_panic() {
    // Clear environment variables
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");

    let router = RouterModelProvider::new(Arc::new(MockEmptyResolver));
    let req = ProviderRequest::simple("Hello world");
    let result = router.generate(&req).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
}

struct FailoverResolver {
    endpoint: String,
    state: crate::routing::availability::ModelAvailability,
    accounts: std::sync::Mutex<Vec<String>>,
}
#[async_trait]
impl CredentialResolver for FailoverResolver {
    async fn select_account(
        &self,
        _service: &str,
        model: &str,
    ) -> Result<Option<String>, DomainError> {
        let mut list = self.accounts.lock().unwrap();
        let mut found = None;
        for (idx, acc) in list.iter().enumerate() {
            if self.state.check(acc, model).is_ok() {
                found = Some((idx, acc.clone()));
                break;
            }
        }
        if let Some((idx, acc)) = found {
            list.remove(idx);
            list.push(acc.clone());
            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }
    async fn get_token(&self, id: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(format!("{id}-token")))
    }
    async fn get_endpoint_url(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(self.endpoint.clone()))
    }
    async fn service_for_account(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some("openai".into()))
    }
    async fn validate_model(
        &self,
        account: &str,
        model: &str,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        self.state.check(account, model)
    }
    async fn observe_response(
        &self,
        account: &str,
        model: &str,
        status: u16,
        body: &str,
        retry_at: Option<i64>,
    ) -> Result<(), DomainError> {
        self.state.observe(account, model, status, body, retry_at)
    }
}

#[tokio::test]
async fn test_in_flight_429_auto_failover_to_next_account() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/chat/completions"))
            .and(header("authorization", "Bearer account-A-token"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({"error":{"code":"rate_limit_exceeded","message":"Rate limit reached"}})))
            .expect(1).mount(&server).await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer account-B-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "model":"gpt-4o",
            "choices":[{"message":{"content":"response from account B"}}],
            "usage":{"total_tokens":10}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let state = crate::routing::availability::ModelAvailability::default();
    let resolver = Arc::new(FailoverResolver {
        endpoint: server.uri(),
        state: state.clone(),
        accounts: std::sync::Mutex::new(vec!["account-A".into(), "account-B".into()]),
    });
    let provider = RouterModelProvider::new(resolver);
    let mut request = ProviderRequest::simple("hello pool");
    request.model = "gpt-4o".into();
    request.provider_account_id = Some("account-A".into());

    let response = provider
        .generate(&request)
        .await
        .expect("Failover to account B should succeed");
    assert_eq!(response.content, "response from account B");
    assert!(state.check("account-A", "gpt-4o").is_err());
    assert!(state.check("account-B", "gpt-4o").is_ok());
}
