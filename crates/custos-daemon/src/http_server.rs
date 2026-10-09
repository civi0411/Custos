use crate::api::LocalApiDispatcher;
use crate::custos_local_api::ApiRequest;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Maximum allowable HTTP request body (10 MB) to prevent unbounded memory allocation / DoS.
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;
/// Maximum duration to read HTTP headers or body.
const READ_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn start_http_server(
    bind_addr: &str,
    api: Arc<LocalApiDispatcher>,
) -> Result<(u16, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    // Invariant: Non-loopback bindings require explicit opt-in via CUSTOS_ALLOW_NON_LOOPBACK
    let is_loopback = bind_addr.starts_with("127.0.0.1:")
        || bind_addr.starts_with("localhost:")
        || bind_addr.starts_with("[::1]:");

    if !is_loopback && std::env::var("CUSTOS_ALLOW_NON_LOOPBACK").is_err() {
        return Err(format!(
            "Refusing to bind HTTP server to non-loopback address '{bind_addr}' without CUSTOS_ALLOW_NON_LOOPBACK=1"
        )
        .into());
    }

    let listener = TcpListener::bind(bind_addr).await?;
    let port = listener.local_addr()?.port();

    let task = tokio::spawn(async move {
        while let Ok((socket, _peer)) = listener.accept().await {
            let api = api.clone();
            tokio::spawn(async move {
                let _ = handle_http_client(socket, api).await;
            });
        }
    });

    Ok((port, task))
}

fn is_allowed_origin(origin: &str) -> bool {
    let lower = origin.trim().to_lowercase();
    lower == "tauri://localhost"
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://127.0.0.1")
        || lower.starts_with("https://localhost")
        || lower.starts_with("https://127.0.0.1")
}

async fn handle_http_client(
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

    // Extract Origin header
    let mut req_origin: Option<String> = None;
    let mut content_length = 0;
    for line in lines {
        let lower = line.to_lowercase();
        if let Some(val) = lower.strip_prefix("origin:") {
            req_origin = Some(val.trim().to_string());
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
Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
Access-Control-Max-Age: 86400\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\r\n",
            cors_origin_header
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
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

    // Handle POST /api/request or /api/rpc
    if method == "POST" && (path == "/api/request" || path == "/api/rpc" || path == "/rpc") {
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
            return Ok(());
        }

        let body_start = header_end_pos;
        let mut body_bytes = buf[body_start..total_read].to_vec();

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
            return Ok(());
        }

        let body_str = String::from_utf8_lossy(&body_bytes);
        let api_response = match serde_json::from_str::<ApiRequest>(&body_str) {
            Ok(req) => api.handle_request(req).await,
            Err(err) => crate::custos_local_api::ApiResponse::error("", format!("Invalid request JSON: {err}")),
        };

        let resp_json = serde_json::to_string(&api_response).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
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

fn find_header_end(buf: &[u8]) -> Option<usize> {
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
