use crate::api::LocalApiDispatcher;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub async fn write_live<W: tokio::io::AsyncWrite + Unpin>(
    stream: &mut W,
    bytes: &[u8],
) -> std::io::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        stream.write_all(bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Slow SSE subscriber"))?
}

pub async fn handle_sse_events(
    stream: &mut TcpStream,
    api: &Arc<LocalApiDispatcher>,
    path: &str,
    last_event_id: Option<u64>,
    cors_origin_header: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    let param = |key: &str| {
        query
            .split('&')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
            .unwrap_or_default()
    };
    let session = param("session_id");
    let run = param("run_id");
    let valid = |id: &str| {
        !id.is_empty()
            && id.len() <= 256
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    };
    let mut cursor = last_event_id
        .or_else(|| param("cursor").parse().ok())
        .unwrap_or(0);
    let initial = if valid(&session) && valid(&run) {
        api.live_run(&session, &run, cursor).await
    } else {
        Err(custos_domain::DomainError::Validation(
            "Invalid event scope".into(),
        ))
    };
    if initial.is_err() {
        let body = "{\"error\":\"Invalid or inaccessible run/session\"}";
        let response = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
            body.len(),
            cors_origin_header,
            body
        );
        write_live(stream, response.as_bytes()).await?;
        return Ok(());
    }
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nX-Accel-Buffering: no\r\n{}Connection: close\r\n\r\nretry: 1000\n\n",
        cors_origin_header
    );
    write_live(stream, headers.as_bytes()).await?;
    tracing::debug!(run_id = %run, session_id = %session, "SSE subscriber connected");
    let mut initial = initial.ok();
    let mut heartbeat = tokio::time::Instant::now();
    loop {
        let replay = match initial.take() {
            Some(replay) => replay,
            None => match api.live_run(&session, &run, cursor).await {
                Ok(replay) => replay,
                Err(_) => break,
            },
        };
        if replay.gap
            || (replay.events.is_empty()
                && matches!(
                    replay.snapshot.status.as_str(),
                    "completed" | "failed" | "cancelled" | "interrupted"
                ))
        {
            cursor = replay.snapshot.cursor;
            let data =
                serde_json::to_string(&replay.snapshot).map_err(std::io::Error::other)?;
            write_live(
                stream,
                format!("id: {cursor}\nevent: snapshot\ndata: {data}\n\n").as_bytes(),
            )
            .await?;
        } else {
            for event in replay.events {
                cursor = event.id;
                let data = serde_json::to_string(&event).map_err(std::io::Error::other)?;
                write_live(
                    stream,
                    format!("id: {cursor}\nevent: run\ndata: {data}\n\n").as_bytes(),
                )
                .await?;
            }
        }
        if matches!(
            replay.snapshot.status.as_str(),
            "completed" | "failed" | "cancelled" | "interrupted"
        ) {
            break;
        }
        if heartbeat.elapsed() >= Duration::from_secs(15) {
            write_live(stream, b": heartbeat\n\n").await?;
            heartbeat = tokio::time::Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}
