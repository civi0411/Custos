use super::handler::handle_http_client;
use crate::api::LocalApiDispatcher;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

/// Maximum allowable HTTP request body (10 MB) to prevent unbounded memory allocation / DoS.
pub const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;
/// Maximum duration to read HTTP headers or body.
pub const READ_TIMEOUT: Duration = Duration::from_secs(15);

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
