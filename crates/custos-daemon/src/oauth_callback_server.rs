//! OAuth Callback Server
//!
//! Lightweight, temporary loopback HTTP server on 127.0.0.1:1455
//! capturing standard Codex / OpenAI OAuth 2.0 PKCE redirect callbacks.

use custos_adapters::providers::oauth_pkce::OAuthPkceManager;
use custos_domain::{DomainError, ProviderConfig};
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};

pub const OAUTH_CALLBACK_PORT: u16 = 1455;
pub const OAUTH_CALLBACK_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackServerStatus {
    Idle,
    Listening { port: u16 },
    Exchanging { code: String },
    Completed { provider_id: String },
    Failed { error: String },
}

/// Active pending authorization state waiting for loopback callback
#[derive(Debug, Clone)]
pub struct PendingOAuthContext {
    pub provider_id: String,
    pub service_type: String,
    pub client_id: String,
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

pub struct OAuthCallbackServer {
    store: Arc<SqliteTaskStore>,
    pkce_manager: OAuthPkceManager,
    status_tx: watch::Sender<CallbackServerStatus>,
    status_rx: watch::Receiver<CallbackServerStatus>,
    shutdown_tx: broadcast::Sender<()>,
}

fn oauth_provider_endpoint(service_type: &str) -> Option<String> {
    match service_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => Some("https://api.openai.com/v1".to_string()),
        "anthropic" | "claude" => Some("https://api.anthropic.com/v1".to_string()),
        "gemini" | "google" => Some("https://generativelanguage.googleapis.com".to_string()),
        "deepseek" => Some("https://api.deepseek.com/v1".to_string()),
        _ => None,
    }
}

fn oauth_provider_label(provider_id: &str, service_type: &str) -> String {
    match service_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => "OpenAI".to_string(),
        "anthropic" | "claude" => "Anthropic".to_string(),
        "gemini" | "google" => "Google Gemini".to_string(),
        "deepseek" => "DeepSeek".to_string(),
        _ => provider_id.to_string(),
    }
}

fn mask_oauth_token(token: &str) -> String {
    if token.trim().is_empty() {
        return "none".to_string();
    }
    let prefix: String = token.chars().take(6).collect();
    format!("{prefix}********")
}

impl OAuthCallbackServer {
    pub fn new(store: Arc<SqliteTaskStore>) -> Self {
        let (status_tx, status_rx) = watch::channel(CallbackServerStatus::Idle);
        let (shutdown_tx, _) = broadcast::channel(4);
        Self {
            store,
            pkce_manager: OAuthPkceManager::default(),
            status_tx,
            status_rx,
            shutdown_tx,
        }
    }

    pub fn status(&self) -> CallbackServerStatus {
        self.status_rx.borrow().clone()
    }

    pub fn subscribe_status(&self) -> watch::Receiver<CallbackServerStatus> {
        self.status_rx.clone()
    }

    pub fn cancel(&self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.status_tx.send(CallbackServerStatus::Idle);
    }

    /// Spawns the loopback listener for a pending authorization context.
    /// Binds both IPv4 (127.0.0.1) and IPv6 ([::1]) to guarantee compatibility with
    /// operating systems (e.g. Windows) where `localhost` resolves to `::1` first.
    pub async fn start_listening(&self, context: PendingOAuthContext) -> Result<u16, DomainError> {
        // Cancel any existing running callback listener
        let _ = self.shutdown_tx.send(());

        // Wait briefly and retry binding IPv4 loopback to avoid WSAEADDRINUSE race condition
        let mut listener_v4 = None;
        let mut last_v4_err = None;
        for _ in 0..10 {
            match TcpListener::bind(format!("127.0.0.1:{OAUTH_CALLBACK_PORT}")).await {
                Ok(l) => {
                    listener_v4 = Some(l);
                    break;
                }
                Err(e) => {
                    last_v4_err = Some(e);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }

        let listener_v4 = match listener_v4 {
            Some(l) => l,
            None => {
                let err_msg = last_v4_err
                    .as_ref()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown socket error".to_string());
                tracing::warn!(
                    port = OAUTH_CALLBACK_PORT,
                    "Cannot bind IPv4 OAuth loopback listener: {err_msg}. Fallback to manual callback paste."
                );
                return Err(DomainError::Validation(format!(
                    "Failed to bind IPv4 OAuth callback listener on port {OAUTH_CALLBACK_PORT}: {err_msg}"
                )));
            }
        };

        // Attempt IPv6 loopback bind ([::1]) to serve localhost on Windows cleanly
        let mut listener_v6 = None;
        for _ in 0..5 {
            match TcpListener::bind(format!("[::1]:{OAUTH_CALLBACK_PORT}")).await {
                Ok(l) => {
                    listener_v6 = Some(l);
                    break;
                }
                Err(e) => {
                    tracing::debug!(
                        port = OAUTH_CALLBACK_PORT,
                        "IPv6 loopback listener not yet bound: {e}"
                    );
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }

        let port = listener_v4
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(OAUTH_CALLBACK_PORT);

        let _ = self
            .status_tx
            .send(CallbackServerStatus::Listening { port });

        let status_tx = self.status_tx.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let store = self.store.clone();
        let pkce_manager = self.pkce_manager.clone();

        tokio::spawn(async move {
            tracing::info!(
                port = port,
                has_ipv6 = listener_v6.is_some(),
                provider_id = %context.provider_id,
                "OAuth loopback callback server active on port {port} (dual-stack: IPv4 + IPv6)"
            );

            let server_future = async {
                loop {
                    let (mut socket, _addr) = if let Some(ref l6) = listener_v6 {
                        tokio::select! {
                            res = listener_v4.accept() => {
                                match res {
                                    Ok(pair) => pair,
                                    Err(e) => {
                                        tracing::warn!("Error accepting IPv4 connection on port {port}: {e}");
                                        break;
                                    }
                                }
                            }
                            res = l6.accept() => {
                                match res {
                                    Ok(pair) => pair,
                                    Err(e) => {
                                        tracing::warn!("Error accepting IPv6 connection on port {port}: {e}");
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        match listener_v4.accept().await {
                            Ok(pair) => pair,
                            Err(e) => {
                                tracing::warn!("Error accepting IPv4 connection on port {port}: {e}");
                                break;
                            }
                        }
                    };

                    let mut buf = vec![0u8; 4096];
                    let n = match socket.read(&mut buf).await {
                        Ok(read_bytes) if read_bytes > 0 => read_bytes,
                        _ => continue,
                    };

                    let request_str = String::from_utf8_lossy(&buf[..n]);
                    let first_line = request_str.lines().next().unwrap_or_default();

                    // Accept GET /auth/callback, GET /oauth_callback, or GET /callback
                    let is_callback = first_line.starts_with("GET /auth/callback")
                        || first_line.starts_with("GET /oauth_callback")
                        || first_line.starts_with("GET /callback");

                    if is_callback {
                        let query_str =
                            first_line.split_whitespace().nth(1).unwrap_or_default();

                                let code = extract_query_param(query_str, "code");
                                let state = extract_query_param(query_str, "state");

                                if let Some(auth_code) = code {
                                    // Send clean HTML response to browser
                                    let html_body = render_success_html();
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\n\
                                         Content-Type: text/html; charset=utf-8\r\n\
                                         Content-Length: {}\r\n\
                                         Connection: close\r\n\r\n{}",
                                        html_body.len(),
                                        html_body
                                    );

                                    let _ = socket.write_all(response.as_bytes()).await;
                                    let _ = socket.flush().await;

                                    let _ = status_tx.send(CallbackServerStatus::Exchanging {
                                        code: auth_code.clone(),
                                    });

                                    tracing::info!(
                                        provider_id = %context.provider_id,
                                        "Received OAuth callback code from browser, exchanging tokens"
                                    );

                                    // Verify state if provided
                                    if let Some(ref st) = state {
                                        if !context.state.is_empty() && st != &context.state {
                                            tracing::warn!("OAuth state parameter mismatch: expected {}, got {}", context.state, st);
                                        }
                                    }

                                    // Exchange authorization code for tokens
                                    match pkce_manager
                                        .exchange_code(
                                            &context.provider_id,
                                            &context.service_type,
                                            &context.client_id,
                                            &auth_code,
                                            &context.redirect_uri,
                                            &context.code_verifier,
                                        )
                                        .await
                                    {
                                        Ok(tokens) => {
                                            let providers = store.providers();
                                            if let Err(err) = providers.save_oauth_token(&tokens) {
                                                tracing::error!(
                                                    "Failed to persist exchanged OAuth token: {err}"
                                                );
                                                let _ =
                                                    status_tx.send(CallbackServerStatus::Failed {
                                                        error: format!(
                                                            "Failed to save token: {err}"
                                                        ),
                                                    });
                                            } else {
                                                let now = chrono::Utc::now().timestamp_millis();
                                                let provider_config = ProviderConfig {
                                                    id: context.provider_id.clone(),
                                                    name: format!(
                                                        "{} (OAuth)",
                                                        oauth_provider_label(
                                                            &context.provider_id,
                                                            &context.service_type
                                                        )
                                                    ),
                                                    service_type: context.service_type.clone(),
                                                    api_key_masked: mask_oauth_token(
                                                        &tokens.access_token,
                                                    ),
                                                    status: "active".to_string(),
                                                    endpoint_url: oauth_provider_endpoint(
                                                        &context.service_type,
                                                    ),
                                                    default_model: None,
                                                    context_window: None,
                                                    fast_mode: Some(false),
                                                    created_at: now,
                                                    updated_at: now,
                                                };
                                                if let Err(err) =
                                                    providers.save_provider(&provider_config)
                                                {
                                                    tracing::error!(
                                                        "Failed to persist OAuth provider config: {err}"
                                                    );
                                                    let _ = status_tx
                                                        .send(CallbackServerStatus::Failed {
                                                        error: format!(
                                                            "Failed to save provider config: {err}"
                                                        ),
                                                    });
                                                    break;
                                                }

                                                tracing::info!(
                                                    provider_id = %context.provider_id,
                                                    "Successfully exchanged and persisted OAuth provider credentials"
                                                );
                                                let _ = status_tx.send(
                                                    CallbackServerStatus::Completed {
                                                        provider_id: context.provider_id.clone(),
                                                    },
                                                );
                                            }
                                        }
                                        Err(err) => {
                                            tracing::error!("OAuth token exchange failed: {err}");
                                            let _ = status_tx.send(CallbackServerStatus::Failed {
                                                error: format!("Token exchange failed: {err}"),
                                            });
                                        }
                                    }

                                    break;
                                }
                            } else if first_line.starts_with("GET /favicon.ico") {
                                let not_found =
                                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                                let _ = socket.write_all(not_found.as_bytes()).await;
                            }
                }
            };

            tokio::select! {
                _ = server_future => {
                    tracing::debug!("OAuth loopback server completed connection handling");
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("OAuth loopback server received shutdown signal");
                }
                _ = tokio::time::sleep(OAUTH_CALLBACK_TIMEOUT) => {
                    tracing::info!("OAuth loopback server timed out after 10 minutes");
                    let _ = status_tx.send(CallbackServerStatus::Idle);
                }
            }
        });

        Ok(port)
    }
}

fn extract_query_param(path_and_query: &str, param_name: &str) -> Option<String> {
    let query = path_and_query.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.split('=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == param_name {
                let decoded = urlencoding_decode(v);
                return Some(decoded);
            }
        }
    }
    None
}

fn urlencoding_decode(val: &str) -> String {
    let mut result = String::new();
    let bytes = val.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&val[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn render_success_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Custos - Authorization Successful</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
      background: #0f1117;
      color: #f3f4f6;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
      margin: 0;
      padding: 20px;
      box-sizing: border-box;
    }
    .card {
      max-width: 440px;
      width: 100%;
      background: #1a1d27;
      border: 1px solid #2e3346;
      border-radius: 20px;
      padding: 40px 32px;
      text-align: center;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    }
    .icon {
      width: 56px;
      height: 56px;
      margin: 0 auto 20px auto;
      background: rgba(16, 185, 129, 0.15);
      border: 1px solid rgba(16, 185, 129, 0.3);
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #10b981;
      font-size: 28px;
    }
    h2 {
      font-size: 20px;
      font-weight: 600;
      color: #f9fafb;
      margin: 0 0 12px 0;
    }
    p {
      font-size: 14px;
      color: #9ca3af;
      line-height: 1.5;
      margin: 0 0 24px 0;
    }
    .badge {
      display: inline-block;
      padding: 6px 14px;
      background: #252a38;
      border: 1px solid #374151;
      border-radius: 8px;
      font-size: 12px;
      font-family: monospace;
      color: #10b981;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">&#10003;</div>
    <h2>Authorization Successful</h2>
    <p>Custos has securely connected to your OpenAI account. You may now close this browser tab and return to the application.</p>
    <div class="badge">Safe to close this window</div>
  </div>
</body>
</html>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query_param_basic() {
        let uri = "/auth/callback?code=abc123xyz&state=state_secret";
        assert_eq!(
            extract_query_param(uri, "code"),
            Some("abc123xyz".to_string())
        );
        assert_eq!(
            extract_query_param(uri, "state"),
            Some("state_secret".to_string())
        );
        assert_eq!(extract_query_param(uri, "missing"), None);
    }

    #[test]
    fn test_extract_query_param_url_encoded() {
        let uri = "/oauth_callback?code=test%2F123%2Bcode&state=val+space";
        assert_eq!(
            extract_query_param(uri, "code"),
            Some("test/123+code".to_string())
        );
        assert_eq!(
            extract_query_param(uri, "state"),
            Some("val space".to_string())
        );
    }

    #[test]
    fn test_render_success_html() {
        let html = render_success_html();
        assert!(html.contains("Authorization Successful"));
        assert!(html.contains("Custos has securely connected"));
    }
}
