//! Browser session and tab automation handlers.

use crate::api::*;
use base64::Engine;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations_browser(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_BROWSER_SESSIONS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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
            METHOD_BROWSER_TABS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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
            METHOD_BROWSER_TABS_NAVIGATE => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; navigation is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
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
                            tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("open")
                                .arg(raw_url)
                                .output(),
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
            METHOD_BROWSER_TABS_SNAPSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; snapshot capture is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
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
                    let snap_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("snapshot")
                        .arg("-i")
                        .arg("--urls")
                        .output()
                        .await;

                    let screenshot_out = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&tab_id)
                        .arg("--json")
                        .arg("screenshot")
                        .output()
                        .await;

                    let mut screenshot_path = None;
                    if let Ok(ref s_out) = screenshot_out {
                        if s_out.status.success() {
                            if let Ok(val) =
                                serde_json::from_slice::<serde_json::Value>(&s_out.stdout)
                            {
                                if let Some(p) = val
                                    .get("data")
                                    .and_then(|d| d.get("path"))
                                    .and_then(|p| p.as_str())
                                {
                                    screenshot_path = Some(p.to_string());
                                }
                            }
                        }
                    }

                    if let Ok(s_out) = snap_out {
                        if s_out.status.success() {
                            if let Ok(val) =
                                serde_json::from_slice::<serde_json::Value>(&s_out.stdout)
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
            METHOD_BROWSER_TABS_CLICK => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; click is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("click")
                    .arg(&formatted_ref)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            // After click, update snapshot
                            let mut tab = match fleet.get_browser_tab(tab_id) {
                                Ok(Some(t)) => t,
                                _ => {
                                    return ApiResponse::success(
                                        req.id,
                                        serde_json::json!({ "clicked": formatted_ref }),
                                    )
                                }
                            };
                            let snap_out = tokio::process::Command::new(&agent_bin)
                                .arg("--session")
                                .arg(tab_id)
                                .arg("--json")
                                .arg("snapshot")
                                .arg("-i")
                                .arg("--urls")
                                .output()
                                .await;

                            if let Ok(so) = snap_out {
                                if let Ok(val) =
                                    serde_json::from_slice::<serde_json::Value>(&so.stdout)
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
                            }
                            match serde_json::to_value(&tab) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(_) => ApiResponse::success(
                                    req.id,
                                    serde_json::json!({ "clicked": formatted_ref }),
                                ),
                            }
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Click failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}"))
                    }
                }
            }
            METHOD_BROWSER_TABS_FILL => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; fill is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
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

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("fill")
                    .arg(&formatted_ref)
                    .arg(text)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            let tab = fleet.get_browser_tab(tab_id).ok().flatten();
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "filled": formatted_ref,
                                    "text": text,
                                    "tab": tab
                                }),
                            )
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Fill failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}"))
                    }
                }
            }
            METHOD_BROWSER_TABS_SCREENSHOT => {
                if !browser_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed browser adapter is not connected; screenshot is fail-closed",
                    );
                }
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

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(tab_id)
                    .arg("--json")
                    .arg("screenshot")
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout)
                            {
                                if let Some(p) = val
                                    .get("data")
                                    .and_then(|d| d.get("path"))
                                    .and_then(|p| p.as_str())
                                {
                                    let image_base64 = std::fs::read(p).ok().map(|bytes| {
                                        format!(
                                            "data:image/png;base64,{}",
                                            base64::engine::general_purpose::STANDARD
                                                .encode(&bytes)
                                        )
                                    });
                                    return ApiResponse::success(
                                        req.id,
                                        serde_json::json!({
                                            "path": p,
                                            "tab_id": tab_id,
                                            "image_base64": image_base64
                                        }),
                                    );
                                }
                            }
                            ApiResponse::success(req.id, serde_json::json!({ "tab_id": tab_id }))
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(
                                req.id,
                                format!("Screenshot failed: {}", err_msg.trim()),
                            )
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}"))
                    }
                }
            }
            METHOD_BROWSER_TABS_CLOSE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let tab_id = match req.params.get("tab_id").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return ApiResponse::error(req.id, "Missing tab_id parameter"),
                };
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(tab_id)
                        .arg("close")
                        .output()
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
