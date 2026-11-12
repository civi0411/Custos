//! Ring 2 interactive notebook cells and kernel session use cases.

use crate::api::*;
use custos_core::contracts::notebook::{
    CellExecutionStatus, ExecuteCellParams, KernelStatus, NotebookCell, NotebookCellType,
    NotebookStoragePort,
};

impl LocalApiDispatcher {
    pub(crate) async fn handle_shared_notebooks(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_NOTEBOOK_CELLS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let shared = match self.workbenches.shared_capabilities.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SharedCapabilitiesRepository not configured on daemon",
                        )
                    }
                };
                match shared.list_notebook_cells_scoped(session_id, notebook_id) {
                    Ok(cells) => match serde_json::to_value(&cells) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_SAVE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let mut cells: Vec<NotebookCell> = match req
                    .params
                    .get("cells")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(req.id, "Missing or invalid cells parameter")
                    }
                };
                if cells.iter().any(|cell| cell.session_id != session_id) {
                    return ApiResponse::error(
                        req.id,
                        "Notebook cell session_id must match the requested session_id",
                    );
                }
                let shared = match self.workbenches.shared_capabilities.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SharedCapabilitiesRepository not configured on daemon",
                        )
                    }
                };
                if let Some(kernel) = self.env.python_kernel.as_ref() {
                    match kernel
                        .get_status_scoped(
                            session_id,
                            notebook_id,
                            Some(shared.as_ref()
                                as &(dyn NotebookStoragePort
                                      + Send
                                      + Sync)),
                        )
                        .await
                    {
                        Ok(state) if state.status == KernelStatus::Busy => {
                            return ApiResponse::error(
                                req.id,
                                "Wait for notebook execution or interrupt it before changing cells",
                            )
                        }
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        _ => {}
                    }
                }
                let stored = match shared.list_notebook_cells_scoped(session_id, notebook_id) {
                    Ok(cells) => cells,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                for cell in &mut cells {
                    if let Some(nb) = notebook_id {
                        if cell.notebook_id.is_none() {
                            cell.notebook_id = Some(nb.to_string());
                        }
                    }
                    if let Some(previous) = stored.iter().find(|saved| saved.id == cell.id) {
                        if previous.source == cell.source && previous.cell_type == cell.cell_type {
                            cell.execution_count = previous.execution_count;
                            cell.status = previous.status;
                            cell.stdout = previous.stdout.clone();
                            cell.stderr = previous.stderr.clone();
                            cell.output_image = previous.output_image.clone();
                            cell.wall_ms = previous.wall_ms;
                            cell.cell_version = previous.cell_version.or(Some(1));
                        } else {
                            cell.execution_count = None;
                            cell.status = CellExecutionStatus::Idle;
                            cell.stdout = None;
                            cell.stderr = None;
                            cell.output_image = None;
                            cell.wall_ms = None;
                            cell.cell_version = Some(previous.cell_version.unwrap_or(1) + 1);
                        }
                        cell.epoch = previous.epoch;
                    } else {
                        cell.execution_count = None;
                        cell.status = CellExecutionStatus::Idle;
                        cell.stdout = None;
                        cell.stderr = None;
                        cell.output_image = None;
                        cell.wall_ms = None;
                        cell.epoch = 0;
                        cell.cell_version = Some(1);
                    }
                }
                match shared.replace_notebook_cells_scoped(session_id, notebook_id, &cells) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "count": cells.len() }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.env.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let mut params: ExecuteCellParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid execute cell params: {e}"),
                        )
                    }
                };
                let workspace_id = match params
                    .workspace_id
                    .as_deref()
                    .filter(|id| !id.trim().is_empty())
                {
                    Some(id) => id,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "Notebook execution requires a registered workspace_id",
                        )
                    }
                };
                let workspace = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(workspace) => workspace,
                    Err(response) => return response,
                };
                let shared = match self.workbenches.shared_capabilities.as_ref() {
                    Some(repository) => repository,
                    None => return ApiResponse::error(req.id, "SharedCapabilitiesRepository not configured on daemon"),
                };
                let saved = match shared
                    .list_notebook_cells_scoped(&params.session_id, params.notebook_id.as_deref())
                {
                    Ok(cells) => cells,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match saved.iter().find(|cell| cell.id == params.cell_id) {
                    Some(cell)
                        if cell.source == params.code
                            && cell.cell_type == NotebookCellType::Code =>
                    {
                        if let (Some(expected), Some(actual)) =
                            (params.cell_version, cell.cell_version)
                        {
                            if expected != actual {
                                return ApiResponse::error(
                                    req.id,
                                    format!(
                                        "Cell version mismatch: requested {expected}, but saved version is {actual}"
                                    ),
                                );
                            }
                        }
                        if let (Some(ref req_nb), Some(ref saved_nb)) =
                            (&params.notebook_id, &cell.notebook_id)
                        {
                            if req_nb != saved_nb {
                                return ApiResponse::error(
                                    req.id,
                                    format!(
                                        "Notebook mismatch: cell belongs to {saved_nb}, not {req_nb}"
                                    ),
                                );
                            }
                        }
                    }
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Save the exact code cell in this session before executing it",
                        )
                    }
                }
                // cwd is resolved from the registered workspace, never from daemon process state.
                if params
                    .cwd
                    .as_deref()
                    .is_some_and(|cwd| cwd != workspace.path)
                {
                    return ApiResponse::error(
                        req.id,
                        "Notebook cwd must match the registered workspace",
                    );
                }
                params.cwd = Some(workspace.path);

                let repo_ref = self.workbenches.shared_capabilities.as_deref().map(|s| {
                    s as &(dyn NotebookStoragePort + Send + Sync)
                });
                match kernel.execute_cell(params, repo_ref).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_INTERRUPT => {
                let kernel = match self.env.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                match kernel.interrupt_scoped(session_id, notebook_id).await {
                    Ok(interrupted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "interrupted": interrupted }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_RESET => {
                let kernel = match self.env.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.workbenches.shared_capabilities.as_deref().map(|s| {
                    s as &(dyn NotebookStoragePort + Send + Sync)
                });
                match kernel.reset_scoped(session_id, notebook_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_STATUS => {
                let kernel = match self.env.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.workbenches.shared_capabilities.as_deref().map(|s| {
                    s as &(dyn NotebookStoragePort + Send + Sync)
                });
                match kernel
                    .get_status_scoped(session_id, notebook_id, repo_ref)
                    .await
                {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(
                req.id,
                format!("Unknown research notebook method: {unknown}"),
            ),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_core::contracts::notebook::{NotebookCell, NotebookCellType};
    use custos_persistence::SqliteTaskStore;

    #[tokio::test]
    async fn notebook_group_test_without_dispatcher() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.shared_capabilities();

        let session_id = "test-session-nb";
        let initial = repo.list_notebook_cells(session_id).unwrap();
        assert_eq!(initial.len(), 0);

        let cell = NotebookCell {
            id: "cell-1".to_string(),
            session_id: session_id.to_string(),
            cell_type: NotebookCellType::Code,
            source: "print('hello')".to_string(),
            status: CellExecutionStatus::Idle,
            ..Default::default()
        };

        repo.replace_notebook_cells(session_id, &[cell]).unwrap();
        let updated = repo.list_notebook_cells(session_id).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].source, "print('hello')");
    }
}
