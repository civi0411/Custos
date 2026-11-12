//! Browser session handlers.

use crate::api::*;

impl LocalApiDispatcher {
    pub(super) async fn handle_browser_sessions(&self, req: ApiRequest) -> ApiResponse {
        let fleet = match self.workbenches.browser.as_ref() {
            Some(f) => f,
            None => {
                return ApiResponse::error(
                    req.id,
                    "BrowserRepository not configured",
                )
            }
        };

        match req.method.as_str() {
            METHOD_BROWSER_SESSIONS_LIST => {
                let ws_id = req.params.get("workspace_id").and_then(|v| v.as_str());
                match fleet.list_browser_sessions() {
                    Ok(sessions) => {
                        let filtered = if let Some(ws) = ws_id {
                            sessions
                                .into_iter()
                                .filter(|s| s.workspace_id.as_deref() == Some(ws))
                                .collect()
                        } else {
                            sessions
                        };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_SESSIONS_CREATE => {
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Default Session");
                let ws_id = req
                    .params
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let session = BrowserSession::new(name, ws_id);
                match fleet.save_browser_session(&session) {
                    Ok(()) => match serde_json::to_value(&session) {
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
