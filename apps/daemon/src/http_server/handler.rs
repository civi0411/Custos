use super::security::{is_allowed_origin, is_authorized_ipc_client};
use super::server::{MAX_BODY_SIZE, READ_TIMEOUT};
use super::sse::handle_sse_events;
use crate::api::LocalApiDispatcher;
use crate::custos_local_api::ApiRequest;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub fn find_header_end(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(3) {
        if &buf[i..i + 4] == b"\r\n\r\n" {
            return Some(i + 4);
        }
    }
    for i in 0..buf.len().saturating_sub(1) {
        if &buf[i..i + 2] == b"\n\n" {
            return Some(i + 2);
        }
    }
    None
}

pub async fn handle_http_client(
    mut stream: TcpStream,
    api: Arc<LocalApiDispatcher>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 65536];
    let mut total_read = 0;

    // Read headers with strict timeout
    let header_end_pos = match tokio::time::timeout(READ_TIMEOUT, async {
        loop {
            let n = stream.read(&mut buf[total_read..]).await?;
            if n == 0 {
                return Ok::<Option<usize>, std::io::Error>(None);
            }
            total_read += n;

            if let Some(pos) = find_header_end(&buf[..total_read]) {
                return Ok(Some(pos));
            }
            if total_read >= buf.len() {
                return Ok(None);
            }
        }
    })
    .await
    {
        Ok(Ok(Some(pos))) => pos,
        _ => return Ok(()),
    };

    let header_str = String::from_utf8_lossy(&buf[..header_end_pos]);
    let mut lines = header_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return Ok(()),
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Extract headers
    let mut req_origin: Option<String> = None;
    let mut auth_header: Option<String> = None;
    let mut token_header: Option<String> = None;
    let mut content_length = 0;
    let mut last_event_id = None;
    for line in lines {
        let lower = line.to_lowercase();
        if let Some(val) = lower.strip_prefix("origin:") {
            req_origin = Some(val.trim().to_string());
        } else if let Some(val) = lower.strip_prefix("authorization:") {
            auth_header = Some(val.trim().to_string());
        } else if let Some(val) = lower.strip_prefix("x-custos-token:") {
            token_header = Some(val.trim().to_string());
        } else if let Some(val) = lower.strip_prefix("last-event-id:") {
            last_event_id = val.trim().parse::<u64>().ok();
        } else if let Some(val) = lower.strip_prefix("content-length:") {
            content_length = val.trim().parse::<usize>().unwrap_or(0);
        }
    }

    // Origin gate: if Origin header is present, enforce trusted origin validation
    let cors_origin_header = if let Some(ref origin) = req_origin {
        if !is_allowed_origin(origin) {
            let forbidden = r#"{"error":"Forbidden: untrusted cross-origin request rejected"}"#;
            let resp = format!(
                "HTTP/1.1 403 Forbidden\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\r\n{}",
                forbidden.len(),
                forbidden
            );
            stream.write_all(resp.as_bytes()).await?;
            stream.flush().await?;
            return Ok(());
        }
        format!("Access-Control-Allow-Origin: {origin}\r\n")
    } else {
        String::new()
    };

    // Handle OPTIONS CORS preflight
    if method == "OPTIONS" {
        let resp = format!(
            "HTTP/1.1 204 No Content\r\n\
{}Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization, X-Custos-Token, Last-Event-ID\r\n\
Access-Control-Max-Age: 86400\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\r\n",
            cors_origin_header
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    let query_token = path.split_once('?').and_then(|(_, q)| {
        q.split('&')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| *k == "token")
            .map(|(_, v)| v)
    });

    // IPC Token gate: verify token if mandated by CUSTOS_IPC_TOKEN
    if !is_authorized_ipc_client(
        req_origin.as_deref(),
        auth_header.as_deref(),
        token_header.as_deref(),
        query_token,
    ) {
        let unauth = r#"{"error":"Unauthorized: valid IPC token required"}"#;
        let resp = format!(
            "HTTP/1.1 401 Unauthorized\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            unauth.len(),
            cors_origin_header,
            unauth
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    if method == "GET" && path.starts_with("/api/events?") {
        return handle_sse_events(
            &mut stream,
            &api,
            path,
            last_event_id,
            &cors_origin_header,
        )
        .await;
    }

    // Handle GET /api/health
    if method == "GET" && (path == "/api/health" || path == "/health" || path == "/") {
        let body = r#"{"status":"ok","daemon":"custos-daemon","version":"0.1.0"}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            body.len(),
            cors_origin_header,
            body
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    // Handle POST /v1/otlp/traces and POST /v1/otlp/logs (Loopback + IPC token protected)
    if method == "POST" && (path == "/v1/otlp/traces" || path == "/v1/otlp/logs") {
        let body_bytes = match read_request_body(
            &mut stream,
            &buf[header_end_pos..total_read],
            content_length,
            &cors_origin_header,
        )
        .await
        {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(e) => return Err(e),
        };
        let ingest_res = if path == "/v1/otlp/traces" {
            api.ingest_otlp_traces_bytes(&body_bytes).await
        } else {
            api.ingest_otlp_logs_bytes(&body_bytes).await
        };
        let (status_line, resp_json) = match ingest_res {
            Ok(accepted) => (
                "200 OK",
                serde_json::json!({ "ok": true, "accepted": accepted }).to_string(),
            ),
            Err(err) => (
                "400 Bad Request",
                serde_json::json!({ "ok": false, "error": err.to_string() }).to_string(),
            ),
        };
        let resp = format!(
            "HTTP/1.1 {status_line}\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            resp_json.len(),
            cors_origin_header,
            resp_json
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    // Handle POST /api/request or /api/rpc
    if method == "POST" && (path == "/api/request" || path == "/api/rpc" || path == "/rpc") {
        let body_bytes = match read_request_body(
            &mut stream,
            &buf[header_end_pos..total_read],
            content_length,
            &cors_origin_header,
        )
        .await
        {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(e) => return Err(e),
        };

        let body_str = String::from_utf8_lossy(&body_bytes);
        let api_response = match serde_json::from_str::<ApiRequest>(&body_str) {
            Ok(req) => api.handle_request(req).await,
            Err(err) => crate::custos_local_api::ApiResponse::error(
                "",
                format!("Invalid request JSON: {err}"),
            ),
        };

        let resp_json = serde_json::to_string(&api_response)
            .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
        let resp = format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            resp_json.len(),
            cors_origin_header,
            resp_json
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    // Default 404
    let not_found = r#"{"error":"Not Found"}"#;
    let resp = format!(
        "HTTP/1.1 404 Not Found\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
        not_found.len(),
        cors_origin_header,
        not_found
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_request_body(
    stream: &mut TcpStream,
    initial_slice: &[u8],
    content_length: usize,
    cors_origin_header: &str,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if content_length > MAX_BODY_SIZE {
        let err_body = format!(
            r#"{{"error":"Payload Too Large: request body exceeds {} byte limit"}}"#,
            MAX_BODY_SIZE
        );
        let resp = format!(
            "HTTP/1.1 413 Payload Too Large\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            err_body.len(),
            cors_origin_header,
            err_body
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(None);
    }

    let mut body_bytes = initial_slice.to_vec();
    let read_body_res = tokio::time::timeout(READ_TIMEOUT, async {
        while body_bytes.len() < content_length {
            let mut chunk = vec![0u8; (content_length - body_bytes.len()).min(65536)];
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&chunk[..n]);
        }
        Ok::<(), std::io::Error>(())
    })
    .await;

    if read_body_res.is_err() {
        let timeout_body = r#"{"error":"Request Timeout while reading payload body"}"#;
        let resp = format!(
            "HTTP/1.1 408 Request Timeout\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
{}Connection: close\r\n\r\n{}",
            timeout_body.len(),
            cors_origin_header,
            timeout_body
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(None);
    }

    Ok(Some(body_bytes))
}
