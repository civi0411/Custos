//! Tasks application use cases over the shared daemon context.

use super::*;
use crate::custos_local_api::{
    AdvanceTaskRequest, ApiResponse, CancelTaskRequest, CompleteTaskRequest, CreateTaskRequest,
};
use custos_core::{AdvanceTask, CancelTask, CompleteTask, CreateTask, TaskService};
use custos_domain::TaskStatus;

struct TaskApiHandler;

impl TaskApiHandler {
    async fn create_task(
        task_service: &TaskService,
        req_id: &str,
        params_val: serde_json::Value,
    ) -> ApiResponse {
        let params: CreateTaskRequest = match serde_json::from_value(params_val.clone()) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req_id, format!("Invalid params: {e}")),
        };

        let mut meta = params.metadata.unwrap_or_else(|| serde_json::json!({}));
        if let Some(sid) = params_val.get("session_id").and_then(|v| v.as_str()) {
            if meta.get("session_id").is_none() {
                meta["session_id"] = serde_json::json!(sid);
            }
        }

        let cmd = CreateTask {
            title: params.title,
            metadata: Some(meta),
            contract: params.contract,
        };

        match task_service.execute_create(cmd).await {
            Ok((task, _event)) => {
                let val = match serde_json::to_value(&task) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req_id, e.to_string()),
                };

                ApiResponse::success(req_id, val)
            }
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }

    async fn get_task(
        task_service: &TaskService,
        req_id: &str,
        params_val: &serde_json::Value,
    ) -> ApiResponse {
        let task_id = match params_val.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ApiResponse::error(req_id, "Missing task_id param"),
        };

        match task_service.get_task(task_id).await {
            Ok(Some(task)) => match serde_json::to_value(&task) {
                Ok(val) => ApiResponse::success(req_id, val),
                Err(e) => ApiResponse::error(req_id, e.to_string()),
            },
            Ok(None) => ApiResponse::error(req_id, format!("Task {task_id} not found")),
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }

    async fn cancel_task(
        task_service: &TaskService,
        req_id: &str,
        params_val: serde_json::Value,
    ) -> ApiResponse {
        let params: CancelTaskRequest = match serde_json::from_value(params_val) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req_id, format!("Invalid params: {e}")),
        };

        let task = match task_service.get_task(&params.task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ApiResponse::error(req_id, format!("Task {} not found", params.task_id));
            }
            Err(e) => return ApiResponse::error(req_id, e.to_string()),
        };

        let cmd = CancelTask {
            task_id: params.task_id,
            expected_epoch: task.epoch,
            reason: params
                .reason
                .unwrap_or_else(|| "User cancelled via API".into()),
        };

        match task_service.execute_cancel(cmd).await {
            Ok((task, _event)) => match serde_json::to_value(&task) {
                Ok(val) => ApiResponse::success(req_id, val),
                Err(e) => ApiResponse::error(req_id, e.to_string()),
            },
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }

    async fn advance_task(
        task_service: &TaskService,
        req_id: &str,
        params_val: serde_json::Value,
    ) -> ApiResponse {
        let params: AdvanceTaskRequest = match serde_json::from_value(params_val) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req_id, format!("Invalid params: {e}")),
        };

        if params.target_status == TaskStatus::Succeeded {
            return ApiResponse::error(
                req_id,
                "Cannot advance directly to Succeeded. Use v1.tasks.complete with verification proof.",
            );
        }

        let task = match task_service.get_task(&params.task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ApiResponse::error(req_id, format!("Task {} not found", params.task_id));
            }
            Err(e) => return ApiResponse::error(req_id, e.to_string()),
        };

        let cmd = AdvanceTask {
            task_id: params.task_id,
            expected_epoch: task.epoch,
            next_status: params.target_status,
            rationale: Some("Advance requested via Local API".into()),
        };

        match task_service.execute_advance(cmd).await {
            Ok((task, _event)) => match serde_json::to_value(&task) {
                Ok(val) => ApiResponse::success(req_id, val),
                Err(e) => ApiResponse::error(req_id, e.to_string()),
            },
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }

    async fn complete_task(
        task_service: &TaskService,
        req_id: &str,
        params_val: serde_json::Value,
    ) -> ApiResponse {
        let params: CompleteTaskRequest = match serde_json::from_value(params_val) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req_id, format!("Invalid params: {e}")),
        };

        let task = match task_service.get_task(&params.task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ApiResponse::error(req_id, format!("Task {} not found", params.task_id));
            }
            Err(e) => return ApiResponse::error(req_id, e.to_string()),
        };

        // Strict boundary check (F01 fix): all submitted claims must match target task_id and have valid metadata
        for claim in &params.evidence_claims {
            if claim.task_id != params.task_id {
                return ApiResponse::error(
                    req_id,
                    format!(
                        "Evidence claim '{}' belongs to foreign task '{}', not target task '{}'",
                        claim.id, claim.task_id, params.task_id
                    ),
                );
            }
            if claim.id.trim().is_empty() {
                return ApiResponse::error(req_id, "Evidence claim id cannot be empty");
            }
            if claim.verifier_id.trim().is_empty() {
                return ApiResponse::error(
                    req_id,
                    format!("Evidence claim '{}' missing verifier_id", claim.id),
                );
            }
        }

        let cmd = CompleteTask {
            task_id: params.task_id,
            expected_epoch: task.epoch,
            summary: params
                .summary
                .unwrap_or_else(|| "Completed via Local API".to_string()),
            evidence_claims: params.evidence_claims,
        };

        match task_service.execute_complete(cmd).await {
            Ok((task, _event)) => match serde_json::to_value(&task) {
                Ok(val) => ApiResponse::success(req_id, val),
                Err(e) => ApiResponse::error(req_id, e.to_string()),
            },
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }

    async fn list_tasks(
        task_service: &TaskService,
        req_id: &str,
        params_val: &serde_json::Value,
    ) -> ApiResponse {
        let session_id_opt = params_val.get("session_id").and_then(|v| v.as_str());
        match task_service.list_tasks().await {
            Ok(tasks) => {
                let filtered: Vec<_> = if let Some(sid) = session_id_opt {
                    tasks
                        .into_iter()
                        .filter(|t| {
                            t.metadata.get("session_id").and_then(|v| v.as_str()) == Some(sid)
                        })
                        .collect()
                } else {
                    tasks
                };
                match serde_json::to_value(&filtered) {
                    Ok(val) => ApiResponse::success(req_id, val),
                    Err(e) => ApiResponse::error(req_id, e.to_string()),
                }
            }
            Err(e) => ApiResponse::error(req_id, e.to_string()),
        }
    }
}

impl LocalApiDispatcher {
    pub(crate) async fn handle_tasks(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.tasks.create" => {
                TaskApiHandler::create_task(&self.kernel.task_service, &req.id, req.params).await
            }
            "v1.tasks.get" => {
                TaskApiHandler::get_task(&self.kernel.task_service, &req.id, &req.params).await
            }
            "v1.tasks.cancel" => {
                TaskApiHandler::cancel_task(&self.kernel.task_service, &req.id, req.params).await
            }
            "v1.tasks.advance" => {
                TaskApiHandler::advance_task(&self.kernel.task_service, &req.id, req.params).await
            }
            "v1.tasks.complete" => {
                TaskApiHandler::complete_task(&self.kernel.task_service, &req.id, req.params).await
            }
            "v1.tasks.list" => {
                TaskApiHandler::list_tasks(&self.kernel.task_service, &req.id, &req.params).await
            }
            "v1.tasks.spans" => {
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.kernel.task_service.list_spans(task_id).await {
                    Ok(spans) => match serde_json::to_value(&spans) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
