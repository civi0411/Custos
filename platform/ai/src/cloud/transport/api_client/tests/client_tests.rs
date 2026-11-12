use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Response, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::cloud::transport::api_client::types::{
    ApiClient, AuthMethod, DEFAULT_CONNECT_TIMEOUT_SECS,
};

async fn spawn_chunked_server(gap_ms: u64, chunks: usize) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                if sock
                    .write_all(
                        b"HTTP/1.1 200 OK\r\n\
                          content-type: text/event-stream\r\n\
                          transfer-encoding: chunked\r\n\r\n",
                    )
                    .await
                    .is_err()
                {
                    return;
                }
                for i in 0..chunks {
                    if i > 0 {
                        tokio::time::sleep(Duration::from_millis(gap_ms)).await;
                    }
                    let data = format!("data: {}\n\n", i);
                    let chunk = format!("{:x}\r\n{}\r\n", data.len(), data);
                    if sock.write_all(chunk.as_bytes()).await.is_err() {
                        return;
                    }
                    let _ = sock.flush().await;
                }
                let _ = sock.write_all(b"0\r\n\r\n").await;
            });
        }
    });
    addr
}

fn client_with_timeout(addr: SocketAddr, timeout_ms: u64) -> ApiClient {
    let mut client = ApiClient::with_timeout_and_tls(
        format!("http://{}", addr),
        AuthMethod::NoAuth,
        Duration::from_millis(timeout_ms),
        None,
    )
    .unwrap();
    client.client = Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .read_timeout(client.timeout)
        .build()
        .unwrap();
    client
}

async fn drain_counting_data_lines(mut response: Response) -> Result<usize, reqwest::Error> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).matches("data:").count())
}

#[tokio::test]
async fn https_only_rejects_http_after_client_rebuild() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = ApiClient::new_with_tls(
        format!("http://{addr}"),
        AuthMethod::BearerToken("secret".to_string()),
        None,
    )
    .unwrap()
    .with_https_only()
    .unwrap()
    .with_header("x-test", "value")
    .unwrap();

    assert!(client.response_get("models").await.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn loopback_transport_rejects_remote_http_redirect() {
    for status in ["307 Temporary Redirect", "308 Permanent Redirect"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request).await;
            let response = format!(
                "HTTP/1.1 {status}\r\nLocation: http://192.0.2.1/capture\r\nContent-Length: 0\r\n\r\n"
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let client = ApiClient::new_with_tls(
            format!("http://{addr}"),
            AuthMethod::BearerToken("secret".to_string()),
            None,
        )
        .unwrap()
        .with_loopback_http_only()
        .unwrap()
        .with_header("x-test", "value")
        .unwrap();

        let error = client
            .response_post("chat", &serde_json::json!({ "secret": "prompt" }))
            .await
            .unwrap_err();

        assert!(
            format!("{error:#}").contains("redirect violates the loopback transport policy"),
            "unexpected redirect error: {error:#}"
        );
    }
}

#[tokio::test]
async fn streaming_request_survives_beyond_total_timeout() {
    let addr = spawn_chunked_server(50, 12).await;
    let client = client_with_timeout(addr, 400);

    let response = client
        .request("v1/messages")
        .streaming(true)
        .response_post(&serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let count = drain_counting_data_lines(response).await.unwrap();
    assert_eq!(count, 12);
}

#[tokio::test]
async fn streaming_request_fails_when_stream_stalls() {
    let addr = spawn_chunked_server(5_000, 2).await;
    let client = client_with_timeout(addr, 400);

    let response = client
        .request("v1/messages")
        .streaming(true)
        .response_post(&serde_json::json!({}))
        .await
        .unwrap();

    let err = drain_counting_data_lines(response)
        .await
        .expect_err("stalled stream should time out, not complete");
    assert!(err.is_timeout(), "expected a timeout error, got: {err}");
}

#[tokio::test]
async fn non_streaming_request_enforces_total_deadline() {
    let addr = spawn_chunked_server(50, 12).await;
    let client = client_with_timeout(addr, 400);

    let response = client
        .request("v1/messages")
        .response_post(&serde_json::json!({}))
        .await
        .unwrap();

    let err = drain_counting_data_lines(response)
        .await
        .expect_err("total deadline should cut off the response body");
    assert!(err.is_timeout(), "expected a timeout error, got: {err}");
}

#[tokio::test]
async fn streaming_request_times_out_before_response_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                while sock.read(&mut buf).await.is_ok_and(|n| n > 0) {}
            });
        }
    });
    let client = client_with_timeout(addr, 400);

    let started = std::time::Instant::now();
    let err = client
        .request("v1/messages")
        .streaming(true)
        .response_post(&serde_json::json!({}))
        .await
        .expect_err("the phase before the response body must stay bounded");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "should fail near the configured timeout, took {:?}",
        started.elapsed()
    );
    assert!(matches!(
        crate::errors::ProviderError::from(err),
        crate::errors::ProviderError::NetworkError(message)
            if message.starts_with("Request timed out")
    ));
}

#[tokio::test]
async fn streaming_error_body_shares_send_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 8192];
        let _ = socket.read(&mut buf).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        socket
            .write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 1\r\n\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = socket.write_all(b"x").await;
    });

    let client = client_with_timeout(addr, 400);
    let started = std::time::Instant::now();
    let response = client
        .request("v1/messages")
        .streaming(true)
        .response_post(&serde_json::json!({}))
        .await
        .unwrap();
    crate::cloud::transport::http_status::handle_status(response)
        .await
        .unwrap_err();

    assert!(
        started.elapsed() < Duration::from_millis(550),
        "send and error body used separate deadlines: {:?}",
        started.elapsed()
    );
}

#[test]
fn test_model_headers_applied_and_override_static_headers() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let client = ApiClient::new_with_tls(
            "http://localhost:8080".to_string(),
            AuthMethod::NoAuth,
            None,
        )
        .unwrap()
        .with_header("x-static", "static-value")
        .unwrap()
        .with_header("queue_threshold", "1000")
        .unwrap();

        let model_config = crate::model::ModelConfig::new("test-model").with_request_headers(
            Some(std::collections::HashMap::from([
                ("queue_threshold".to_string(), "500".to_string()),
                ("Idempotency-Key".to_string(), "abc-123".to_string()),
            ])),
        );

        let request = client
            .request("/test")
            .model_headers(&model_config)
            .unwrap()
            .send_request(|url, client| client.get(url))
            .await
            .unwrap();

        let headers = request.build().unwrap().headers().clone();
        let get = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };
        assert_eq!(get("queue_threshold"), Some("500".to_string()));
        assert_eq!(get("Idempotency-Key"), Some("abc-123".to_string()));
    });
}

#[test]
fn test_model_headers_rejects_invalid_header_name() {
    let client = ApiClient::new_with_tls(
        "http://localhost:8080".to_string(),
        AuthMethod::NoAuth,
        None,
    )
    .unwrap();

    let model_config = crate::model::ModelConfig::new("test-model").with_request_headers(Some(
        std::collections::HashMap::from([("bad header name".to_string(), "value".to_string())]),
    ));

    assert!(client
        .request("/test")
        .model_headers(&model_config)
        .is_err());
}

#[test]
fn test_request_builder_decorator() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let client = ApiClient::new_with_tls(
            "http://localhost:8080".to_string(),
            AuthMethod::BearerToken("test-token".to_string()),
            None,
        )
        .unwrap()
        .with_request_builder(Arc::new(|request| {
            Ok(request.header("test-my-session-id", "test-session_id-456"))
        }));

        let request = client
            .request("/test")
            .send_request(|url, client| client.get(url))
            .await
            .unwrap();

        let headers = request.build().unwrap().headers().clone();
        let actual = headers
            .get("test-my-session-id")
            .and_then(|value| value.to_str().ok());
        assert_eq!(actual, Some("test-session_id-456"));
    });
}

