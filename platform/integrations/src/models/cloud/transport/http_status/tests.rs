use super::error_mapper::is_context_length_exceeded;
use super::response_handler::{
    handle_response_with_limit, handle_status_with_limit, read_json_response_with_limit,
};
use super::retry::{
    duration_from_finite_secs, extract_retry_after, parse_http_date, parse_retry_after_header,
    MAX_RETRY_AFTER_SECS,
};
use crate::errors::ProviderError;
use flate2::{write::GzEncoder, Compression};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::Response;
use serde_json::{json, Value};
use std::io::Write;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn empty_headers() -> HeaderMap {
    HeaderMap::new()
}

fn headers_with_retry_after(value: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(RETRY_AFTER, value.parse().unwrap());
    h
}

async fn response_from_raw(raw_response: Vec<u8>) -> Response {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let _ = socket.read(&mut request).await;
        socket.write_all(&raw_response).await.unwrap();
    });

    reqwest::Client::new()
        .get(format!("http://{addr}"))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn bounded_json_accepts_body_without_content_length() {
    let response = response_from_raw(
        b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{\"ok\":true}"
            .to_vec(),
    )
    .await;

    let value: Value = read_json_response_with_limit(response, 64).await.unwrap();
    assert_eq!(value, json!({"ok": true}));
}

#[tokio::test]
async fn bounded_json_rejects_oversized_chunked_body() {
    let body = format!("{{\"value\":\"{}\"}}", "a".repeat(64));
    let raw = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        body.len(),
        body
    )
    .into_bytes();
    let response = response_from_raw(raw).await;

    let err = read_json_response_with_limit::<Value>(response, 64)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ProviderError::RequestFailed(ref msg) if msg.contains("exceeds the 64 byte limit")),
        "expected limit error, got: {err:?}"
    );
}

#[tokio::test]
async fn bounded_json_accepts_many_small_chunks() {
    let mut raw = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n"
        .to_vec();
    for byte in b"{\"ok\":true}" {
        raw.extend_from_slice(b"1\r\n");
        raw.push(*byte);
        raw.extend_from_slice(b"\r\n");
    }
    raw.extend_from_slice(b"0\r\n\r\n");

    let response = response_from_raw(raw).await;
    let value: Value = read_json_response_with_limit(response, 64).await.unwrap();
    assert_eq!(value, json!({"ok": true}));
}

#[tokio::test]
async fn bounded_json_limits_decompressed_body() {
    let server = MockServer::start().await;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let decompressed = format!("{{\"value\":\"{}\"}}", "a".repeat(128));
    encoder.write_all(decompressed.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .insert_header("content-type", "application/json")
                .set_body_bytes(compressed),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .get(server.uri())
        .send()
        .await
        .unwrap();

    let err = read_json_response_with_limit::<Value>(response, 64)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ProviderError::RequestFailed(ref msg) if msg.contains("exceeds the 64 byte limit")),
        "expected limit error on decompressed bytes, got: {err:?}"
    );
}

#[tokio::test]
async fn bounded_status_preserves_authentication_for_oversized_error_body() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(format!("{{\"error\":{{\"message\":\"{}\"}}}}", "a".repeat(128))),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .get(server.uri())
        .send()
        .await
        .unwrap();

    let err = handle_status_with_limit(response, 32).await.unwrap_err();
    assert!(
        matches!(err, ProviderError::Authentication(_)),
        "expected Authentication error, got: {err:?}"
    );
}

#[tokio::test]
async fn bounded_status_preserves_retry_after_for_oversized_error_body() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "12")
                .set_body_string(format!("{{\"error\":{{\"message\":\"{}\"}}}}", "a".repeat(128))),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .get(server.uri())
        .send()
        .await
        .unwrap();

    let err = handle_status_with_limit(response, 32).await.unwrap_err();
    assert!(
        matches!(
            err,
            ProviderError::RateLimitExceeded {
                retry_delay: Some(delay),
                ..
            } if delay == Duration::from_secs(12)
        ),
        "expected preserved retry delay, got: {err:?}"
    );
}

#[tokio::test]
async fn bounded_handle_response_rejects_oversized_success_body() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"value\":\"{}\"}}",
            "a".repeat(128)
        )))
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .get(server.uri())
        .send()
        .await
        .unwrap();

    let err = handle_response_with_limit(response, 32).await.unwrap_err();
    assert!(
        matches!(err, ProviderError::RequestFailed(ref msg) if msg.contains("exceeds the 32 byte limit")),
        "expected limit error, got: {err:?}"
    );
}

#[test]
fn retry_after_returns_none_when_neither_present() {
    assert_eq!(extract_retry_after(&empty_headers(), None), None);
}

#[test]
fn retry_after_prefers_body_seconds_over_header() {
    let headers = headers_with_retry_after("10");
    let payload = json!({
        "error": {
            "metadata": {
                "retry_after_seconds": 2.5
            }
        }
    });
    assert_eq!(
        extract_retry_after(&headers, Some(&payload)),
        Some(Duration::from_millis(2500))
    );
}

#[test]
fn retry_after_falls_back_to_header_when_body_missing() {
    let headers = headers_with_retry_after("15");
    assert_eq!(
        extract_retry_after(&headers, None),
        Some(Duration::from_secs(15))
    );
}

#[test]
fn retry_after_ignores_negative_or_nan_body_seconds() {
    let headers = headers_with_retry_after("7");
    let neg = json!({"error": {"metadata": {"retry_after_seconds": -1.0}}});
    let nan = json!({"error": {"metadata": {"retry_after_seconds": "not-a-number"}}});
    assert_eq!(
        extract_retry_after(&headers, Some(&neg)),
        Some(Duration::from_secs(7))
    );
    assert_eq!(
        extract_retry_after(&headers, Some(&nan)),
        Some(Duration::from_secs(7))
    );
}

#[test]
fn retry_after_clamps_absurd_body_seconds() {
    let payload = json!({
        "error": {
            "metadata": {
                "retry_after_seconds": 1e30
            }
        }
    });
    assert_eq!(
        extract_retry_after(&empty_headers(), Some(&payload)),
        Some(Duration::from_secs_f64(MAX_RETRY_AFTER_SECS))
    );
}

#[test]
fn retry_after_clamps_infinite_body_seconds() {
    assert_eq!(duration_from_finite_secs(f64::INFINITY), None);
    assert_eq!(duration_from_finite_secs(f64::NEG_INFINITY), None);
    assert_eq!(duration_from_finite_secs(f64::NAN), None);
    assert_eq!(duration_from_finite_secs(-0.1), None);
}

#[test]
fn retry_after_past_http_date_means_retry_now() {
    let delay = parse_retry_after_header("Sun, 06 Nov 1994 08:49:37 GMT");
    assert_eq!(delay, Some(Duration::ZERO));
}

#[test]
fn retry_after_future_http_date_parsed() {
    // A future HTTP-date should produce a positive duration up to the cap.
    let target = chrono::Utc::now() + chrono::Duration::seconds(45);
    let header_value = target.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let headers = headers_with_retry_after(&header_value);
    let delay = extract_retry_after(&headers, None).expect("should parse future HTTP-date");
    // Allow some slack for the clock advancing between header creation and parse.
    assert!(
        delay >= Duration::from_secs(30) && delay <= Duration::from_secs(60),
        "expected ~45s, got {delay:?}"
    );
}

#[test]
fn retry_after_parses_rfc850_http_date() {
    assert!(parse_http_date("Sunday, 06-Nov-94 08:49:37 GMT").is_some());
}

#[test]
fn retry_after_parses_asctime_http_date() {
    assert!(parse_http_date("Sun Nov  6 08:49:37 1994").is_some());
}

#[test]
fn context_length_classifier_accepts_context_window_errors() {
    let message = "This model's maximum context length is 8192 tokens. However, your messages resulted in 9000 tokens.";
    assert!(is_context_length_exceeded(None, message));
}

#[test]
fn context_length_classifier_rejects_generic_bad_request_errors() {
    let message = "Invalid parameter: max_tokens must be positive.";
    assert!(!is_context_length_exceeded(None, message));
}

#[test]
fn structured_context_length_classifier_handles_supported_and_invalid_payloads() {
    let exceeded_by_code = json!({
        "error": {
            "code": "context_length_exceeded",
            "message": "Custom message without standard tokens"
        }
    });
    assert!(is_context_length_exceeded(
        Some(&exceeded_by_code),
        "Custom message without standard tokens"
    ));

    let exceeded_by_tokens = json!({
        "error": {
            "n_prompt_tokens": 8193,
            "n_ctx": 8192,
            "message": "exceeded context"
        }
    });
    assert!(is_context_length_exceeded(
        Some(&exceeded_by_tokens),
        "exceeded context"
    ));

    let not_exceeded_tokens = json!({
        "error": {
            "n_prompt_tokens": 100,
            "n_ctx": 8192,
            "message": "invalid argument"
        }
    });
    assert!(!is_context_length_exceeded(
        Some(&not_exceeded_tokens),
        "invalid argument"
    ));

    let invalid_token_types = json!({
        "error": {
            "n_prompt_tokens": "too_many",
            "n_ctx": null,
            "message": "invalid argument"
        }
    });
    assert!(!is_context_length_exceeded(
        Some(&invalid_token_types),
        "invalid argument"
    ));
}
