//! Research notebook cells and experiment runs use cases.

use crate::api::*;
use custos_domain::{
    CellExecutionStatus, NotebookCell, ResearchExperimentRun,
};

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research_notebooks(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_RUNS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_runs() {
                    Ok(runs) => match serde_json::to_value(&runs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let run: ResearchExperimentRun = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid run payload: {e}"))
                    }
                };
                let run = match custos_core::research_ingress::experiment_draft(run) {
                    Ok(run) => run,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_runs() {
                    Ok(runs) if runs.iter().any(|existing| existing.run_id == run.run_id) => {
                        return ApiResponse::error(req.id, "Experiment draft id already exists");
                    }
                    Ok(_) => {}
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_run(&run) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "run_id": run.run_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_notebook_cells(session_id) {
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
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                if let Some(kernel) = self.python_kernel.as_ref() {
                    let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                    match kernel
                        .get_status_scoped(
                            session_id,
                            notebook_id,
                            Some(research.as_ref()
                                as &(dyn custos_core::contracts::storage::NotebookStoragePort
                                      + Send
                                      + Sync)),
                        )
                        .await
                    {
                        Ok(state) if state.status == custos_domain::KernelStatus::Busy => {
                            return ApiResponse::error(
                                req.id,
                                "Wait for notebook execution or interrupt it before changing cells",
                            )
                        }
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        _ => {}
                    }
                }
                let stored = match research.list_notebook_cells(session_id) {
                    Ok(cells) => cells,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                for cell in &mut cells {
                    if let Some(previous) = stored.iter().find(|saved| saved.id == cell.id) {
                        if previous.source == cell.source && previous.cell_type == cell.cell_type {
                            cell.execution_count = previous.execution_count;
                            cell.status = previous.status;
                            cell.stdout = previous.stdout.clone();
                            cell.stderr = previous.stderr.clone();
                            cell.output_image = previous.output_image.clone();
                            cell.wall_ms = previous.wall_ms;
                        } else {
                            cell.execution_count = None;
                            cell.status = CellExecutionStatus::Idle;
                            cell.stdout = None;
                            cell.stderr = None;
                            cell.output_image = None;
                            cell.wall_ms = None;
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
                    }
                }
                match research.replace_notebook_cells(session_id, &cells) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "count": cells.len() }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.python_kernel.as_ref() {
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
                let research = match self.research.as_ref() {
                    Some(repository) => repository,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let saved = match research.list_notebook_cells(&params.session_id) {
                    Ok(cells) => cells,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match saved.iter().find(|cell| cell.id == params.cell_id) {
                    Some(cell)
                        if cell.source == params.code
                            && cell.cell_type == custos_domain::NotebookCellType::Code => {}
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

                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
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
                let kernel = match self.python_kernel.as_ref() {
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
                let kernel = match self.python_kernel.as_ref() {
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
                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
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
                let kernel = match self.python_kernel.as_ref() {
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
                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
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
            "v1.research.executions.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.list_execution_records_for_recipe(recipe_id) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.executions.get" | "v1.research.executions.resume.assess" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let execution_id = match req.params.get("execution_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing execution_id parameter"),
                };
                match research.get_execution_record(execution_id) {
                    Ok(Some(record)) => {
                        match if req.method == "v1.research.executions.resume.assess" {
                            serde_json::to_value(record.assess_checkpoint_resume())
                        } else {
                            serde_json::to_value(&record)
                        } {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("Execution record {execution_id} not found"),
                    ),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown research notebook method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_domain::{NotebookCell, NotebookCellType};
    use custos_persistence::SqliteTaskStore;

    #[tokio::test]
    async fn notebook_group_test_without_dispatcher() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.research();

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
