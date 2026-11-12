//! Candidate race execution use case.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_candidates_race(&self, req: ApiRequest) -> ApiResponse {
        let workflow = match self.kernel.workflow.as_ref() {
            Some(w) => w,
            None => {
                return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
            }
        };
        let coordinator = match self.env.workspace.as_ref() {
            Some(c) => c,
            None => {
                return ApiResponse::error(
                    req.id,
                    "WorkspaceCoordinator not configured on daemon",
                )
            }
        };

        let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
        };

        match self.kernel.task_service.get_task(&task_id).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return ApiResponse::error(req.id, format!("Task '{task_id}' not found"))
            }
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        }

        let base_workspace_id = req
            .params
            .get("base_workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let test_command = req
            .params
            .get("test_command")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let timeout_seconds = req
            .params
            .get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(600);

        let candidates: Vec<custos_domain::ModelCandidateConfig> = if let Some(raw_cands) =
            req.params.get("candidates").and_then(|v| v.as_array())
        {
            match serde_json::from_value(serde_json::Value::Array(raw_cands.clone())) {
                Ok(c) => c,
                Err(e) => {
                    return ApiResponse::error(
                        req.id,
                        format!("Invalid candidates config: {e}"),
                    )
                }
            }
        } else {
            let all_ws = match coordinator.list_workspaces().await {
                Ok(list) => list,
                Err(e) => return ApiResponse::error(req.id, e.to_string()),
            };
            let mut task_cands: Vec<_> = all_ws
                .into_iter()
                .filter(|ws| ws.owner_task_id.as_deref() == Some(&task_id) && ws.is_ready())
                .collect();
            task_cands.sort_by_key(|w| w.name.clone());

            if task_cands.is_empty() {
                return ApiResponse::error(
                    req.id,
                    format!("No ready candidate worktrees found for task '{task_id}'. Spawn candidates first."),
                );
            }

            let default_models = [
                ("claude-3-5-sonnet", "claude_code"),
                ("gpt-4o", "generic_cli"),
                ("deepseek-coder", "generic_cli"),
            ];

            task_cands
                .into_iter()
                .enumerate()
                .map(|(idx, ws)| {
                    let (model_id, harness_id) = default_models[idx % default_models.len()];
                    let cand_id = ws
                        .metadata
                        .get("candidate_index")
                        .and_then(|v| v.as_u64())
                        .map(|i| format!("cand_{i}"))
                        .unwrap_or_else(|| ws.id.to_string());
                    custos_domain::ModelCandidateConfig::new(cand_id, model_id, harness_id)
                        .with_workspace_id(ws.id.to_string())
                })
                .collect()
        };

        let mut spec =
            custos_domain::CandidateRaceSpec::new(&task_id, &base_workspace_id, candidates)
                .with_timeout_seconds(timeout_seconds);

        if let Some(cmd) = test_command {
            spec = spec.with_test_command(cmd);
        }

        match workflow.execute_candidate_race(spec).await {
            Ok(result) => ApiResponse::success(
                req.id,
                serde_json::to_value(&result).unwrap_or_default(),
            ),
            Err(e) => ApiResponse::error(req.id, format!("Candidate race failed: {e}")),
        }
    }
}
