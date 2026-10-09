//! OAuth Callback Server
//!
//! Lightweight, temporary loopback HTTP server on 127.0.0.1:1455
//! capturing standard Codex / OpenAI OAuth 2.0 PKCE redirect callbacks.

use custos_adapters::providers::oauth_pkce::OAuthPkceManager;
use custos_domain::DomainError;
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
    pub async fn start_listening(
        &self,
        context: PendingOAuthContext,
    ) -> Result<u16, DomainError> {
        // Cancel any existing running callback listener
        let _ = self.shutdown_tx.send(());

        let bind_addr = format!("127.0.0.1:{OAUTH_CALLBACK_PORT}");
        let listener = match TcpListener::bind(&bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(
                    port = OAUTH_CALLBACK_PORT,
                    "Cannot bind OAuth loopback listener (port in use?): {e}. Fallback to manual callback paste."
                );
                return Err(DomainError::Validation(format!(
                    "Failed to bind OAuth callback listener on port {OAUTH_CALLBACK_PORT}: {e}"
                )));
            }
        };

        let port = listener
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
                provider_id = %context.provider_id,
                "OAuth loopback callback server active on port {port}"
            );

            let server_future = async {
                loop {
                    match listener.accept().await {
                        Ok((mut socket, _addr)) => {
                            let mut buf = vec![0u8; 4096];
                            let n = match socket.read(&mut buf).await {
                                Ok(read_bytes) if read_bytes > 0 => read_bytes,
                                _ => continue,
                            };

                            let request_str = String::from_utf8_lossy(&buf[..n]);
                            let first_line = request_str.lines().next().unwrap_or_default();

                            // Look for GET /auth/callback?...
                            if first_line.starts_with("GET /auth/callback") {
                                let query_str = first_line
                                    .split_whitespace()
                                    .nth(1)
                                    .unwrap_or_default();

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
                                            if let Err(err) = store
                                                .providers()
                                                .save_oauth_token(&tokens)
                                            {
                                                tracing::error!(
                                                    "Failed to persist exchanged OAuth token: {err}"
                                                );
                                                let _ = status_tx.send(CallbackServerStatus::Failed {
                                                    error: format!("Failed to save token: {err}"),
                                                });
                                            } else {
                                                tracing::info!(
                                                    provider_id = %context.provider_id,
                                                    "Successfully exchanged and persisted OAuth token record"
                                                );
                                                let _ = status_tx.send(CallbackServerStatus::Completed {
                                                    provider_id: context.provider_id.clone(),
                                                });
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
                                let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                                let _ = socket.write_all(not_found.as_bytes()).await;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Error accepting connection on port {port}: {e}");
                            break;
                        }
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
