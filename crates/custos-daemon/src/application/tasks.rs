//! tasks application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_tasks(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.tasks.create" => {
                tasks::TaskApiHandler::create_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.get" => {
                tasks::TaskApiHandler::get_task(&self.task_service, &req.id, &req.params).await
            }
            "v1.tasks.cancel" => {
                tasks::TaskApiHandler::cancel_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.advance" => {
                tasks::TaskApiHandler::advance_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.complete" => {
                tasks::TaskApiHandler::complete_task(&self.task_service, &req.id, req.params).await
            }
            "v1.tasks.list" => {
                tasks::TaskApiHandler::list_tasks(&self.task_service, &req.id, &req.params).await
            }
            "v1.tasks.spans" => {
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing task_id param"),
                };

                match self.task_service.list_spans(task_id).await {
                    Ok(spans) => match serde_json::to_value(&spans) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.oi.explain" => {
                let params: crate::custos_local_api::ExplainPlanRequest =
                    match serde_json::from_value(req.params) {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid params: {e}"))
                        }
                    };

                let task_id = params
                    .task_id
                    .unwrap_or_else(|| custos_domain::new_id("task"));
                let mut snapshot = custos_domain::oi::DecisionSnapshot::new(&task_id);
                if let Some(budget) = params.budget_limit_tokens {
                    snapshot.remaining_budget_tokens = budget;
                }
                if let Some(ref contract) = params.contract {
                    snapshot.required_capabilities = contract.required_capabilities.clone();
                }

                match custos_runtime::oi::ExplainService::explain(&snapshot) {
                    Ok(report) => match serde_json::to_value(&report) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.workflow.session_runs" => {
                let Some(session) = req.params.get("session_id").and_then(|v| v.as_str()) else {
                    return ApiResponse::error(req.id, "Missing session_id");
                };
                if self
                    .session_manager
                    .get_session(&SessionId(session.to_string()))
                    .await
                    .is_none()
                {
                    return ApiResponse::error(req.id, "Unknown session");
                }
                let Some(store) = &self.usage_runs else {
                    return ApiResponse::error(req.id, "Run store unavailable");
                };
                let tasks = match self.task_service.list_tasks().await {
                    Ok(tasks) => tasks,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut runs = Vec::new();
                for task in tasks {
                    let task_runs = match store.list_runs_for_task(&task.id).await {
                        Ok(runs) => runs,
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    };
                    for run in task_runs {
                        if run.metadata.get("session_id").and_then(|v| v.as_str()) == Some(session)
                            && (run.metadata.get("background").and_then(|v| v.as_bool())
                                == Some(true)
                                || run.metadata.get("journal_occurred_at").is_some()
                                || !run.status.is_terminal())
                        {
                            runs.push(serde_json::json!({"run_id":run.id,"model":run.metadata.get("requested_model").filter(|value|!value.is_null()).or_else(||run.metadata.get("harness_id")),"status":run.status.to_string(),"journal_occurred_at":run.metadata.get("journal_occurred_at")}));
                        }
                    }
                }
                ApiResponse::success(req.id, serde_json::json!(runs))
            }
            "v1.workflow.live_run" => {
                let session = req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let run = req
                    .params
                    .get("run_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let cursor = req
                    .params
                    .get("cursor")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                match self.live_run(session, run, cursor).await {
                    Ok(replay) => match serde_json::to_value(replay) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_WORKFLOW_START_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: StartRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };

                if let Some(ref sid) = params.session_id {
                    let sess_opt = self
                        .session_manager
                        .get_session(&SessionId(sid.to_string()))
                        .await;
                    if sess_opt.is_none() {
                        return ApiResponse::error(
                            req.id,
                            format!("Admission gate refused unverified session_id '{sid}': session does not exist"),
                        );
                    }
                }

                let actor = match params.actor {
                    Some(ref a) if !a.trim().is_empty() => a.trim().to_string(),
                    _ => "daemon_user".into(),
                };

                let resolved_workspace_root = if let Some(ref workspace_id) = params.workspace_id {
                    let workspace = match self
                        .resolve_workspace_for_dispatch(&req.id, workspace_id)
                        .await
                    {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    if !workspace.is_ready() {
                        return ApiResponse::error(
                            req.id,
                            format!("Workspace '{workspace_id}' is not ready for execution"),
                        );
                    }
                    Some(workspace.path)
                } else if params.preferred_mode.as_deref() == Some("native") {
                    return ApiResponse::error(
                        req.id,
                        "Native Task runs require workspace_id; raw workspace paths are not accepted",
                    );
                } else {
                    params.workspace_root.clone()
                };

                let mut cmd = custos_domain::StartRunCommand::new(params.task_id, actor);
                if let Some(rev) = params.workflow_revision {
                    cmd = cmd.with_workflow_revision(rev);
                }
                if let Some(mode) = params.preferred_mode {
                    cmd = cmd.with_preferred_mode(mode);
                }
                if let Some(harness) = params.harness_id {
                    cmd = cmd.with_harness_id(harness);
                }
                if let Some(root) = resolved_workspace_root {
                    cmd = cmd.with_workspace_root(root);
                }
                if let Some(ref ws_id) = params.workspace_id {
                    cmd = cmd.with_workspace_id(ws_id.clone());
                }
                if let Some(ref session_id) = params.session_id {
                    cmd = cmd.with_session_id(session_id.clone());
                }
                if let Some(ref turn_id) = params.turn_id {
                    cmd = cmd.with_turn_id(turn_id.clone());
                }
                if let Some(ref m) = params.model {
                    cmd = cmd.with_model(m.clone());
                }

                cmd.reasoning_effort = params.reasoning_effort;
                cmd.provider_account_id = params.provider_account_id;

                let prompt = if let Some(p) = params.prompt {
                    Some(p)
                } else if let Some(ref sid) = params.session_id {
                    if let Some(journal) = self
                        .session_manager
                        .get_journal(&SessionId(sid.to_string()))
                        .await
                    {
                        journal
                            .entries
                            .iter()
                            .rev()
                            .find(|e| e.entry_type == "user_message")
                            .map(|e| e.entry_data.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(p) = prompt {
                    cmd = cmd.with_prompt(p);
                }

                let background = params.background;
                let result = if background {
                    workflow.start_run_background(cmd).await
                } else {
                    workflow.start_run(cmd).await
                };
                match result {
                    Ok(handle) => {
                        if let (Some(ref sid), Some(ref out)) = (
                            params.session_id.as_ref().filter(|_| !background),
                            handle.output.as_ref(),
                        ) {
                            let _ = self
                                .session_manager
                                .append_assistant_message(&SessionId(sid.to_string()), out)
                                .await;
                        }
                        match serde_json::to_value(&handle) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKFLOW_CANCEL_RUN => {
                let workflow = match self.workflow.as_ref() {
                    Some(w) => w,
                    None => {
                        return ApiResponse::error(req.id, "WorkflowPort not configured on daemon")
                    }
                };
                let params: CancelRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                let reason = params
                    .reason
                    .unwrap_or_else(|| "User requested cancellation".into());

                match workflow.request_cancel(&params.run_id, &reason).await {
                    Ok(receipt) => match serde_json::to_value(&receipt) {
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
