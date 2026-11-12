//! Browser tab navigation and screenshot handlers.

use crate::api::*;
use crate::application::capabilities::browser_execution_adapter_connected;
use custos_adapters::host::binaries::resolve_agent_browser_binary;

impl LocalApiDispatcher {
    pub(super) async fn handle_browser_navigation(&self, req: ApiRequest) -> ApiResponse {
        if !browser_execution_adapter_connected() {
            let op = if req.method == METHOD_BROWSER_TABS_NAVIGATE {
                "navigation"
            } else {
                "screenshot"
            };
            return ApiResponse::error(
                req.id,
                format!("Governed browser adapter is not connected; {op} is fail-closed"),
            );
        }

        match req.method.as_str() {
            METHOD_BROWSER_TABS_NAVIGATE => {
                let fleet = match self.workbenches.browser.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "BrowserRepository not configured",
                        )
                    }
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let raw_url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(raw_url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                let mut tab = match fleet.get_browser_tab(tab_id) {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found"))
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let now = chrono::Utc::now().timestamp_millis();
                tab.url = raw_url.to_string();
                tab.title = if raw_url.is_empty() || raw_url == "about:blank" {
                    "New Tab".to_string()
                } else {
                    raw_url.to_string()
                };

                // Launch real browser navigation via agent-browser if available
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    if !raw_url.is_empty() && raw_url != "about:blank" {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(1500),
                            custos_adapters::integrations::run_agent_browser(
                                &agent_bin,
                                tab_id,
                                &["open", raw_url],
                            ),
                        )
                        .await;
                    }
                }

                tab.status = BrowserTabStatus::Ready;
                tab.updated_at = now;
                match fleet.save_browser_tab(&tab) {
                    Ok(()) => match serde_json::to_value(&tab) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_BROWSER_TABS_SCREENSHOT => {
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                match custos_adapters::integrations::run_agent_browser_json(
                    &agent_bin,
                    tab_id,
                    &["screenshot"],
                )
                .await
                {
                    Ok(val) => {
                        if let Some(p) = val
                            .get("data")
                            .and_then(|d| d.get("path"))
                            .and_then(|p| p.as_str())
                        {
                            let image_base64 =
                                custos_adapters::integrations::read_screenshot_base64(p);
                            return ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "path": p,
                                    "tab_id": tab_id,
                                    "image_base64": image_base64
                                }),
                            );
                        }
                        ApiResponse::success(req.id, serde_json::json!({ "tab_id": tab_id }))
                    }
                    Err(err_msg) => {
                        ApiResponse::error(req.id, format!("Screenshot failed: {err_msg}"))
                    }
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
