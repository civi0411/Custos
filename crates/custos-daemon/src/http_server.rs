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
    if lower == "tauri://localhost" {
        return true;
    }

    let Some(authority) = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
    else {
        return false;
    };

    // An Origin is scheme + authority only. Parse this narrow local profile
    // strictly instead of using a prefix match: `localhost.evil` and
    // `127.0.0.1.attacker.test` must never inherit local-client trust.
    if authority.is_empty() || authority.contains(['/', '?', '#', '@']) {
        return false;
    }

    if authority == "localhost" || authority == "127.0.0.1" || authority == "[::1]" {
        return true;
    }

    ["localhost:", "127.0.0.1:", "[::1]:"]
        .iter()
        .find_map(|prefix| authority.strip_prefix(prefix))
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_authorized_ipc_client(
    origin: Option<&str>,
    auth_header: Option<&str>,
    custom_token_header: Option<&str>,
    query_token: Option<&str>,
) -> bool {
    // 1. Strict Origin check: if an Origin header is present, it MUST be an allowed local/tauri origin
    if let Some(org) = origin {
        if !is_allowed_origin(org) {
            return false;
        }
    }

    // 2. Token enforcement: if CUSTOS_IPC_TOKEN is mandated, origin cannot bypass it
    if let Ok(expected_token) = std::env::var("CUSTOS_IPC_TOKEN") {
        let expected = expected_token.trim();
        if !expected.is_empty() {
            let provided = auth_header
                .and_then(|h| h.strip_prefix("Bearer ").or_else(|| h.strip_prefix("bearer ")))
                .map(|s| s.trim())
                .or_else(|| custom_token_header.map(|s| s.trim()))
                .or_else(|| query_token.map(|s| s.trim()));

            return provided == Some(expected);
        }
    }

    // 3. Default loopback allowance when CUSTOS_IPC_TOKEN is not strictly enforced
    true
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

    let query_token = path.split_once('?')
        .and_then(|(_, q)| {
            q.split('&')
                .filter_map(|p| p.split_once('='))
                .find(|(k, _)| *k == "token")
                .map(|(_, v)| v)
        });

    // IPC Token gate: verify token if mandated by CUSTOS_IPC_TOKEN
    if !is_authorized_ipc_client(req_origin.as_deref(), auth_header.as_deref(), token_header.as_deref(), query_token) {
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
        let query = path.split_once('?').map(|(_,q)|q).unwrap_or("");
        let param = |key: &str| query.split('&').filter_map(|p|p.split_once('=')).find(|(k,_)|*k==key).map(|(_,v)|v.to_string()).unwrap_or_default();
        let session = param("session_id"); let run = param("run_id");
        let valid = |id: &str| !id.is_empty() && id.len() <= 256 && id.bytes().all(|b|b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        let mut cursor = last_event_id.or_else(||param("cursor").parse().ok()).unwrap_or(0);
        let initial = if valid(&session) && valid(&run) { api.live_run(&session,&run,cursor).await } else { Err(custos_domain::DomainError::Validation("Invalid event scope".into())) };
        if initial.is_err() {
            let body = "{\"error\":\"Invalid or inaccessible run/session\"}";
            let response = format!("HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",body.len(),cors_origin_header,body);
            write_live(&mut stream,response.as_bytes()).await?; return Ok(());
        }
        let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nX-Accel-Buffering: no\r\n{}Connection: close\r\n\r\nretry: 1000\n\n",cors_origin_header);
        write_live(&mut stream,headers.as_bytes()).await?;
        tracing::debug!(run_id = %run, session_id = %session, "SSE subscriber connected");
        let mut initial = initial.ok();
        let mut heartbeat = tokio::time::Instant::now();
        loop {
            let replay = match initial.take() { Some(replay) => replay, None => match api.live_run(&session,&run,cursor).await { Ok(replay)=>replay, Err(_)=>break } };
            if replay.gap || (replay.events.is_empty() && matches!(replay.snapshot.status.as_str(),"completed"|"failed"|"cancelled"|"interrupted")) {
                cursor = replay.snapshot.cursor;
                let data = serde_json::to_string(&replay.snapshot).map_err(std::io::Error::other)?;
                write_live(&mut stream,format!("id: {cursor}\nevent: snapshot\ndata: {data}\n\n").as_bytes()).await?;
            } else {
                for event in replay.events {
                    cursor = event.id;
                    let data=serde_json::to_string(&event).map_err(std::io::Error::other)?;
                    write_live(&mut stream,format!("id: {cursor}\nevent: run\ndata: {data}\n\n").as_bytes()).await?;
                }
            }
            if matches!(replay.snapshot.status.as_str(),"completed"|"failed"|"cancelled"|"interrupted") { break; }
            if heartbeat.elapsed() >= Duration::from_secs(15) { write_live(&mut stream,b": heartbeat\n\n").await?; heartbeat=tokio::time::Instant::now(); }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn slow_subscriber_writer_times_out() {
        use super::*;
        let (mut server,client)=tokio::io::duplex(64);
        let data=vec![b'x';1024];
        let error=tokio::time::timeout(Duration::from_secs(7),write_live(&mut server,&data)).await.unwrap().unwrap_err();
        assert_eq!(error.kind(),std::io::ErrorKind::TimedOut);drop(client);
    }

    #[tokio::test]
    async fn sse_replays_after_start_and_checks_session_scope() {
        use std::sync::Arc;
        use custos_core::{contracts::workflow::WorkflowPort,TaskService};
        use custos_runtime::{SessionManager,workflow::TaskRuntime};
        use custos_domain::{SessionMode,StartRunCommand};
        use super::*;
        let store=Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let tasks=Arc::new(TaskService::new(store.clone()));let sessions=Arc::new(SessionManager::with_store(store.clone()));
        let task=tasks.create_task("SSE fixture".into()).await.unwrap();
        let session=sessions.create_session(SessionMode::Assisted).await;
        let other=sessions.create_session(SessionMode::Assisted).await;
        let bridge=Arc::new(custos_bridge::BridgeService::new(sessions.clone(),tasks.clone()));
        let runtime=Arc::new(TaskRuntime::new().with_run_store(store.clone()).with_model(Arc::new(custos_adapters::providers::fake::FakeProvider::with_default_text("test","answer"))));
        let run=runtime.start_run_background(StartRunCommand::new(&task.id,"chat").with_prompt("prompt").with_session_id(session.id.to_string()).with_preferred_mode("model")).await.unwrap();
        let api=Arc::new(LocalApiDispatcher::new(tasks,sessions,bridge).with_workflow(runtime));
        let (port,server)=start_http_server("127.0.0.1:0",api).await.unwrap();
        let url=format!("http://127.0.0.1:{port}/api/events?session_id={}&run_id={}",session.id,run.run_id);
        let response=reqwest::get(&url).await.unwrap();assert_eq!(response.status(),200);assert!(response.headers()["content-type"].to_str().unwrap().starts_with("text/event-stream"));
        let events=response.text().await.unwrap();assert!(events.contains("event: run"));assert!(events.contains("queued"));assert!(events.contains("completed"));assert!(events.contains("answer"));
        let cursor=events.lines().filter_map(|line|line.strip_prefix("id: ")).last().unwrap();
        let response=reqwest::Client::new().get(&url).header("Last-Event-ID",cursor).send().await.unwrap();let replay=response.text().await.unwrap();assert!(replay.contains("event: snapshot"));assert!(!replay.contains("text_delta"));
        let wrong=format!("http://127.0.0.1:{port}/api/events?session_id={}&run_id={}",other.id,run.run_id);assert_eq!(reqwest::get(wrong).await.unwrap().status(),403);
        server.abort();
    }

    #[test]
    fn origin_policy_accepts_only_exact_loopback_authorities() {
        for allowed in [
            "tauri://localhost",
            "http://localhost",
            "http://localhost:1420",
            "https://127.0.0.1:443",
            "http://[::1]:3000",
        ] {
            assert!(is_allowed_origin(allowed), "expected allowed: {allowed}");
        }

        for rejected in [
            "http://localhost.evil.test",
            "http://localhost@evil.test",
            "https://127.0.0.1.attacker.test",
            "http://127.0.0.1/path",
            "http://localhost:not-a-port",
            "https://example.com",
            "null",
        ] {
            assert!(
                !is_allowed_origin(rejected),
                "expected rejected: {rejected}"
            );
        }
    }

    #[test]
    fn ipc_token_enforcement_cannot_be_bypassed_by_origin() {
        std::env::set_var("CUSTOS_IPC_TOKEN", "secret-token-123");

        // Allowed origin without token MUST be rejected
        assert!(!is_authorized_ipc_client(Some("http://localhost:1420"), None, None, None));

        // Allowed origin with wrong token MUST be rejected
        assert!(!is_authorized_ipc_client(Some("http://localhost:1420"), Some("Bearer wrong"), None, None));

        // Untrusted origin even with valid token MUST be rejected
        assert!(!is_authorized_ipc_client(Some("http://evil.com"), Some("Bearer secret-token-123"), None, None));

        // Allowed origin with valid Bearer token MUST be accepted
        assert!(is_authorized_ipc_client(Some("http://localhost:1420"), Some("Bearer secret-token-123"), None, None));

        // Allowed origin with valid X-Custos-Token header MUST be accepted
        assert!(is_authorized_ipc_client(Some("http://localhost:1420"), None, Some("secret-token-123"), None));

        // Allowed origin with valid query token MUST be accepted
        assert!(is_authorized_ipc_client(Some("http://localhost:1420"), None, None, Some("secret-token-123")));

        // Direct client without origin with valid token MUST be accepted
        assert!(is_authorized_ipc_client(None, Some("Bearer secret-token-123"), None, None));

        std::env::remove_var("CUSTOS_IPC_TOKEN");

        // Without CUSTOS_IPC_TOKEN, untrusted origin MUST still be rejected
        assert!(!is_authorized_ipc_client(Some("http://evil.com"), None, None, None));
        // Without CUSTOS_IPC_TOKEN, allowed origin is accepted
        assert!(is_authorized_ipc_client(Some("http://localhost:1420"), None, None, None));
    }
}

async fn write_live<W: tokio::io::AsyncWrite + Unpin>(stream: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async { stream.write_all(bytes).await?; stream.flush().await }).await.map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut,"Slow SSE subscriber"))?
}
