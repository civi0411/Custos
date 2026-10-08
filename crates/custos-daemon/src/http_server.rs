use crate::api::LocalApiDispatcher;
use crate::custos_local_api::ApiRequest;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn start_http_server(
    bind_addr: &str,
    api: Arc<LocalApiDispatcher>,
) -> Result<(u16, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
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

async fn handle_http_client(
    mut stream: TcpStream,
    api: Arc<LocalApiDispatcher>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 65536];
    let mut total_read = 0;

    // Read headers
    let header_end_pos = loop {
        let n = stream.read(&mut buf[total_read..]).await?;
        if n == 0 {
            return Ok(());
        }
        total_read += n;

        if let Some(pos) = find_header_end(&buf[..total_read]) {
            break pos;
        }
        if total_read >= buf.len() {
            return Ok(());
        }
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

    // Handle OPTIONS CORS preflight
    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
Access-Control-Max-Age: 86400\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\r\n";
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
Access-Control-Allow-Origin: *\r\n\
Connection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    // Handle POST /api/request or /api/rpc
    if method == "POST" && (path == "/api/request" || path == "/api/rpc" || path == "/rpc") {
        let mut content_length = 0;
        for line in lines {
            if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                content_length = val.trim().parse::<usize>().unwrap_or(0);
            }
        }

        let body_start = header_end_pos;
        let mut body_bytes = buf[body_start..total_read].to_vec();

        while body_bytes.len() < content_length {
            let mut chunk = vec![0u8; content_length - body_bytes.len()];
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&chunk[..n]);
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
Access-Control-Allow-Origin: *\r\n\
Connection: close\r\n\r\n{}",
            resp_json.len(),
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
Access-Control-Allow-Origin: *\r\n\
Connection: close\r\n\r\n{}",
        not_found.len(),
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
