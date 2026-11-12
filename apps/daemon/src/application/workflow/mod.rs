//! Workflow execution application use cases over the shared daemon context.

use super::*;

mod telemetry;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workflow(&self, req: ApiRequest) -> ApiResponse {
        let submethod = req
            .method
            .strip_prefix("v1.workflow.")
            .or_else(|| req.method.strip_prefix("custos.workflow."))
            .or_else(|| req.method.strip_prefix("workflow."))
            .or_else(|| req.method.strip_prefix("v1.supervision."))
            .or_else(|| req.method.strip_prefix("custos.supervision."))
            .or_else(|| req.method.strip_prefix("supervision."))
            .unwrap_or(req.method.as_str())
            .to_string();

        match submethod.as_str() {
            "session_runs" => {
                let Some(session) = req.params.get("session_id").and_then(|v| v.as_str()) else {
                    return ApiResponse::error(req.id, "Missing session_id");
                };
                if self
                    .kernel.session_manager
                    .get_session(&SessionId(session.to_string()))
                    .await
                    .is_none()
                {
                    return ApiResponse::error(req.id, "Unknown session");
                }
                let Some(store) = &self.kernel.usage_runs else {
                    return ApiResponse::error(req.id, "Run store unavailable");
                };
                let tasks = match self.kernel.task_service.list_tasks().await {
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
                            runs.push(serde_json::json!({
                                "run_id": run.id,
                                "model": run.metadata.get("requested_model").filter(|value| !value.is_null()).or_else(|| run.metadata.get("harness_id")),
                                "status": run.status.to_string(),
                                "journal_occurred_at": run.metadata.get("journal_occurred_at")
                            }));
                        }
                    }
                }
                ApiResponse::success(req.id, serde_json::json!(runs))
            }
            "live_run" => {
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
            "start_run" => {
                let workflow = match self.kernel.workflow.as_ref() {
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
                        .kernel.session_manager
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
                    Some(ref a) => {
                        let trimmed = a.trim();
                        if trimmed.is_empty() {
                            "daemon_user".to_string()
                        } else if trimmed.eq_ignore_ascii_case("system")
                            || trimmed.eq_ignore_ascii_case("kernel")
                            || trimmed.eq_ignore_ascii_case("root")
                        {
                            return ApiResponse::error(
                                req.id,
                                format!("Admission gate refused privileged actor override '{trimmed}': caller cannot claim system/kernel authority over Local API transport"),
                            );
                        } else if trimmed.len() > 128
                            || !trimmed.chars().all(|c| {
                                c.is_alphanumeric() || c == '_' || c == '-' || c == '@' || c == '.'
                            })
                        {
                            return ApiResponse::error(
                                req.id,
                                format!("Admission gate refused invalid actor identifier '{trimmed}': must be valid alphanumeric identifier up to 128 chars"),
                            );
                        } else {
                            trimmed.to_string()
                        }
                    }
                    None => "daemon_user".into(),
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
                        .kernel.session_manager
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
                        if let (Some(ref sid), Some(out)) = (
                            params.session_id.as_ref().filter(|_| !background),
                            handle.output.as_ref(),
                        ) {
                            let _ = self
                                .kernel.session_manager
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
            "cancel_run" => {
                let workflow = match self.kernel.workflow.as_ref() {
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
            // All telemetry and watchdog submethods routed cleanly:
            telemetry_submethod => {
                let sub_str = telemetry_submethod.to_string();
                self.handle_workflow_telemetry(req, &sub_str).await
            }
        }
    }
}
