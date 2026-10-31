use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    pub session_id: String,
    pub tool_call_request_id: String,
    pub working_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit_id: Option<String>,
}

impl ToolCallContext {
    pub fn new(session_id: impl Into<String>, tool_call_request_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            tool_call_request_id: tool_call_request_id.into(),
            working_dir: None,
            task_id: None,
            run_id: None,
            permit_id: None,
        }
    }

    pub fn with_working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }

    /// Attach SADE execution governance identifiers (Task, Run, and ExecutionPermit)
    pub fn with_sade_scope(
        mut self,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        permit_id: Option<String>,
    ) -> Self {
        self.task_id = Some(task_id.into());
        self.run_id = Some(run_id.into());
        self.permit_id = permit_id;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustosMcpHostInfo {
    pub explicit_extensions: bool,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
}

pub type GooseMcpHostInfo = CustosMcpHostInfo;

#[async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn list_tools(&self, next_cursor: Option<String>) -> Result<Vec<Tool>>;
    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult>;
    async fn list_resources(&self, _next_cursor: Option<String>) -> Result<Vec<Value>> {
        Ok(Vec::new())
    }
    async fn read_resource(&self, _uri: &str) -> Result<Value> {
        anyhow::bail!("read_resource is not supported by this client")
    }
}

/// Stdio connection managing a live child process communicating via JSON-RPC 2.0 lines.
pub struct StdioProcess {
    _child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout_reader: Lines<BufReader<ChildStdout>>,
    next_id: u64,
}

impl StdioProcess {
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        current_dir: Option<&PathBuf>,
        host_info: &CustosMcpHostInfo,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.kill_on_drop(true);

        #[cfg(not(windows))]
        {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let mut parts: Vec<String> = Vec::new();
            if let Some(home) = dirs::home_dir() {
                let local_bin = home.join(".local/bin");
                if local_bin.exists() {
                    parts.push(local_bin.to_string_lossy().to_string());
                }
                let cargo_bin = home.join(".cargo/bin");
                if cargo_bin.exists() {
                    parts.push(cargo_bin.to_string_lossy().to_string());
                }
            }
            if std::path::Path::new("/opt/homebrew/bin").exists() {
                parts.push("/opt/homebrew/bin".to_string());
            }
            if std::path::Path::new("/usr/local/bin").exists() {
                parts.push("/usr/local/bin".to_string());
            }
            if !current_path.is_empty() {
                parts.push(current_path);
            }
            cmd.env("PATH", parts.join(":"));
        }

        for (k, v) in env {
            cmd.env(k, v);
        }

        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to acquire child stdin for MCP stdio"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to acquire child stdout for MCP stdio"))?;

        let mut writer = BufWriter::new(stdin);
        let mut reader = BufReader::new(stdout).lines();

        // Send MCP initialize handshake
        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": host_info.client_name.as_deref().unwrap_or("custos"),
                    "version": host_info.client_version.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"))
                }
            }
        });

        writer.write_all(format!("{}\n", init_req).as_bytes()).await?;
        writer.flush().await?;

        // Wait for initialize response with 15s timeout
        let init_timeout = std::time::Duration::from_secs(15);
        tokio::time::timeout(init_timeout, async {
            while let Some(line) = reader.next_line().await? {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                    if v.get("id").and_then(|id| id.as_u64()) == Some(1) {
                        if let Some(err) = v.get("error") {
                            anyhow::bail!("MCP initialize error: {:?}", err);
                        }
                        return Ok(());
                    }
                }
            }
            anyhow::bail!("MCP child process stdout closed before initialize response")
        })
        .await
        .map_err(|_| anyhow::anyhow!("MCP initialize timed out after 15 seconds"))??;

        // Send notifications/initialized
        let notify = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        writer.write_all(format!("{}\n", notify).as_bytes()).await?;
        writer.flush().await?;

        Ok(Self {
            _child: child,
            stdin: writer,
            stdout_reader: reader,
            next_id: 2,
        })
    }

    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let timeout_duration = std::time::Duration::from_secs(30);
        tokio::time::timeout(timeout_duration, self.send_request_inner(method, params))
            .await
            .map_err(|_| anyhow::anyhow!("MCP stdio request '{}' timed out after 30 seconds", method))?
    }

    async fn send_request_inner(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.stdin.write_all(format!("{}\n", req).as_bytes()).await?;
        self.stdin.flush().await?;

        while let Some(line) = self.stdout_reader.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                    if let Some(err) = v.get("error") {
                        let code = err.get("code");
                        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
                        anyhow::bail!("MCP error (code {:?}): {}", code, msg);
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }

        anyhow::bail!("MCP child process stdout closed before response for request {}", id)
    }
}

/// HTTP connection managing communication with an external MCP HTTP/SSE endpoint.
pub struct HttpClient {
    url: String,
    auth_token: Option<String>,
    headers: HashMap<String, String>,
    client: reqwest::Client,
    next_id: AtomicU64,
}

impl HttpClient {
    pub fn new(
        url: String,
        auth_token: Option<String>,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            url,
            auth_token,
            headers,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let mut rb = self.client.post(&self.url).json(&req);
        if let Some(ref token) = self.auth_token {
            rb = rb.bearer_auth(token);
        }
        for (k, v) in &self.headers {
            rb = rb.header(k, v);
        }

        let resp = rb.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("MCP HTTP request failed with status {}", resp.status());
        }

        let v: Value = resp.json().await?;
        if let Some(err) = v.get("error") {
            let code = err.get("code");
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
            anyhow::bail!("MCP HTTP error (code {:?}): {}", code, msg);
        }

        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }
}

#[allow(clippy::large_enum_variant)]
enum TransportState {
    Mock {
        tools: Arc<Mutex<Vec<Tool>>>,
    },
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        current_dir: Option<PathBuf>,
        process: Mutex<Option<StdioProcess>>,
    },
    Http {
        client: HttpClient,
    },
}

/// CustosMcpClient manages communication with an external or embedded MCP server.
pub struct CustosMcpClient {
    pub server_name: String,
    pub host_info: CustosMcpHostInfo,
    state: TransportState,
}

impl CustosMcpClient {
    /// Creates a mock MCP client (default for unit test harnesses).
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            host_info: CustosMcpHostInfo {
                explicit_extensions: true,
                client_name: Some("custos".into()),
                client_version: Some(env!("CARGO_PKG_VERSION").into()),
            },
            state: TransportState::Mock {
                tools: Arc::new(Mutex::new(Vec::new())),
            },
        }
    }

    /// Creates a real Stdio MCP client that communicates with a local child process.
    pub fn new_stdio(
        server_name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        env: HashMap<String, String>,
        current_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            server_name: server_name.into(),
            host_info: CustosMcpHostInfo {
                explicit_extensions: true,
                client_name: Some("custos".into()),
                client_version: Some(env!("CARGO_PKG_VERSION").into()),
            },
            state: TransportState::Stdio {
                command: command.into(),
                args,
                env,
                current_dir,
                process: Mutex::new(None),
            },
        }
    }

    /// Creates a real Streamable HTTP MCP client.
    pub fn new_http(
        server_name: impl Into<String>,
        url: impl Into<String>,
        auth_token: Option<String>,
        headers: HashMap<String, String>,
    ) -> Self {
        let url_str = url.into();
        Self {
            server_name: server_name.into(),
            host_info: CustosMcpHostInfo {
                explicit_extensions: true,
                client_name: Some("custos".into()),
                client_version: Some(env!("CARGO_PKG_VERSION").into()),
            },
            state: TransportState::Http {
                client: HttpClient::new(url_str, auth_token, headers),
            },
        }
    }

    pub async fn register_mock_tool(&self, tool: Tool) {
        if let TransportState::Mock { ref tools } = self.state {
            let mut list = tools.lock().await;
            list.push(tool);
        }
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        match &self.state {
            TransportState::Mock { .. } => {
                anyhow::bail!("send_request not supported directly on Mock transport")
            }
            TransportState::Http { client } => client.send_request(method, params).await,
            TransportState::Stdio {
                command,
                args,
                env,
                current_dir,
                process,
            } => {
                let mut guard = process.lock().await;
                if guard.is_none() {
                    let proc = StdioProcess::spawn(
                        command,
                        args,
                        env,
                        current_dir.as_ref(),
                        &self.host_info,
                    )
                    .await?;
                    *guard = Some(proc);
                }

                let proc = guard.as_mut().expect("process initialized");
                proc.send_request(method, params).await
            }
        }
    }
}

#[async_trait]
impl McpClientTrait for CustosMcpClient {
    async fn list_tools(&self, next_cursor: Option<String>) -> Result<Vec<Tool>> {
        match &self.state {
            TransportState::Mock { tools } => {
                let list = tools.lock().await;
                Ok(list.clone())
            }
            _ => {
                let mut all_tools = Vec::new();
                let mut current_cursor = next_cursor;
                loop {
                    let params = match &current_cursor {
                        Some(cursor) => serde_json::json!({ "cursor": cursor }),
                        None => serde_json::json!({}),
                    };
                    let res_val = self.send_request("tools/list", params).await?;
                    let tools_val = res_val
                        .get("tools")
                        .cloned()
                        .unwrap_or_else(|| Value::Array(Vec::new()));
                    let tools: Vec<Tool> = serde_json::from_value(tools_val)?;
                    all_tools.extend(tools);

                    let next = res_val
                        .get("nextCursor")
                        .or_else(|| res_val.get("next_cursor"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(n) = next {
                        if !n.is_empty() {
                            current_cursor = Some(n);
                            continue;
                        }
                    }
                    break;
                }
                Ok(all_tools)
            }
        }
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        // B6 / S8: Dangerous tool patterns require explicit grant & permit
        if crate::mcp::security::is_dangerous_tool_pattern(name) {
            tracing::warn!(
                server = %self.server_name,
                tool = %name,
                "Dangerous MCP tool pattern detected; explicit execution permit/grant required"
            );
            if ctx.permit_id.is_none() {
                anyhow::bail!(
                    "Execution of dangerous MCP tool '{}' blocked: requires explicit grant and permit",
                    name
                );
            }
        }

        tracing::info!(
            server = %self.server_name,
            tool = %name,
            session_id = %ctx.session_id,
            task_id = ?ctx.task_id,
            permit_id = ?ctx.permit_id,
            "Calling MCP tool under SADE governance"
        );

        match &self.state {
            TransportState::Mock { .. } => {
                let result_content = format!(
                    "Tool '{}' executed on server '{}' with args: {:?}",
                    name, self.server_name, arguments
                );
                Ok(CallToolResult::success(vec![
                    rmcp::model::ContentBlock::text(result_content),
                ]))
            }
            _ => {
                let mut params = serde_json::json!({ "name": name });
                if let Some(args) = arguments {
                    params["arguments"] = args;
                }
                let res_val = self.send_request("tools/call", params).await?;
                let result: CallToolResult = serde_json::from_value(res_val)?;
                Ok(result)
            }
        }
    }

    async fn list_resources(&self, next_cursor: Option<String>) -> Result<Vec<Value>> {
        match &self.state {
            TransportState::Mock { .. } => Ok(Vec::new()),
            _ => {
                let params = match next_cursor {
                    Some(cursor) => serde_json::json!({ "cursor": cursor }),
                    None => serde_json::json!({}),
                };
                let res_val = self.send_request("resources/list", params).await?;
                let list = res_val
                    .get("resources")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                Ok(list)
            }
        }
    }

    async fn read_resource(&self, uri: &str) -> Result<Value> {
        match &self.state {
            TransportState::Mock { .. } => {
                Ok(serde_json::json!({ "contents": [] }))
            }
            _ => {
                let params = serde_json::json!({ "uri": uri });
                self.send_request("resources/read", params).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_call_mock() {
        let client = CustosMcpClient::new("test-server");
        let ctx = ToolCallContext::new("sess-1", "req-1");

        let res = client
            .call_tool(
                &ctx,
                "read_file",
                Some(serde_json::json!({"path": "Cargo.toml"})),
            )
            .await
            .unwrap();

        assert!(!res.content.is_empty());
        let text = res.content[0].as_text().unwrap();
        assert!(text.text.contains("read_file"));
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let mut headers = HashMap::new();
        headers.insert("X-Custos-Trace".into(), "test-trace".into());
        let client = CustosMcpClient::new_http(
            "remote-server",
            "http://127.0.0.1:9099/mcp",
            Some("bearer-token-123".into()),
            headers,
        );
        assert_eq!(client.server_name, "remote-server");
    }

    #[tokio::test]
    async fn test_dangerous_tool_pattern_requires_permit() {
        let client = CustosMcpClient::new("test-server");

        // 1. Calling dangerous tool without permit must fail
        let unpermitted_ctx = ToolCallContext::new("sess-1", "req-1");
        let res_err = client.call_tool(&unpermitted_ctx, "bash_execute", None).await;
        assert!(res_err.is_err());
        assert!(res_err.unwrap_err().to_string().contains("requires explicit grant"));

        // 2. Calling dangerous tool with permit succeeds on mock
        let permitted_ctx = ToolCallContext::new("sess-1", "req-2")
            .with_sade_scope("task-1", "run-1", Some("pmt-1".into()));
        let res_ok = client.call_tool(&permitted_ctx, "bash_execute", None).await;
        assert!(res_ok.is_ok());
    }
}
