use async_trait::async_trait;
pub use custos_protocol::local_api::{ApiRequest, ApiResponse, ApiTransport};

/// In-process closure transport for testing and local integration
pub struct FnTransport<F> {
    handler: F,
}

impl<F> FnTransport<F> {
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl<F, Fut> ApiTransport for FnTransport<F>
where
    F: Fn(ApiRequest) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ApiResponse> + Send,
{
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        Ok((self.handler)(req).await)
    }
}

/// Child process stdio-jsonl transport communicating with custos-daemon binary
pub struct ProcessTransport {
    stdin: tokio::sync::Mutex<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    lines: tokio::sync::Mutex<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>>,
    child: tokio::sync::Mutex<tokio::process::Child>,
}

impl ProcessTransport {
    pub fn spawn(daemon_binary: &str, db_path: Option<&str>) -> Result<Self, std::io::Error> {
        let mut cmd = tokio::process::Command::new(daemon_binary);
        if let Some(db) = db_path {
            cmd.env("CUSTOS_DB_PATH", db);
            if let Some(parent) = std::path::Path::new(db).parent() {
                cmd.env("CUSTOS_PROFILE_DIR", parent);
            }
        }
        cmd.env("CUSTOS_BIND", "127.0.0.1:0");
        cmd.env("CUSTOS_HTTP_BIND", "127.0.0.1:0");
        cmd.kill_on_drop(true);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn()?;
        let stdin = tokio::io::BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("Failed to capture child stdin"))?,
        );
        let stdout = tokio::io::BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| std::io::Error::other("Failed to capture child stdout"))?,
        );
        let lines = tokio::io::AsyncBufReadExt::lines(stdout);

        Ok(Self {
            stdin: tokio::sync::Mutex::new(stdin),
            lines: tokio::sync::Mutex::new(lines),
            child: tokio::sync::Mutex::new(child),
        })
    }

    pub async fn kill(&self) -> Result<(), std::io::Error> {
        let mut child = self.child.lock().await;
        child.kill().await
    }
}

#[async_trait]
impl ApiTransport for ProcessTransport {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        use tokio::io::AsyncWriteExt;

        let mut encoded = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        encoded.push(b'\n');

        // Hold both transport locks across write+read so concurrent callers cannot cross-talk responses.
        let mut stdin = self.stdin.lock().await;
        let mut lines = self.lines.lock().await;
        stdin.write_all(&encoded).await.map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())?;

        while let Some(line) =
            tokio::time::timeout(std::time::Duration::from_secs(60), lines.next_line())
                .await
                .map_err(|_| "Timed out waiting for daemon stdout response".to_string())?
                .map_err(|e| e.to_string())?
        {
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str::<ApiResponse>(&line)
                .map_err(|e| format!("Invalid JSON response: {e}"));
        }
        Err("Daemon closed stdout stream without response".to_string())
    }
}

/// TCP socket transport communicating with custos-daemon local API server
pub struct TcpTransport {
    addr: String,
}

impl TcpTransport {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

#[async_trait]
impl ApiTransport for TcpTransport {
    async fn send_request(&self, mut req: ApiRequest) -> Result<ApiResponse, String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        // Daemon TCP enforces CUSTOS_IPC_TOKEN when set; forward it from the client environment.
        if let Ok(token) = std::env::var("CUSTOS_IPC_TOKEN") {
            let token = token.trim().to_string();
            if !token.is_empty() {
                if let Some(obj) = req.params.as_object_mut() {
                    obj.insert("_ipc_token".into(), serde_json::Value::String(token));
                } else if req.params.is_null() {
                    req.params = serde_json::json!({ "_ipc_token": token });
                }
            }
        }

        let stream = tokio::net::TcpStream::connect(&self.addr)
            .await
            .map_err(|e| format!("Failed to connect to daemon at {}: {e}", self.addr))?;
        let (reader, mut writer) = stream.into_split();

        let mut encoded = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        encoded.push(b'\n');

        writer
            .write_all(&encoded)
            .await
            .map_err(|e| e.to_string())?;
        writer.flush().await.map_err(|e| e.to_string())?;

        let mut lines = BufReader::new(reader).lines();
        while let Some(line) =
            tokio::time::timeout(std::time::Duration::from_secs(60), lines.next_line())
                .await
                .map_err(|_| "Timed out waiting for daemon TCP response".to_string())?
                .map_err(|e| e.to_string())?
        {
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str::<ApiResponse>(&line)
                .map_err(|e| format!("Invalid JSON response: {e}"));
        }
        Err("Daemon closed TCP connection without response".to_string())
    }
}
