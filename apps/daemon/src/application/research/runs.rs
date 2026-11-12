//! Research notebook cells and experiment runs use cases.

use crate::api::*;
use custos_workbench_research::ResearchExperimentRun;

impl LocalApiDispatcher {
    pub(crate) async fn handle_research_runs(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_RUNS_LIST => {
                let research = match self.research_repo() {
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
                let research = match self.research_repo() {
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
                let run = match custos_workbench_research::experiment_draft(run) {
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
            "v1.research.executions.list" => {
                let research = match self.research_repo() {
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
                let research = match self.research_repo() {
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
            unknown => ApiResponse::error(
                req.id,
                format!("Unknown research notebook method: {unknown}"),
            ),
        }
    }
}

