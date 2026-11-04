use super::*;

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

    pub fn from_port(port: u16) -> Self {
        Self::from_tcp(format!("127.0.0.1:{port}"))
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
            Ok(resp) => {
                serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
            }
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

    pub async fn delete_session(&self, req_id: &str, session_id: &str) -> Result<bool, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_DELETE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        Ok(result
            .get("deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
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
    ) -> Result<custos_domain::oi::ExplainReport, String> {
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
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
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
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse ExecutionWorkspace: {e}"))
    }

    pub async fn list_workspaces(&self, req_id: &str) -> Result<Vec<ExecutionWorkspace>, String> {
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse Vec<ExecutionWorkspace>: {e}"))
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
            force: false,
        })
        .map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_WORKSPACES_ARCHIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn get_workspace_file_tree(
        &self,
        req_id: &str,
        workspace_id: &str,
        relative_dir: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<WorkspaceFileTree, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "relative_dir": relative_dir,
            "max_depth": max_depth,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_TREE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse WorkspaceFileTree: {e}"))
    }

    pub async fn read_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        max_bytes: Option<usize>,
    ) -> Result<WorkspaceFileContent, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "max_bytes": max_bytes,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_READ, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse WorkspaceFileContent: {e}"))
    }

    pub async fn write_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        content: &str,
        create_parents: Option<bool>,
        overwrite: Option<bool>,
    ) -> Result<WorkspaceFileContent, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "content": content,
            "create_parents": create_parents.unwrap_or(true),
            "overwrite": overwrite.unwrap_or(true),
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILES_WRITE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse WorkspaceFileContent: {e}"))
    }

    pub async fn get_workspace_diff(
        &self,
        req_id: &str,
        workspace_id: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceDiffSummary, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "staged": staged,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_DIFF, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse WorkspaceDiffSummary: {e}"))
    }

    pub async fn get_workspace_file_diff(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
        staged: Option<bool>,
    ) -> Result<WorkspaceFileDiff, String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
            "staged": staged,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_FILE_DIFF, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse WorkspaceFileDiff: {e}"))
    }

    pub async fn stage_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_STAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn unstage_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_UNSTAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn discard_workspace_file(
        &self,
        req_id: &str,
        workspace_id: &str,
        path: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "workspace_id": workspace_id,
            "path": path,
        });
        let req = ApiRequest::new(req_id, METHOD_WORKSPACE_GIT_DISCARD, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        Ok(())
    }

    pub async fn list_harnesses(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_core::contracts::harness::HarnessDescriptor>, String> {
        let req = ApiRequest::new(req_id, METHOD_HARNESS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse harnesses list: {e}"))
    }

    pub async fn get_harness(
        &self,
        req_id: &str,
        harness_id: &str,
    ) -> Result<custos_core::contracts::harness::HarnessDescriptor, String> {
        let params = serde_json::json!({ "harness_id": harness_id });
        let req = ApiRequest::new(req_id, METHOD_HARNESS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse harness descriptor: {e}"))
    }

    pub async fn run_native_harness(
        &self,
        req_id: &str,
        request: RunNativeHarnessRequest,
    ) -> Result<custos_core::contracts::harness::HarnessExecutionResult, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_HARNESS_RUN_NATIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse run native result: {e}"))
    }

    pub async fn list_artifacts(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_domain::ArtifactSummary>, String> {
        let req = ApiRequest::new(req_id, METHOD_ARTIFACTS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact list: {e}"))
    }

    pub async fn get_artifact(
        &self,
        req_id: &str,
        path: &str,
    ) -> Result<ArtifactDetailResponse, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_ARTIFACTS_GET,
            serde_json::json!({ "artifact_path": path }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact detail: {e}"))
    }

    pub async fn get_artifact_lineage_graph(
        &self,
        req_id: &str,
        filter_path: Option<&str>,
    ) -> Result<custos_domain::ArtifactLineageGraph, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_ARTIFACTS_LINEAGE_GRAPH,
            serde_json::json!({ "artifact_path": filter_path }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse lineage graph: {e}"))
    }

    pub async fn list_notes(
        &self,
        req_id: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<custos_domain::NoteRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_LIST,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notes list: {e}"))
    }

    pub async fn get_note(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_domain::NoteRecord>, String> {
        let req = ApiRequest::new(req_id, METHOD_NOTES_GET, serde_json::json!({ "id": id }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note: {e}"))
    }

    pub async fn save_note(
        &self,
        req_id: &str,
        params: SaveNoteParams,
    ) -> Result<custos_domain::NoteRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTES_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved note: {e}"))
    }

    pub async fn list_note_versions(
        &self,
        req_id: &str,
        note_id: &str,
    ) -> Result<Vec<custos_domain::NoteVersionRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_HISTORY,
            serde_json::json!({ "note_id": note_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note versions: {e}"))
    }

    pub async fn list_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, String> {
        self.list_notebook_cells_scoped(req_id, session_id, None)
            .await
    }

    pub async fn list_notebook_cells_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_CELLS_LIST, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notebook cells: {e}"))
    }

    pub async fn save_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
        cells: Vec<NotebookCell>,
    ) -> Result<(), String> {
        self.save_notebook_cells_scoped(req_id, session_id, None, cells)
            .await
    }

    pub async fn save_notebook_cells_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: Vec<NotebookCell>,
    ) -> Result<(), String> {
        let mut params = serde_json::json!({ "session_id": session_id, "cells": cells });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_CELLS_SAVE, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn execute_notebook_cell(
        &self,
        req_id: &str,
        params: ExecuteCellParams,
    ) -> Result<ExecuteCellResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse execution result: {e}"))
    }

    pub async fn interrupt_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<(), String> {
        self.interrupt_notebook_kernel_scoped(req_id, session_id, None)
            .await
    }

    pub async fn interrupt_notebook_kernel_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<(), String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_INTERRUPT, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn reset_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        self.reset_notebook_kernel_scoped(req_id, session_id, None)
            .await
    }

    pub async fn reset_notebook_kernel_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_RESET, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn get_notebook_kernel_status(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        self.get_notebook_kernel_status_scoped(req_id, session_id, None)
            .await
    }

    pub async fn get_notebook_kernel_status_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_STATUS, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn list_reviews(
        &self,
        req_id: &str,
        target_type: Option<ReviewTargetType>,
        target_id: Option<&str>,
    ) -> Result<Vec<ReviewerRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_LIST,
            serde_json::json!({
                "target_type": target_type,
                "target_id": target_id,
            }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse reviews: {e}"))
    }

    pub async fn get_review(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<ReviewerRecord>, String> {
        let req = ApiRequest::new(req_id, METHOD_REVIEWS_GET, serde_json::json!({ "id": id }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse review: {e}"))
    }

    pub async fn record_review(
        &self,
        req_id: &str,
        params: RecordReviewParams,
    ) -> Result<ReviewerRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_REVIEWS_RECORD, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse recorded review: {e}"))
    }

    pub async fn mark_review_stale(&self, req_id: &str, id: &str) -> Result<(), String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_MARK_STALE,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn list_synthesis_proposals(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_domain::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_SYNTHESIS_PROPOSALS_LIST,
            serde_json::json!({}),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposals: {e}"))
    }

    pub async fn get_synthesis_proposal(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_domain::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_SYNTHESIS_PROPOSALS_GET,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposal: {e}"))
    }

    pub async fn save_synthesis_proposal(
        &self,
        req_id: &str,
        params: custos_domain::SaveSynthesisProposalParams,
    ) -> Result<custos_domain::ResearchSynthesisProposal, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_PROPOSALS_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved proposal: {e}"))
    }

    pub async fn execute_synthesis_handoff(
        &self,
        req_id: &str,
        params: custos_domain::HandoffToCodingParams,
    ) -> Result<custos_domain::HandoffToCodingResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_HANDOFF_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse handoff result: {e}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetailResponse {
    pub artifact_path: String,
    pub latest_version: u32,
    pub latest_content_hash: String,
    pub versions: Vec<custos_domain::ArtifactLineageNode>,
    pub annotations: Vec<custos_domain::AnnotationRecord>,
    pub content: Option<String>,
    /// Current workspace bytes, not an archived version snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<custos_domain::WorkspaceFileContent>,
    /// Present only when current bytes match the recorded version hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNoteParams {
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

