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
    let database_path = profile.database_path();

    // Bind TCP Listener on loopback
    let bind_addr = std::env::var("CUSTOS_BIND").unwrap_or_else(|_| "127.0.0.1:0".to_string());
    let tcp_listener = TcpListener::bind(&bind_addr).await?;
    let local_port = tcp_listener.local_addr()?.port();
    profile.write_port(local_port)?;

    let local_api = runtime.local_api.clone();

    // Bind HTTP Server on loopback (default 3000, or env CUSTOS_HTTP_BIND)
    let http_bind = std::env::var("CUSTOS_HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let http_api = local_api.clone();
    let (http_port, http_task) = match custos_daemon::start_http_server(&http_bind, http_api).await {
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
    let tcp_task = tokio::spawn(async move {
        while let Ok((mut socket, _peer)) = tcp_listener.accept().await {
            let api = tcp_api.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let response = match serde_json::from_str::<ApiRequest>(&line) {
                        Ok(req) => api.handle_request(req).await,
                        Err(err) => ApiResponse::error("", format!("Invalid request JSON: {err}")),
                    };
                    if let Ok(mut encoded) = serde_json::to_vec(&response) {
                        encoded.push(b'\n');
                        if writer.write_all(&encoded).await.is_err() || writer.flush().await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    });

    // Stdio handler: processes incoming requests if piped
    let stdio_api = runtime.local_api.clone();
    let stdio_task = tokio::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        let mut stdout = tokio::io::stdout();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
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
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("Custos daemon received interrupt signal, shutting down...");
        }
        _ = tcp_task => {}
        _ = http_task => {}
        _ = stdio_task => {}
    }

    let _ = profile.remove_port();
    drop(lock);

    Ok(())
}
