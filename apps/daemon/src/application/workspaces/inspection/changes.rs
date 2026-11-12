use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspace_changes(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.get_diff(&ws, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILE_DIFF => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let staged = req.params.get("staged").and_then(|v| v.as_bool());
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.get_file_diff(&ws, path, staged).await {
                    Ok(diff) => match serde_json::to_value(&diff) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_STAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.stage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "staged": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_UNSTAGE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.unstage_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "unstaged": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_DISCARD => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.discard_file(&ws, path).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "discarded": true, "path": path }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_GIT_COMMIT => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let message = match req.params.get("message").and_then(|v| v.as_str()) {
                    Some(m) if !m.trim().is_empty() => m,
                    _ => return ApiResponse::error(req.id, "Missing message parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.commit(&ws, message).await {
                    Ok(out) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "committed": true, "output": out }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown workspace changes method: {unknown}")),
        }
    }
}
