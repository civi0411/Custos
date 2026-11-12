//! Browser tab lifecycle handlers.

use crate::api::*;
use custos_adapters::host::binaries::resolve_agent_browser_binary;

impl LocalApiDispatcher {
    pub(super) async fn handle_browser_tabs(&self, req: ApiRequest) -> ApiResponse {
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
            METHOD_BROWSER_TABS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                match fleet.list_browser_tabs(Some(session_id)) {
                    Ok(tabs) => match serde_json::to_value(&tabs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CREATE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = req.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let tab = match BrowserTab::new(session_id, url) {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_CLOSE => {
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = custos_adapters::integrations::run_agent_browser(
                        &agent_bin,
                        tab_id,
                        &["close"],
                    )
                    .await;
                }
                match fleet.delete_browser_tab(tab_id) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "closed": true, "tab_id": tab_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
