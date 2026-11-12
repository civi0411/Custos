//! Browser tab element click and form fill handlers.

use crate::api::*;
use crate::application::capabilities::browser_execution_adapter_connected;
use custos_adapters::host::binaries::resolve_agent_browser_binary;

impl LocalApiDispatcher {
    pub(super) async fn handle_browser_interaction(&self, req: ApiRequest) -> ApiResponse {
        if !browser_execution_adapter_connected() {
            let op = if req.method == METHOD_BROWSER_TABS_CLICK {
                "click"
            } else {
                "fill"
            };
            return ApiResponse::error(
                req.id,
                format!("Governed browser adapter is not connected; {op} is fail-closed"),
            );
        }
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
            METHOD_BROWSER_TABS_CLICK => {
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                match custos_adapters::integrations::run_agent_browser_json(
                    &agent_bin,
                    tab_id,
                    &["click", &formatted_ref],
                )
                .await
                {
                    Ok(_) => {
                        let mut tab = match fleet.get_browser_tab(tab_id) {
                            Ok(Some(t)) => t,
                            _ => {
                                return ApiResponse::success(
                                    req.id,
                                    serde_json::json!({ "clicked": formatted_ref }),
                                )
                            }
                        };
                        if let Ok(val) = custos_adapters::integrations::run_agent_browser_json(
                            &agent_bin,
                            tab_id,
                            &["snapshot", "-i", "--urls"],
                        )
                        .await
                        {
                            if let Some(data) = val.get("data") {
                                let origin = data
                                    .get("origin")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&tab.url)
                                    .to_string();
                                let snap_text = data
                                    .get("snapshot")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                tab.url = origin.clone();
                                if let Some(snap) = tab.last_snapshot.as_mut() {
                                    snap.url = origin;
                                    snap.text_content = snap_text.clone();
                                    snap.dom_tree_summary = snap_text;
                                }
                                let _ = fleet.save_browser_tab(&tab);
                            }
                        }
                        match serde_json::to_value(&tab) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(_) => ApiResponse::success(
                                req.id,
                                serde_json::json!({ "clicked": formatted_ref }),
                            ),
                        }
                    }
                    Err(err_msg) => ApiResponse::error(req.id, format!("Click failed: {err_msg}")),
                }
            }
            METHOD_BROWSER_TABS_FILL => {
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                let element_ref = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter (e.g. @e1)"),
                };
                let text = match req.params.get("text").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing text parameter"),
                };

                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                let formatted_ref = if element_ref.starts_with('@') {
                    element_ref.to_string()
                } else {
                    format!("@{}", element_ref)
                };

                match custos_adapters::integrations::run_agent_browser_json(
                    &agent_bin,
                    tab_id,
                    &["fill", &formatted_ref, text],
                )
                .await
                {
                    Ok(_) => {
                        let tab = fleet.get_browser_tab(tab_id).ok().flatten();
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "filled": formatted_ref,
                                "text": text,
                                "tab": tab
                            }),
                        )
                    }
                    Err(err_msg) => ApiResponse::error(req.id, format!("Fill failed: {err_msg}")),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
