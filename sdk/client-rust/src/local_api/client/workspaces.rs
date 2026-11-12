use super::LocalApiClient;
use crate::local_api::envelope::ApiRequest;
use crate::local_api::methods::*;
use crate::local_api::params::*;
use custos_domain::{
    ExecutionWorkspace, WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff,
    WorkspaceFileTree,
};

impl LocalApiClient {
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
}
