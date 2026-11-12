use super::html::render_success_html;
use super::utils::{
    extract_query_param, mask_oauth_token, oauth_provider_endpoint, oauth_provider_label,
};
use custos_domain::{DomainError, ProviderConfig};
use custos_persistence::SqliteTaskStore;
use custos_adapters::connectors::oauth::OAuthPkceManager;
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
    pub expires_at: i64,
}

pub struct OAuthCallbackServer {
    pub(crate) credentials: Arc<crate::credentials::ProviderCredentials>,
    pub(crate) pending: std::sync::Mutex<Option<PendingOAuthContext>>,
    pub(crate) status_tx: watch::Sender<CallbackServerStatus>,
    pub(crate) status_rx: watch::Receiver<CallbackServerStatus>,
    pub(crate) shutdown_tx: broadcast::Sender<()>,
}

impl OAuthCallbackServer {
    pub fn new(store: Arc<SqliteTaskStore>) -> Self {
        let repo = Arc::new(store.providers().clone());
        #[cfg(test)]
        let credentials = crate::credential_tests::fake_credentials(repo);
        #[cfg(not(test))]
        let credentials = {
            let vault: Arc<dyn custos_core::contracts::credentials::CredentialVault> =
                if std::env::var("CUSTOS_VAULT_BACKEND").as_deref() == Ok("keyring") {
                    Arc::new(custos_adapters::host::vault::KeyringCredentialVault)
                } else {
                    Arc::new(custos_adapters::host::vault::FileCredentialVault::new(
                        custos_adapters::host::vault::FileCredentialVault::default_dir(),
                    ))
                };
            Arc::new(crate::credentials::ProviderCredentials::new(repo, vault))
        };
        Self::with_credentials(credentials)
    }

    pub fn with_credentials(credentials: Arc<crate::credentials::ProviderCredentials>) -> Self {
        let (status_tx, status_rx) = watch::channel(CallbackServerStatus::Idle);
        let (shutdown_tx, _) = broadcast::channel(4);
        Self {
            credentials,
            pending: std::sync::Mutex::new(None),
            status_tx,
            status_rx,
            shutdown_tx,
        }
    }

    pub fn start_copilot_device(&self, device: serde_json::Value, account_id: String) {
        self.cancel();
        let mut shutdown = self.shutdown_tx.subscribe();
        let status = self.status_tx.clone();
        let credentials = self.credentials.clone();
        let _ = status.send(CallbackServerStatus::Listening { port: 0 });
        tokio::spawn(async move {
            let result = tokio::select! {
                _ = shutdown.recv() => return,
                result = custos_adapters::connectors::oauth::complete_copilot_device(&device) => result,
            };
            match result {
                Ok((mut token, identity)) => {
                    token.provider_id = account_id.clone();
                    let config = ProviderConfig {
                        id: account_id.clone(),
                        service_type: "copilot".into(),
                        name: identity.unwrap_or_else(|| "GitHub Copilot".into()),
                        api_key_masked: "oauth".into(),
                        status: "active".into(),
                        endpoint_url: Some("https://api.githubcopilot.com".into()),
                        default_model: None,
                        context_window: None,
                        fast_mode: Some(false),
                        created_at: token.created_at,
                        updated_at: token.updated_at,
                    };
                    match credentials
                        .save_oauth_connection(&token, Some(&config))
                        .await
                    {
                        Ok(_) => {
                            let _ = status.send(CallbackServerStatus::Completed {
                                provider_id: account_id.clone(),
                            });
                        }
                        Err(e) => {
                            let _ = status.send(CallbackServerStatus::Failed {
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = status.send(CallbackServerStatus::Failed {
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    pub fn status(&self) -> CallbackServerStatus {
        self.status_rx.borrow().clone()
    }

    pub fn subscribe_status(&self) -> watch::Receiver<CallbackServerStatus> {
        self.status_rx.clone()
    }

    pub fn validate_manual_callback(
        &self,
        provider: &str,
        verifier: &str,
        redirect: &str,
        callback: &str,
    ) -> Result<(), DomainError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| DomainError::Validation("OAuth state lock unavailable".into()))?;
        let context = pending.as_ref().ok_or_else(|| {
            DomainError::Validation("Start OAuth authorization before exchanging a callback".into())
        })?;
        if chrono::Utc::now().timestamp_millis() > context.expires_at {
            return Err(DomainError::Validation(
                "OAuth authorization expired; start again".into(),
            ));
        }
        if context.provider_id != provider
            || context.code_verifier != verifier
            || context.redirect_uri != redirect
            || extract_query_param(callback, "state")
                .or_else(|| callback.split_once('#').map(|v| v.1.to_string()))
                .as_deref()
                != Some(context.state.as_str())
        {
            return Err(DomainError::Validation(
                "OAuth callback does not match the pending authorization".into(),
            ));
        }
        if !matches!(
            self.status(),
            CallbackServerStatus::Listening { .. } | CallbackServerStatus::Idle
        ) {
            return Err(DomainError::Validation(
                "OAuth authorization already consumed or expired".into(),
            ));
        }
        *pending = None;
        let _ = self.shutdown_tx.send(());
        Ok(())
    }

    pub fn cancel(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
        let _ = self.shutdown_tx.send(());
        let _ = self.status_tx.send(CallbackServerStatus::Idle);
    }

    /// Spawns the loopback listener for a pending authorization context.
    /// Binds both IPv4 (127.0.0.1) and IPv6 ([::1]) to guarantee compatibility with
    /// operating systems (e.g. Windows) where `localhost` resolves to `::1` first.
    pub async fn start_listening(&self, context: PendingOAuthContext) -> Result<u16, DomainError> {
        *self
            .pending
            .lock()
            .map_err(|_| DomainError::Validation("OAuth state lock unavailable".into()))? =
            Some(context.clone());
        let callback_port = context
            .redirect_uri
            .split(':')
            .nth(2)
            .and_then(|v| v.split('/').next())
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(OAUTH_CALLBACK_PORT);
        // Cancel any existing running callback listener
        let _ = self.shutdown_tx.send(());

        // Wait briefly and retry binding IPv4 loopback to avoid WSAEADDRINUSE race condition
        let mut listener_v4 = None;
        let mut last_v4_err = None;
        for _ in 0..10 {
            match TcpListener::bind(format!("127.0.0.1:{callback_port}")).await {
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
                    port = callback_port,
                    "Cannot bind IPv4 OAuth loopback listener: {err_msg}. Fallback to manual callback paste."
                );
                return Err(DomainError::Validation(format!(
                    "Failed to bind IPv4 OAuth callback listener on port {callback_port}: {err_msg}"
                )));
            }
        };

        // Attempt IPv6 loopback bind ([::1]) to serve localhost on Windows cleanly
        let mut listener_v6 = None;
        for _ in 0..5 {
            match TcpListener::bind(format!("[::1]:{callback_port}")).await {
                Ok(l) => {
                    listener_v6 = Some(l);
                    break;
                }
                Err(e) => {
                    tracing::debug!(
                        port = callback_port,
                        "IPv6 loopback listener not yet bound: {e}"
                    );
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }

        let port = listener_v4
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(callback_port);

        let _ = self
            .status_tx
            .send(CallbackServerStatus::Listening { port });

        let status_tx = self.status_tx.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let credentials = self.credentials.clone();

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
                                tracing::warn!(
                                    "Error accepting IPv4 connection on port {port}: {e}"
                                );
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
                    let is_callback = first_line.starts_with("GET /oauth-callback")
                        || first_line.starts_with("GET /auth/callback")
                        || first_line.starts_with("GET /oauth_callback")
                        || first_line.starts_with("GET /callback");

                    if is_callback {
                        let query_str = first_line.split_whitespace().nth(1).unwrap_or_default();

                        let code = extract_query_param(query_str, "code");
                        let state = extract_query_param(query_str, "state");

                        if let Some(auth_code) = code {
                            if state.as_deref() != Some(context.state.as_str()) {
                                let _ = status_tx.send(CallbackServerStatus::Failed {
                                    error: "OAuth state mismatch or missing state".into(),
                                });
                                break;
                            }

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

                            let settings =
                                match custos_adapters::connectors::oauth::oauth_settings(
                                    &context.service_type,
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        let _ = status_tx.send(CallbackServerStatus::Failed {
                                            error: e.to_string(),
                                        });
                                        break;
                                    }
                                };
                            let pkce_manager = OAuthPkceManager::new(&settings.0, &settings.1);

                            // Exchange authorization code for tokens
                            match pkce_manager
                                .exchange_code_with_identity(
                                    &context.provider_id,
                                    &context.service_type,
                                    &context.client_id,
                                    &auth_code,
                                    &context.redirect_uri,
                                    &context.code_verifier,
                                )
                                .await
                            {
                                Ok((tokens, identity)) => {
                                    let now = chrono::Utc::now().timestamp_millis();
                                    let provider_config = ProviderConfig {
                                        id: context.provider_id.clone(),
                                        name: identity.unwrap_or_else(|| {
                                            format!(
                                                "{} (OAuth)",
                                                oauth_provider_label(
                                                    &context.provider_id,
                                                    &context.service_type
                                                )
                                            )
                                        }),
                                        service_type: context.service_type.clone(),
                                        api_key_masked: mask_oauth_token(&tokens.access_token),
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
                                    if let Err(err) = credentials
                                        .save_oauth_connection(&tokens, Some(&provider_config))
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to persist OAuth provider config: {err}"
                                        );
                                        let _ = status_tx.send(CallbackServerStatus::Failed {
                                            error: format!("Failed to save provider config: {err}"),
                                        });
                                        break;
                                    }

                                    tracing::info!(
                                        provider_id = %context.provider_id,
                                        "Successfully exchanged and persisted OAuth provider credentials"
                                    );
                                    let _ = status_tx.send(CallbackServerStatus::Completed {
                                        provider_id: context.provider_id.clone(),
                                    });
                                }
                                Err(err) => {
                                    tracing::error!("OAuth account connection failed: {err}");
                                    let _ = status_tx.send(CallbackServerStatus::Failed {
                                        error: format!("OAuth account connection failed: {err}"),
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

