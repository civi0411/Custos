//! Custos Local API Contracts & Client
//!
//! Canonical protocol types for IPC communication between CLI / UI / VS Code and custos-daemon.

use async_trait::async_trait;
use custos_domain::{
    ExecutionWorkspace, Session, SessionJournalEntry, Task, TaskContract, TaskStatus,
    VerificationClaim, WorkspaceKind, WorkspaceLineage,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const METHOD_TASKS_CREATE: &str = "v1.tasks.create";
pub const METHOD_TASKS_GET: &str = "v1.tasks.get";
pub const METHOD_TASKS_LIST: &str = "v1.tasks.list";
pub const METHOD_TASKS_SPANS: &str = "v1.tasks.spans";
pub const METHOD_TASKS_CANCEL: &str = "v1.tasks.cancel";
pub const METHOD_TASKS_ADVANCE: &str = "v1.tasks.advance";
pub const METHOD_TASKS_COMPLETE: &str = "v1.tasks.complete";
pub const METHOD_SESSIONS_CREATE: &str = "v1.sessions.create";
pub const METHOD_SESSIONS_GET: &str = "v1.sessions.get";
pub const METHOD_SESSIONS_LIST: &str = "v1.sessions.list";
pub const METHOD_SESSIONS_JOURNAL: &str = "v1.sessions.journal";
pub const METHOD_SESSIONS_MESSAGE: &str = "v1.sessions.message";
pub const METHOD_SESSIONS_PROMOTE: &str = "v1.sessions.promote";
pub const METHOD_BRIDGE_ATTACH: &str = "v1.bridge.attach";
pub const METHOD_BRIDGE_STEER: &str = "v1.bridge.steer";
pub const METHOD_OI_EXPLAIN: &str = "v1.oi.explain";
pub const METHOD_WORKFLOW_START_RUN: &str = "v1.workflow.start_run";
pub const METHOD_WORKFLOW_CANCEL_RUN: &str = "v1.workflow.cancel_run";
pub const METHOD_WORKSPACES_CREATE: &str = "v1.workspaces.create";
pub const METHOD_WORKSPACES_GET: &str = "v1.workspaces.get";
pub const METHOD_WORKSPACES_LIST: &str = "v1.workspaces.list";
pub const METHOD_WORKSPACES_ARCHIVE: &str = "v1.workspaces.archive";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ApiRequest {
    pub fn new(
        id: impl Into<String>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

impl ApiResponse {
    pub fn ok(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self::ok(id, result)
    }

    pub fn error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(error.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }
}

// Request and Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub contract: Option<TaskContract>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskRequest {
    pub task_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceTaskRequest {
    pub task_id: String,
    pub target_status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub summary: Option<String>,
    #[serde(default)]
    pub evidence_claims: Vec<VerificationClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGetRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAttachRequest {
    pub session_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSteerRequest {
    pub session_id: String,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPlanRequest {
    pub title: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub contract: Option<TaskContract>,
    #[serde(default)]
    pub estimated_files_count: Option<u32>,
    #[serde(default)]
    pub estimated_complexity: Option<u32>,
    #[serde(default)]
    pub budget_limit_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunRequest {
    pub task_id: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    #[serde(default)]
    pub preferred_mode: Option<String>,
    #[serde(default)]
    pub harness_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRunRequest {
    pub run_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceApiRequest {
    pub name: String,
    pub kind: WorkspaceKind,
    pub path: String,
    #[serde(default)]
    pub lineage: Option<WorkspaceLineage>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub setup_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWorkspaceApiRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveWorkspaceApiRequest {
    pub workspace_id: String,
    #[serde(default)]
    pub delete_physical: bool,
}

/// Abstract transport for communicating with the Custos Daemon
#[async_trait]
pub trait ApiTransport: Send + Sync {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String>;
}

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
        }
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

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(&encoded).await.map_err(|e| e.to_string())?;
            stdin.flush().await.map_err(|e| e.to_string())?;
        }

        let mut lines = self.lines.lock().await;
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
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
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stream = tokio::net::TcpStream::connect(&self.addr)
            .await
            .map_err(|e| format!("Failed to connect to daemon at {}: {e}", self.addr))?;
        let (reader, mut writer) = stream.into_split();

        let mut encoded = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        encoded.push(b'\n');

        writer.write_all(&encoded).await.map_err(|e| e.to_string())?;
        writer.flush().await.map_err(|e| e.to_string())?;

        let mut lines = BufReader::new(reader).lines();
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str::<ApiResponse>(&line)
                .map_err(|e| format!("Invalid JSON response: {e}"));
        }
        Err("Daemon closed TCP connection without response".to_string())
    }
}

/// Strongly-typed client for the Custos Local API
#[derive(Clone)]
pub struct LocalApiClient {
    transport: Arc<dyn ApiTransport>,
}

impl LocalApiClient {
    pub fn new(transport: Box<dyn ApiTransport>) -> Self {
        Self {
            transport: Arc::from(transport),
        }
    }

    pub fn with_arc_transport(transport: Arc<dyn ApiTransport>) -> Self {
        Self { transport }
    }

    pub fn from_tcp(addr: impl Into<String>) -> Self {
        Self::with_arc_transport(Arc::new(TcpTransport::new(addr)))
    }

    pub fn from_profile(profile: &crate::profile::ProfileResolver) -> Result<Self, String> {
        let port = profile
            .read_port()
            .ok_or_else(|| "Daemon port file not found or daemon not running".to_string())?;
        Ok(Self::from_tcp(format!("127.0.0.1:{port}")))
    }

    pub async fn ping(&self) -> Result<(), String> {
        let req = ApiRequest::new("ping", "v1.ping", serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        self.transport.send_request(req).await
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let req: ApiRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = ApiResponse::error("", format!("Invalid JSON request: {e}"));
                return serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string());
            }
        };
        match self.transport.send_request(req).await {
            Ok(resp) => serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
            Err(err) => {
                let err_resp = ApiResponse::error("", err);
                serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }


    pub async fn create_task(
        &self,
        req_id: &str,
        title: &str,
        contract: Option<TaskContract>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CreateTaskRequest {
            title: title.to_string(),
            contract,
            metadata,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn get_task(&self, req_id: &str, task_id: &str) -> Result<Task, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn cancel_task(
        &self,
        req_id: &str,
        task_id: &str,
        reason: Option<String>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CancelTaskRequest {
            task_id: task_id.to_string(),
            reason,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CANCEL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn advance_task(
        &self,
        req_id: &str,
        task_id: &str,
        target_status: TaskStatus,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(AdvanceTaskRequest {
            task_id: task_id.to_string(),
            target_status,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_ADVANCE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn complete_task(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
    ) -> Result<Task, String> {
        self.complete_task_with_evidence(req_id, task_id, summary, Vec::new())
            .await
    }

    pub async fn complete_task_with_evidence(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
        evidence_claims: Vec<VerificationClaim>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CompleteTaskRequest {
            task_id: task_id.to_string(),
            summary,
            evidence_claims,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_COMPLETE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn list_tasks(&self, req_id: &str) -> Result<Vec<Task>, String> {
        let req = ApiRequest::new(req_id, METHOD_TASKS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse tasks list: {e}"))
    }

    pub async fn list_spans(
        &self,
        req_id: &str,
        task_id: &str,
    ) -> Result<Vec<custos_domain::Span>, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_SPANS, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse spans list: {e}"))
    }

    pub async fn create_session(
        &self,
        req_id: &str,
        mode: Option<&str>,
    ) -> Result<Session, String> {
        let params = serde_json::json!({
            "mode": mode.unwrap_or("bare")
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn get_session(&self, req_id: &str, session_id: &str) -> Result<Session, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn list_sessions(&self, req_id: &str) -> Result<Vec<Session>, String> {
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse sessions list: {e}"))
    }

    pub async fn append_session_message(
        &self,
        req_id: &str,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "role": role,
            "content": content,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_MESSAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn get_session_journal(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<SessionJournalEntry>, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_JOURNAL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        #[derive(Deserialize)]
        struct JournalWrapper {
            entries: Vec<SessionJournalEntry>,
        }
        if let Ok(wrapper) = serde_json::from_value::<JournalWrapper>(result.clone()) {
            return Ok(wrapper.entries);
        }
        serde_json::from_value(result).map_err(|e| format!("Failed to parse journal entries: {e}"))
    }

    pub async fn attach_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_ATTACH, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn steer_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
        message: &str,
    ) -> Result<serde_json::Value, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
            "message": message,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_STEER, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        resp.result.ok_or_else(|| "Empty result in response".into())
    }

    pub async fn promote_session(
        &self,
        req_id: &str,
        session_id: &str,
        contract: TaskContract,
    ) -> Result<Task, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "contract": contract,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_PROMOTE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn explain_plan(
        &self,
        req_id: &str,
        request: ExplainPlanRequest,
    ) -> Result<custos_runtime::oi::ExplainReport, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_OI_EXPLAIN, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExplainReport: {e}"))
    }

    pub async fn create_workspace(
        &self,
        req_id: &str,
        req: CreateWorkspaceApiRequest,
    ) -> Result<ExecutionWorkspace, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
    }

    pub async fn get_workspace(
        &self,
        req_id: &str,
        workspace_id: &str,
    ) -> Result<ExecutionWorkspace, String> {
        let params = serde_json::json!({ "workspace_id": workspace_id });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
    }

    pub async fn list_workspaces(
        &self,
        req_id: &str,
    ) -> Result<Vec<ExecutionWorkspace>, String> {
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Vec<ExecutionWorkspace>: {e}"))
    }

    pub async fn archive_workspace(
        &self,
        req_id: &str,
        workspace_id: &str,
        delete_physical: bool,
    ) -> Result<(), String> {
        let params = serde_json::to_value(ArchiveWorkspaceApiRequest {
            workspace_id: workspace_id.to_string(),
            delete_physical,
        })
        .map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_ARCHIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport;

    #[async_trait]
    impl ApiTransport for MockTransport {
        async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
            match req.method.as_str() {
                METHOD_TASKS_CREATE => {
                    let task = Task::new(custos_domain::new_id("task"), "Mock Task".to_string());
                    Ok(ApiResponse::ok(
                        req.id,
                        serde_json::to_value(&task).unwrap(),
                    ))
                }
                _ => Ok(ApiResponse::error(req.id, "Unsupported method")),
            }
        }
    }

    #[tokio::test]
    async fn test_client_create_task() {
        let client = LocalApiClient::new(Box::new(MockTransport));
        let task = client
            .create_task("1", "Mock Task", None, None)
            .await
            .unwrap();
        assert_eq!(task.title, "Mock Task");
        assert_eq!(task.status, TaskStatus::Draft);
    }

    #[tokio::test]
    async fn test_tcp_transport_roundtrip() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();
                if let Ok(Some(line)) = lines.next_line().await {
                    let req: ApiRequest = serde_json::from_str(&line).unwrap();
                    assert_eq!(req.method, "v1.ping");
                    let resp = ApiResponse::success(req.id, serde_json::json!({ "status": "pong" }));
                    let mut data = serde_json::to_vec(&resp).unwrap();
                    data.push(b'\n');
                    let _ = writer.write_all(&data).await;
                    let _ = writer.flush().await;
                }
            }
        });

        let client = LocalApiClient::from_tcp(format!("127.0.0.1:{port}"));
        let req = ApiRequest::new("ping_1", "v1.ping", serde_json::json!({}));
        let resp = client.send_request(req).await.unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.result.unwrap(), serde_json::json!({ "status": "pong" }));
    }
}

