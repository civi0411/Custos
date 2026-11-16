use super::super::*;
use super::helpers::TestCredentialResolver;
use std::sync::Arc;
use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ProviderRequest};

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
        crate::routing::probe::probe_endpoint_models("http://localhost:11434/v1", None, "ollama")
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
