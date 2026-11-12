use crate::api::*;
use custos_domain::WriteWorkspaceFileParams;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspace_files(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACE_FILES_TREE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };
                let relative_dir = req.params.get("relative_dir").and_then(|v| v.as_str());
                let max_depth = req
                    .params
                    .get("max_depth")
                    .and_then(|v| v.as_u64())
                    .map(|d| d as usize);

                match self.env.files.get_file_tree(&ws, relative_dir, max_depth).await {
                    Ok(tree) => match serde_json::to_value(&tree) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_READ => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let max_bytes = req
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .map(|b| b as usize);
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                match self.env.files.read_file(&ws, path, max_bytes).await {
                    Ok(content) => match serde_json::to_value(&content) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_WORKSPACE_FILES_WRITE => {
                let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing workspace_id parameter"),
                };
                let path = match req.params.get("path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => return ApiResponse::error(req.id, "Missing path parameter"),
                };
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c.to_string(),
                    None => return ApiResponse::error(req.id, "Missing content parameter"),
                };
                let create_parents = req
                    .params
                    .get("create_parents")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let overwrite = req
                    .params
                    .get("overwrite")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let ws = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(ws) => ws,
                    Err(resp) => return resp,
                };

                let mut params = WriteWorkspaceFileParams::new(ws.id.clone(), path, content);
                params.create_parents = create_parents;
                params.overwrite = overwrite;

                match self.env.files.write_file(&ws, params).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown workspace files method: {unknown}")),
        }
    }
}
