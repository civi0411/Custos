//! Browser tab snapshot and DOM/accessibility tree inspection handlers.

use crate::api::*;
use crate::application::capabilities::browser_execution_adapter_connected;
use custos_adapters::host::binaries::resolve_agent_browser_binary;

impl LocalApiDispatcher {
    pub(super) async fn handle_browser_snapshot(&self, req: ApiRequest) -> ApiResponse {
        if !browser_execution_adapter_connected() {
            return ApiResponse::error(
                req.id,
                "Governed browser adapter is not connected; snapshot capture is fail-closed",
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
        let tab_id = if let Some(t) = req.params.get("tab_id").and_then(|v| v.as_str()) {
            t.to_string()
        } else {
            return ApiResponse::error(req.id, "Missing tab_id parameter");
        };

        let mut tab = match fleet.get_browser_tab(&tab_id) {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ApiResponse::error(req.id, format!("Tab '{tab_id}' not found"))
            }
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        };

        let now = chrono::Utc::now().timestamp_millis();

        if let Ok(explicit_snapshot) =
            serde_json::from_value::<BrowserPageSnapshot>(req.params.clone())
        {
            tab.last_snapshot = Some(explicit_snapshot);
            tab.updated_at = now;
            let _ = fleet.save_browser_tab(&tab);
            return ApiResponse::success(
                req.id,
                serde_json::json!({ "saved": true, "tab_id": tab_id }),
            );
        }

        // If agent-browser is available, execute real AX tree snapshot and real screenshot
        if let Some(agent_bin) = resolve_agent_browser_binary() {
            let snap_res = custos_adapters::integrations::run_agent_browser_json(
                &agent_bin,
                &tab_id,
                &["snapshot", "-i", "--urls"],
            )
            .await;

            let screenshot_res = custos_adapters::integrations::run_agent_browser_json(
                &agent_bin,
                &tab_id,
                &["screenshot"],
            )
            .await;

            let screenshot_path = screenshot_res
                .ok()
                .and_then(|val| val.get("data")?.get("path")?.as_str().map(str::to_string));

            if let Ok(val) = snap_res {
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
                            let refs_val = data.get("refs");

                            let mut links = Vec::new();
                            if let Some(refs_obj) = refs_val.and_then(|v| v.as_object()) {
                                for (key, val) in refs_obj {
                                    let name = val
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let role = val
                                        .get("role")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    links.push(format!("@{} [{}]: {}", key, role, name));
                                }
                            }

                            let summary = format!(
                                "URL: {}\n\nAccessibility Tree (Interactive Elements):\n{}\n\nInteractive Refs Count: {}",
                                origin,
                                if snap_text.is_empty() { "No interactive elements found" } else { &snap_text },
                                links.len()
                            );

                            let snapshot = BrowserPageSnapshot {
                                tab_id: tab_id.clone(),
                                url: origin.clone(),
                                title: if tab.title == "New Tab" {
                                    origin
                                } else {
                                    tab.title.clone()
                                },
                                dom_tree_summary: summary,
                                text_content: snap_text,
                                links,
                                viewport_width: 1280,
                                viewport_height: 800,
                                screenshot_uri: screenshot_path,
                                timestamp: now,
                            };

                            tab.last_snapshot = Some(snapshot);
                            tab.updated_at = now;
                            let _ = fleet.save_browser_tab(&tab);
                            return ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "saved": true,
                                    "tab_id": tab_id,
                                    "snapshot": tab.last_snapshot
                                }),
                            );
                        }
            }
        }

        // Fallback: HTTP GET fetch if agent-browser binary is not found
        let snapshot = if tab.url.starts_with("http://") || tab.url.starts_with("https://")
        {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();
            let start = std::time::Instant::now();
            match client.get(&tab.url).send().await {
                Ok(resp) => {
                    let status_code = resp.status().as_u16();
                    let content_type = resp
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    let duration = start.elapsed().as_millis() as u64;
                    let html_text = resp.text().await.unwrap_or_default();

                    tab.network_requests.push(BrowserNetworkRequest {
                        url: tab.url.clone(),
                        method: "GET".to_string(),
                        status: Some(status_code),
                        content_type: content_type.clone(),
                        duration_ms: duration,
                        timestamp: now,
                    });

                    let extracted_title =
                        if let Some(start_idx) = html_text.to_lowercase().find("<title>") {
                            let rest = &html_text[start_idx + 7..];
                            rest.find("</title>")
                                .map(|end_idx| rest[..end_idx].trim().to_string())
                        } else {
                            None
                        };

                    let mut links = Vec::new();
                    let mut search_slice = html_text.as_str();
                    while let Some(href_pos) = search_slice.find("href=\"") {
                        let remainder = &search_slice[href_pos + 6..];
                        if let Some(quote_end) = remainder.find('"') {
                            let href = &remainder[..quote_end];
                            if (href.starts_with("http://")
                                || href.starts_with("https://")
                                || href.starts_with('/'))
                                && links.len() < 25
                            {
                                links.push(href.to_string());
                            }
                            search_slice = &remainder[quote_end..];
                        } else {
                            break;
                        }
                    }

                    let mut plain_text = String::new();
                    let mut in_tag = false;
                    for c in html_text.chars() {
                        if c == '<' {
                            in_tag = true;
                        } else if c == '>' {
                            in_tag = false;
                            plain_text.push(' ');
                        } else if !in_tag {
                            plain_text.push(c);
                        }
                        if plain_text.len() > 4000 {
                            break;
                        }
                    }
                    let compact_text =
                        plain_text.split_whitespace().collect::<Vec<_>>().join(" ");
                    let final_title = extracted_title.unwrap_or_else(|| tab.url.clone());
                    tab.title = final_title.clone();

                    BrowserPageSnapshot {
                        tab_id: tab_id.clone(),
                        url: tab.url.clone(),
                        title: final_title.clone(),
                        dom_tree_summary: format!(
                            "Page Title: {}\nHTTP Status: {}\nExtracted Links: {}\nText Preview:\n{}",
                            final_title,
                            status_code,
                            links.len(),
                            custos_domain::truncate_str_to_char_boundary(&compact_text, 300)
                        ),
                        text_content: compact_text,
                        links,
                        viewport_width: 1280,
                        viewport_height: 800,
                        screenshot_uri: None,
                        timestamp: now,
                    }
                }
                Err(err) => {
                    tab.console_logs.push(BrowserConsoleEntry {
                        level: "error".to_string(),
                        message: format!("Failed to fetch {}: {}", tab.url, err),
                        timestamp: now,
                    });
                    BrowserPageSnapshot {
                        tab_id: tab_id.clone(),
                        url: tab.url.clone(),
                        title: tab.title.clone(),
                        dom_tree_summary: format!(
                            "Connection error fetching {}: {}",
                            tab.url, err
                        ),
                        text_content: String::new(),
                        links: Vec::new(),
                        viewport_width: 1280,
                        viewport_height: 800,
                        screenshot_uri: None,
                        timestamp: now,
                    }
                }
            }
        } else {
            BrowserPageSnapshot {
                tab_id: tab_id.clone(),
                url: tab.url.clone(),
                title: tab.title.clone(),
                dom_tree_summary: format!("Local tab: {}", tab.url),
                text_content: String::new(),
                links: Vec::new(),
                viewport_width: 1280,
                viewport_height: 800,
                screenshot_uri: None,
                timestamp: now,
            }
        };

        tab.last_snapshot = Some(snapshot);
        tab.updated_at = now;
        match fleet.save_browser_tab(&tab) {
            Ok(()) => ApiResponse::success(
                req.id,
                serde_json::json!({
                    "saved": true,
                    "tab_id": tab_id,
                    "snapshot": tab.last_snapshot
                }),
            ),
            Err(e) => ApiResponse::error(req.id, e.to_string()),
        }
    }
}
