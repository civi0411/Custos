use super::*;

#[test]
fn ollama_namespace_wins_over_cloud_model_names() {
    for model in [
        "ollama/gpt-custom:latest",
        "ollama/claude-local",
        "ollama/gemini",
        "ollama/custom:tag",
        "local/qwen2.5-coder-7b.gguf",
        "local:mistral-7b-instruct",
        "local/llama-3.1-8b-instruct-q4_k_m.gguf",
    ] {
        assert_eq!(RouterModelProvider::classify_provider(model), "local");
    }
}

#[tokio::test]
async fn ollama_native_request_and_errors() {
    use std::io::{Read, Write};
    for (status, body) in [
        (
            "200 OK",
            r#"{"message":{"content":"Local answer"},"done":true,"prompt_eval_count":7,"eval_count":3}"#,
        ),
        ("404 Not Found", r#"{"error":"model not found"}"#),
        ("200 OK", r#"{"message":{},"done":true}"#),
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut buf = [0; 4096];
                let n = socket.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&request);
                if let Some((headers, payload)) = text.split_once("\r\n\r\n") {
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.parse().ok())
                        })
                        .unwrap_or(0);
                    if payload.len() >= length {
                        break;
                    }
                }
            }
            write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
            oauth: false,
            endpoint: format!("{base}/v1/"),
            fail: true,
        }));
        let mut req = ProviderRequest::simple("hello local");
        req.model = "ollama/gpt-custom:tag".into();
        let result = provider.generate(&req).await;
        if body.contains("Local answer") {
            let result = result.unwrap();
            assert_eq!(result.content, "Local answer");
            assert_eq!(result.tokens_used, 10);
            assert_eq!(result.model_id, req.model);
        } else {
            assert!(result.is_err());
        }
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /api/chat "));
        assert!(!request.to_lowercase().contains("authorization:"));
        let payload: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(payload["model"], "gpt-custom:tag");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["options"]["num_predict"], 4096);
    }
}

#[tokio::test]
async fn ollama_stream_payload_routes_custom_tag_without_credentials() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/chat")).and(body_partial_json(serde_json::json!({"model":"gemini-custom:tag","stream":true,"messages":[{"role":"user","content":"hello local"}],"options":{"temperature":0.25,"num_predict":32}})))
            .respond_with(ResponseTemplate::new(200).set_body_string("{\"message\":{\"content\":\"hello\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true,\"prompt_eval_count\":7,\"eval_count\":2}\n").insert_header("content-type","application/x-ndjson"))
            .expect(1).mount(&server).await;
    let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
        oauth: false,
        endpoint: format!("{}/v1/", server.uri()),
        fail: true,
    }));
    let mut req = ProviderRequest::simple("hello local");
    req.model = "ollama/gemini-custom:tag".into();
    req.temperature = Some(0.25);
    req.max_tokens = Some(32);
    let mut receiver = provider.stream(&req).await.unwrap();
    assert_eq!(
        receiver.recv().await.unwrap().text.as_deref(),
        Some("hello")
    );
    let mut completed = false;
    while let Some(event) = receiver.recv().await {
        if event.event_type == custos_provider::ProviderEventType::Completed {
            assert_eq!(event.usage.unwrap().output_tokens, Some(2));
            completed = true;
        }
    }
    assert!(completed);
    assert!(!server.received_requests().await.unwrap()[0]
        .headers
        .contains_key("authorization"));
}

#[tokio::test]
async fn ollama_request_timeout_is_reported() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (_socket, _) = listener.accept().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
    });
    let mut provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
        oauth: false,
        endpoint: base,
        fail: true,
    }));
    provider.http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(30))
        .build()
        .unwrap();
    let mut req = ProviderRequest::simple("hello");
    req.model = "ollama/custom".into();
    assert!(provider
        .generate(&req)
        .await
        .unwrap_err()
        .to_string()
        .contains("Ollama connection failed"));
    server.join().unwrap();
}

#[tokio::test]
#[ignore = "requires a running local Ollama with qwen2.5:0.5b installed"]
async fn ollama_live_smoke() {
    let models =
        super::super::probe::probe_endpoint_models("http://localhost:11434/v1", None, "ollama")
            .await
            .unwrap();
    assert!(models.iter().any(|m| m.id == "ollama/qwen2.5:0.5b"));
    let provider = RouterModelProvider::new(Arc::new(EnvCredentialResolver));
    let mut req = ProviderRequest::simple("Reply with the word hello.");
    req.model = "ollama/qwen2.5:0.5b".into();
    req.max_tokens = Some(32);
    let response = provider.generate(&req).await.unwrap();
    assert!(!response.content.trim().is_empty());
    assert!(response.tokens_used > 0);
}

struct TestCredentialResolver {
    oauth: bool,
    endpoint: String,
    fail: bool,
}
#[async_trait]
impl CredentialResolver for TestCredentialResolver {
    async fn uses_oauth(&self, _: &str) -> Result<bool, DomainError> {
        Ok(self.oauth)
    }
    async fn get_token(&self, _: &str) -> Result<Option<String>, DomainError> {
        if self.fail {
            return Err(DomainError::Validation("OAuth refresh failed".into()));
        }
        Ok(Some(
            if self.oauth {
                "opaque-credential"
            } else {
                "eyJ-api-key"
            }
            .into(),
        ))
    }
    async fn get_endpoint_url(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(self.endpoint.clone()))
    }
}

#[tokio::test]
async fn oauth_with_configured_endpoint_uses_codex_and_preserves_model() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = socket.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&request);
            if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                let len: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|v| v.parse().ok())
                    })
                    .unwrap_or(0);
                if body.len() >= len {
                    break;
                }
            }
        }
        let body = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"OAuth answer\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"total_tokens\":5}}}\n\n";
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        String::from_utf8(request).unwrap()
    });
    let mut provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
        oauth: true,
        endpoint: format!("{base}/v1"),
        fail: false,
    }));
    provider.codex_endpoint = format!("{base}/codex/responses");
    let mut req = ProviderRequest::simple("hello OAuth");
    req.model = "gpt-4o".into();
    let result = provider.generate(&req).await.unwrap();
    assert_eq!(result.content, "OAuth answer");
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /codex/responses "));
    let payload: serde_json::Value =
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(payload["model"], "gpt-4o");
    assert_eq!(payload["input"][0]["content"][0]["text"], "hello OAuth");
}

#[tokio::test]
async fn api_key_uses_custom_api_endpoint_even_for_jwt_shaped_keys() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buf = [0; 4096];
        let n = socket.read(&mut buf).unwrap();
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{"model":"gpt-4o","choices":[{"message":{"content":"API answer"}}],"usage":{"total_tokens":5}}"#;
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        request
    });
    let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
        oauth: false,
        endpoint: format!("{base}/custom/v1"),
        fail: false,
    }));
    let mut req = ProviderRequest::simple("hello API");
    req.model = "gpt-4o".into();
    assert_eq!(provider.generate(&req).await.unwrap().content, "API answer");
    assert!(server
        .join()
        .unwrap()
        .starts_with("POST /custom/v1/chat/completions "));
}

struct AccountResolver {
    endpoint: String,
}
#[async_trait]
impl CredentialResolver for AccountResolver {
    async fn get_token(&self, id: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(format!("{id}-key")))
    }
    async fn get_endpoint_url(&self, id: &str) -> Result<Option<String>, DomainError> {
        assert!(id.starts_with("account-"));
        Ok(Some(self.endpoint.clone()))
    }
}

struct RotatingAccountResolver {
    endpoint: String,
    cursor: std::sync::atomic::AtomicUsize,
}
#[async_trait]
impl CredentialResolver for RotatingAccountResolver {
    async fn select_account(
        &self,
        service: &str,
        model: &str,
    ) -> Result<Option<String>, DomainError> {
        assert_eq!(service, "openai");
        assert_eq!(model, "gpt-test");
        let index = self
            .cursor
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            % 2;
        Ok(Some(format!(
            "account-{}",
            if index == 0 { "A" } else { "B" }
        )))
    }
    async fn get_token(&self, id: &str) -> Result<Option<String>, DomainError> {
        Ok(Some(format!("{id}-key")))
    }
    async fn get_endpoint_url(&self, id: &str) -> Result<Option<String>, DomainError> {
        assert!(id.starts_with("account-"));
        Ok(Some(self.endpoint.clone()))
    }
}
#[tokio::test]
async fn automatic_accounts_rotate_for_generate_and_stream_but_explicit_accounts_stay_pinned() {
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    for account in ["account-A", "account-B"] {
        Mock::given(method("POST")).and(path("/chat/completions")).and(header("authorization",format!("Bearer {account}-key"))).and(body_partial_json(serde_json::json!({"stream":true}))).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"choices\":[{\"delta\":{\"content\":\"answer\"}}]}\n\ndata: [DONE]\n\n").insert_header("content-type","text/event-stream")).with_priority(1).expect(1).mount(&server).await;
        Mock::given(method("POST")).and(path("/chat/completions")).and(header("authorization",format!("Bearer {account}-key"))).and(body_partial_json(serde_json::json!({"model":"gpt-test"}))).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"model":"gpt-test","choices":[{"message":{"content":"answer"}}],"usage":{"total_tokens":1}}))).with_priority(2).mount(&server).await;
    }
    let resolver = Arc::new(RotatingAccountResolver {
        endpoint: server.uri(),
        cursor: std::sync::atomic::AtomicUsize::new(0),
    });
    let provider = RouterModelProvider::new(resolver.clone());
    let mut request = ProviderRequest::simple("hello");
    request.model = "gpt-test".into();
    for _ in 0..2 {
        assert_eq!(provider.generate(&request).await.unwrap().content, "answer");
    }
    for _ in 0..2 {
        let mut events = provider.stream(&request).await.unwrap();
        while events.recv().await.is_some() {}
    }
    request.provider_account_id = Some("account-B".into());
    provider.generate(&request).await.unwrap();
    assert_eq!(resolver.cursor.load(std::sync::atomic::Ordering::SeqCst), 4);
    let requests = server.received_requests().await.unwrap();
    let accounts: Vec<_> = requests
        .iter()
        .map(|r| {
            r.headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        accounts,
        vec![
            "Bearer account-A-key",
            "Bearer account-B-key",
            "Bearer account-A-key",
            "Bearer account-B-key",
            "Bearer account-B-key"
        ]
    );
}
#[tokio::test]
async fn explicitly_selected_accounts_use_their_own_credentials_and_usage() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    for account in ["account-A", "account-B"] {
        Mock::given(method("POST")).and(path("/chat/completions")).and(header("authorization",format!("Bearer {account}-key"))).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"model":"gpt-test","choices":[{"message":{"content":"answer"}}],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12}}))).expect(1).mount(&server).await;
    }
    let provider = RouterModelProvider::new(Arc::new(AccountResolver {
        endpoint: server.uri(),
    }));
    for account in ["account-A", "account-B"] {
        let mut req = ProviderRequest::simple("hello");
        req.model = "gpt-test".into();
        req.provider_account_id = Some(account.into());
        let response = provider.generate(&req).await.unwrap();
        assert_eq!(response.input_tokens, Some(7));
        assert_eq!(response.output_tokens, Some(5));
        assert_eq!(response.total_tokens, Some(12));
    }
}

#[tokio::test]
async fn oauth_refresh_failure_never_falls_back() {
    let provider = RouterModelProvider::new(Arc::new(TestCredentialResolver {
        oauth: true,
        endpoint: "http://127.0.0.1:1/v1".into(),
        fail: true,
    }));
    let mut req = ProviderRequest::simple("hello");
    req.model = "gpt-4o".into();
    assert!(provider
        .generate(&req)
        .await
        .unwrap_err()
        .to_string()
        .contains("OAuth refresh failed"));
}

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
            let body="data: {\"type\":\"response.output_text.delta\",\"delta\":\"answer\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"model\":\"gpt-6.1-sol\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n";
            let mut mock = Mock::given(method("POST"))
                .and(path(endpoint))
                .and(body_partial_json(
                    serde_json::json!({"model":"gpt-6.1-sol","reasoning":{"effort":effort}}),
                ));
            if oauth {
                mock = mock.and(header(
                    "user-agent",
                    super::super::probe::codex_user_agent(),
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
    state: super::super::catalog::ModelAvailability,
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
    let state = super::super::catalog::ModelAvailability::default();
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
    state: super::super::catalog::ModelAvailability,
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

    let state = super::super::catalog::ModelAvailability::default();
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
