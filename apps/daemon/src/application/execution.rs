//! execution application use cases over the shared daemon context.

use super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_execution(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_TERMINAL_SPAWN => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };

                let working_dir_path = if let Some(coordinator) = self.env.workspace.as_ref() {
                    let ws_id = custos_contracts::WorkspaceId::new(workspace_id);
                    let ws_base = match coordinator.get_workspace(&ws_id).await {
                        Ok(Some(ws)) => std::path::PathBuf::from(ws.path),
                        Ok(None) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace {workspace_id} not found"),
                            );
                        }
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace {workspace_id} lookup failed: {e}"),
                            );
                        }
                    };
                    let canonical_base = match ws_base.canonicalize() {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Workspace {workspace_id} root inaccessible: {e}"),
                            );
                        }
                    };
                    match req.params.get("working_dir").and_then(|v| v.as_str()) {
                        Some(dir) => {
                            let req_path = std::path::Path::new(dir);
                            let resolved = if req_path.is_absolute() {
                                req_path.to_path_buf()
                            } else {
                                canonical_base.join(req_path)
                            };
                            let canonical_dir = match resolved.canonicalize() {
                                Ok(p) => p,
                                Err(e) => {
                                    return ApiResponse::error(
                                        req.id,
                                        format!("Invalid working_dir '{dir}': {e}"),
                                    );
                                }
                            };
                            if !canonical_dir.starts_with(&canonical_base) {
                                return ApiResponse::error(
                                    req.id,
                                    format!(
                                        "Security boundary violation: working_dir '{dir}' escapes workspace '{}'",
                                        canonical_base.display()
                                    ),
                                );
                            }
                            canonical_dir
                        }
                        None => canonical_base,
                    }
                } else if let Some(dir) = req.params.get("working_dir").and_then(|v| v.as_str()) {
                    match std::path::Path::new(dir).canonicalize() {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Invalid working_dir '{dir}': {e}"),
                            );
                        }
                    }
                } else {
                    return ApiResponse::error(
                        req.id,
                        "WorkspaceCoordinator not configured on daemon",
                    );
                };

                let command = req.params.get("command").and_then(|v| v.as_str());
                let cols = req
                    .params
                    .get("cols")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(80) as u16;
                let rows = req
                    .params
                    .get("rows")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(24) as u16;

                match self
                    .env
                    .terminal
                    .spawn(workspace_id, &working_dir_path, command, cols, rows)
                    .await
                {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_WRITE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let data = match req.params.get("data").and_then(|v| v.as_str()) {
                    Some(d) => d,
                    None => return ApiResponse::error(req.id, "Missing data parameter"),
                };

                match self.env.terminal.write(session_id, data.as_bytes()).await {
                    Ok(written) => {
                        ApiResponse::success(req.id, serde_json::json!({ "written": written }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_RESIZE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let cols = match req.params.get("cols").and_then(|v| v.as_u64()) {
                    Some(c) => c as u16,
                    None => return ApiResponse::error(req.id, "Missing cols parameter"),
                };
                let rows = match req.params.get("rows").and_then(|v| v.as_u64()) {
                    Some(r) => r as u16,
                    None => return ApiResponse::error(req.id, "Missing rows parameter"),
                };

                match self.env.terminal.resize(session_id, cols, rows).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "ok": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_READ => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let from_seq = req
                    .params
                    .get("from_seq")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let max_bytes = req
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(65536) as usize;

                match self
                    .env
                    .terminal
                    .read_output(session_id, from_seq, max_bytes)
                    .await
                {
                    Ok(chunk) => match serde_json::to_value(&chunk) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_TERMINATE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.env.terminal.terminate(session_id).await {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "terminated": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_LIST => {
                let workspace_id = req.params.get("workspace_id").and_then(|v| v.as_str());

                match self.env.terminal.list_sessions(workspace_id).await {
                    Ok(sessions) => match serde_json::to_value(&sessions) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_TERMINAL_GET => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };

                match self.env.terminal.get_session(session_id).await {
                    Ok(session) => match serde_json::to_value(&session) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_LIST => {
                let harnesses = self.env.harnesses.list();
                match serde_json::to_value(&harnesses) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_HARNESS_GET => {
                let harness_id = req
                    .params
                    .get("harness_id")
                    .or_else(|| req.params.get("id"))
                    .and_then(|v| v.as_str());

                match harness_id {
                    Some(id) => match self.env.harnesses.get_descriptor(id) {
                        Some(desc) => match serde_json::to_value(&desc) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        },
                        None => ApiResponse::error(req.id, format!("Harness '{id}' not found")),
                    },
                    None => ApiResponse::error(req.id, "Missing harness_id parameter"),
                }
            }
            METHOD_HARNESS_RUN_NATIVE => {
                let params: RunNativeHarnessRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid run_native parameters: {e}"),
                        )
                    }
                };

                // Require authoritative workspace binding to prevent arbitrary filesystem escapes
                let coordinator = match self.env.workspace.as_ref() {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "WorkspaceCoordinator not configured on daemon",
                        );
                    }
                };

                let cwd_path = if let Some(ws_id) = &params.workspace_id {
                    match self.resolve_workspace_for_dispatch(&req.id, ws_id).await {
                        Ok(ws) => {
                            let base = std::path::PathBuf::from(&ws.path);
                            let canonical_base = match base.canonicalize() {
                                Ok(p) => p,
                                Err(e) => {
                                    return ApiResponse::error(
                                        req.id,
                                        format!("Workspace root inaccessible: {e}"),
                                    );
                                }
                            };
                            if let Some(c) = &params.cwd {
                                let requested = std::path::PathBuf::from(c);
                                let resolved = if requested.is_absolute() {
                                    requested
                                } else {
                                    canonical_base.join(requested)
                                };
                                let canonical_req = match resolved.canonicalize() {
                                    Ok(p) => p,
                                    Err(e) => {
                                        return ApiResponse::error(
                                            req.id,
                                            format!("Invalid cwd '{c}': {e}"),
                                        );
                                    }
                                };
                                if !canonical_req.starts_with(&canonical_base) {
                                    return ApiResponse::error(
                                        req.id,
                                        format!("Security boundary violation: cwd '{c}' escapes registered workspace '{}'", canonical_base.display()),
                                    );
                                }
                                canonical_req
                            } else {
                                canonical_base
                            }
                        }
                        Err(resp) => return resp,
                    }
                } else if let Some(c) = &params.cwd {
                    let requested = std::path::PathBuf::from(c);
                    let registered_workspaces =
                        coordinator.list_workspaces().await.unwrap_or_default();
                    let canonical = match requested.canonicalize() {
                        Ok(p) => p,
                        Err(e) => {
                            return ApiResponse::error(req.id, format!("Invalid cwd '{c}': {e}"));
                        }
                    };
                    let is_within_registered = registered_workspaces.iter().any(|ws| {
                        std::path::PathBuf::from(&ws.path)
                            .canonicalize()
                            .is_ok_and(|ws_canonical| canonical.starts_with(&ws_canonical))
                    });
                    if !is_within_registered {
                        return ApiResponse::error(
                            req.id,
                            format!("Security boundary violation: raw harness execution requires cwd inside a registered workspace, '{c}' is outside registered boundaries"),
                        );
                    }
                    canonical
                } else {
                    return ApiResponse::error(
                        req.id,
                        "Missing workspace_id: raw harness execution must be bound to a registered workspace",
                    );
                };

                match self
                    .env
                    .harnesses
                    .run_native(&params.harness_id, &params.instruction, &cwd_path)
                    .await
                {
                    Ok(result) => match serde_json::to_value(&result) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(err) => {
                        ApiResponse::error(req.id, format!("Harness execution failed: {err}"))
                    }
                }
            }
            METHOD_HARNESS_CANCEL => {
                let params: CancelHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid cancel parameters: {e}"),
                        )
                    }
                };
                match self
                    .env
                    .harnesses
                    .cancel(&params.harness_id, &params.run_id)
                    .await
                {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "cancelled": true }))
                    }
                    Err(err) => {
                        ApiResponse::error(req.id, format!("Failed to cancel harness: {err}"))
                    }
                }
            }
            METHOD_HARNESS_STEER => {
                let params: SteerHarnessRunRequest = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid steer parameters: {e}"))
                    }
                };
                match self
                    .env
                    .harnesses
                    .steer(&params.harness_id, &params.run_id, &params.guidance)
                    .await
                {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "steered": true })),
                    Err(err) => {
                        ApiResponse::error(req.id, format!("Failed to steer harness: {err}"))
                    }
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
