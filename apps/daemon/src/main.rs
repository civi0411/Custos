use custos_daemon::{
    ApiRequest, ApiResponse, CustosRuntime, DaemonLock, DaemonLockError, ProfileResolver,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = ProfileResolver::from_env();
    let lock_path = profile.lock_path();

    let lock = match DaemonLock::acquire(&lock_path) {
        Ok(lock) => lock,
        Err(DaemonLockError::AlreadyRunning { pid, path }) => {
            eprintln!(
                "Custos daemon is already running (pid: {pid:?}) on lock file: {}",
                path.display()
            );
            return Ok(());
        }
        Err(err) => {
            eprintln!("Failed to acquire daemon process lock: {err}");
            return Err(Box::<dyn std::error::Error>::from(err));
        }
    };

    let runtime = Arc::new(CustosRuntime::bootstrap_profile(&profile)?);
    // Attempt legacy scrubbing before HTTP/SSE readers start. A locked vault
    // must not prevent the desktop from opening; the owned operation can retry.
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        runtime.credentials.initialize(),
    )
    .await
    {
        Ok(Ok(())) => {}
        _ => eprintln!("Provider secure storage needs attention; affected accounts remain blocked"),
    }
    let database_path = profile.database_path();

    // Bind TCP Listener on loopback (non-loopback requires explicit opt-in)
    let bind_addr = std::env::var("CUSTOS_BIND").unwrap_or_else(|_| "127.0.0.1:0".to_string());
    let tcp_loopback = bind_addr.starts_with("127.0.0.1:")
        || bind_addr.starts_with("localhost:")
        || bind_addr.starts_with("[::1]:");
    if !tcp_loopback && std::env::var("CUSTOS_ALLOW_NON_LOOPBACK").is_err() {
        return Err(format!(
            "Refusing to bind TCP server to non-loopback address '{bind_addr}' without CUSTOS_ALLOW_NON_LOOPBACK=1"
        )
        .into());
    }
    let tcp_listener = TcpListener::bind(&bind_addr).await?;
    let local_port = tcp_listener.local_addr()?.port();
    profile.write_port(local_port)?;

    let local_api = runtime.local_api.clone();

    // Bind HTTP Server on loopback (default 3000, or env CUSTOS_HTTP_BIND)
    let http_bind =
        std::env::var("CUSTOS_HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let http_api = local_api.clone();
    let (http_port, mut http_task) = match custos_daemon::start_http_server(&http_bind, http_api).await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Notice: primary HTTP bind on {http_bind} failed ({err}), falling back to dynamic port...");
            custos_daemon::start_http_server("127.0.0.1:0", local_api.clone()).await?
        }
    };
    profile.write_http_port(http_port)?;

    eprintln!(
        "Custos daemon ready; profile={}; database={}; tcp=127.0.0.1:{local_port}; http=http://127.0.0.1:{http_port}; transport=tcp+http+stdio-jsonl",
        profile.profile_id(),
        database_path.display()
    );

    // Spawn TCP accept loop
    let tcp_api = local_api.clone();
    let expected_ipc_token = std::env::var("CUSTOS_IPC_TOKEN")
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let mut tcp_task = tokio::spawn(async move {
        const MAX_TCP_LINE_BYTES: usize = 10 * 1024 * 1024;
        while let Ok((mut socket, _peer)) = tcp_listener.accept().await {
            let api = tcp_api.clone();
            let expected_token = expected_ipc_token.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(tokio::io::AsyncReadExt::take(
                    reader,
                    MAX_TCP_LINE_BYTES as u64,
                ))
                .lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let response = match serde_json::from_str::<ApiRequest>(&line) {
                        Ok(req) => {
                            if let Some(ref expected) = expected_token {
                                let provided = req
                                    .params
                                    .get("_ipc_token")
                                    .and_then(|v| v.as_str())
                                    .map(str::trim);
                                if provided != Some(expected.as_str()) {
                                    ApiResponse::error(
                                        req.id,
                                        "Unauthorized: invalid or missing IPC token",
                                    )
                                } else {
                                    api.handle_request(req).await
                                }
                            } else {
                                api.handle_request(req).await
                            }
                        }
                        Err(err) => ApiResponse::error("", format!("Invalid request JSON: {err}")),
                    };
                    if let Ok(mut encoded) = serde_json::to_vec(&response) {
                        encoded.push(b'\n');
                        if writer.write_all(&encoded).await.is_err()
                            || writer.flush().await.is_err()
                        {
                            break;
                        }
                    }
                }
            });
        }
    });

    // Stdio handler: processes incoming requests if piped
    let stdio_api = runtime.local_api.clone();
    let mut stdio_task = tokio::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        let mut stdout = tokio::io::stdout();

        let mut processed_any = false;
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            processed_any = true;
            let response = match serde_json::from_str::<ApiRequest>(&line) {
                Ok(request) => stdio_api.handle_request(request).await,
                Err(error) => ApiResponse::error("", format!("Invalid request JSON: {error}")),
            };
            if let Ok(mut encoded) = serde_json::to_vec(&response) {
                encoded.push(b'\n');
                if stdout.write_all(&encoded).await.is_err() || stdout.flush().await.is_err() {
                    break;
                }
            }
        }
        processed_any
    });

    let mut stdio_closed_idle = false;
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("Custos daemon received SIGINT, shutting down...");
                break;
            }
            _ = async {
                #[cfg(unix)]
                { sigterm.recv().await; }
                #[cfg(not(unix))]
                { sigterm.await; }
            } => {
                eprintln!("Custos daemon received SIGTERM, shutting down...");
                break;
            }
            _ = &mut tcp_task => {
                eprintln!("Custos daemon TCP listener exited, shutting down...");
                break;
            }
            _ = &mut http_task => {
                eprintln!("Custos daemon HTTP listener exited, shutting down...");
                break;
            }
            res = &mut stdio_task, if !stdio_closed_idle => {
                if let Ok(true) = res {
                    eprintln!("Custos daemon stdio channel closed after processing requests, shutting down...");
                    break;
                } else {
                    // Stdio closed without requests (/dev/null): keep monitoring tcp, http, and signals.
                    stdio_closed_idle = true;
                }
            }
        }
    }

    custos_integrations::host::caffeinate::CAFFEINATE_MANAGER.set_mode("off");
    let _ = profile.remove_port();
    drop(lock);

    Ok(())
}
