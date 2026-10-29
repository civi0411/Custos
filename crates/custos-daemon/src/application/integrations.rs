//! integrations application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_integrations(&self, req: ApiRequest) -> ApiResponse {
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
            METHOD_EMULATOR_DEVICES_LIST => {
                let devices = serde_json::json!([
                    {
                        "id": "iphone_15_pro",
                        "name": "iPhone 15 Pro",
                        "platform": "iOS",
                        "width": 393,
                        "height": 852,
                        "scale": 3.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "iphone_se",
                        "name": "iPhone SE (3rd gen)",
                        "platform": "iOS",
                        "width": 375,
                        "height": 667,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    },
                    {
                        "id": "pixel_8",
                        "name": "Google Pixel 8",
                        "platform": "Android",
                        "width": 412,
                        "height": 915,
                        "scale": 2.625,
                        "user_agent": "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Mobile Safari/537.36",
                        "status": "ready"
                    },
                    {
                        "id": "ipad_mini",
                        "name": "iPad Mini",
                        "platform": "iPadOS",
                        "width": 744,
                        "height": 1133,
                        "scale": 2.0,
                        "user_agent": "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
                        "status": "ready"
                    }
                ]);
                ApiResponse::success(req.id, devices)
            }
            METHOD_EMULATOR_LAUNCH => {
                let device_id = req
                    .params
                    .get("device_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("iphone_15_pro");
                let session_id = req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        format!(
                            "mobile_{}_{}",
                            device_id,
                            chrono::Utc::now().timestamp_millis()
                        )
                    });
                let url = req
                    .params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://google.com");

                let (width, height, scale): (u32, u32, f64) = match device_id {
                    "iphone_se" => (375, 667, 2.0),
                    "pixel_8" => (412, 915, 2.625),
                    "ipad_mini" => (744, 1133, 2.0),
                    _ => (393, 852, 3.0),
                };

                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("set")
                        .arg("viewport")
                        .arg(width.to_string())
                        .arg(height.to_string())
                        .arg(scale.to_string())
                        .output()
                        .await;

                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(&session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "session_id": session_id,
                        "device_id": device_id,
                        "width": width,
                        "height": height,
                        "scale": scale,
                        "url": url,
                        "status": "running"
                    }),
                )
            }
            METHOD_EMULATOR_NAVIGATE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let url = match req.params.get("url").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => return ApiResponse::error(req.id, "Missing url parameter"),
                };
                if let Err(e) = validate_browser_url(url) {
                    return ApiResponse::error(req.id, format!("Invalid URL: {e}"));
                }
                if let Some(agent_bin) = resolve_agent_browser_binary() {
                    let _ = tokio::process::Command::new(&agent_bin)
                        .arg("--session")
                        .arg(session_id)
                        .arg("open")
                        .arg(url)
                        .output()
                        .await;
                }
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "session_id": session_id,
                        "url": url,
                        "status": "ready"
                    }),
                )
            }
            METHOD_EMULATOR_SNAPSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                let snap_out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("--json")
                    .arg("snapshot")
                    .arg("-i")
                    .arg("--urls")
                    .output()
                    .await;

                match snap_out {
                    Ok(o) => {
                        if o.status.success() {
                            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&o.stdout)
                            {
                                if let Some(data) = val.get("data") {
                                    return ApiResponse::success(
                                        req.id,
                                        serde_json::json!({
                                            "session_id": session_id,
                                            "snapshot": data.get("snapshot").and_then(|v| v.as_str()).unwrap_or(""),
                                            "refs": data.get("refs").cloned().unwrap_or(serde_json::json!({})),
                                            "origin": data.get("origin").and_then(|v| v.as_str()).unwrap_or("")
                                        }),
                                    );
                                }
                            }
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({ "session_id": session_id }),
                            )
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(
                                req.id,
                                format!("Snapshot failed: {}", err_msg.trim()),
                            )
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}"))
                    }
                }
            }
            METHOD_EMULATOR_SCREENSHOT => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
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
                                            "session_id": session_id,
                                            "path": p,
                                            "image_base64": image_base64
                                        }),
                                    );
                                }
                            }
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({ "session_id": session_id }),
                            )
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
            METHOD_EMULATOR_TAP => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "Missing session_id parameter"),
                };
                let ref_key = match req.params.get("ref").and_then(|v| v.as_str()) {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "Missing ref parameter"),
                };
                let agent_bin = match resolve_agent_browser_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(req.id, "agent-browser engine binary not found")
                    }
                };

                let selector = if ref_key.starts_with('@') {
                    ref_key.to_string()
                } else {
                    format!("@{ref_key}")
                };

                let out = tokio::process::Command::new(&agent_bin)
                    .arg("--session")
                    .arg(session_id)
                    .arg("click")
                    .arg(&selector)
                    .output()
                    .await;

                match out {
                    Ok(o) => {
                        if o.status.success() {
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "session_id": session_id,
                                    "ref": selector,
                                    "tapped": true
                                }),
                            )
                        } else {
                            let err_msg = String::from_utf8_lossy(&o.stderr);
                            ApiResponse::error(req.id, format!("Tap failed: {}", err_msg.trim()))
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn agent-browser: {e}"))
                    }
                }
            }
            METHOD_FLEET_HOSTS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                match fleet.list_remote_hosts() {
                    Ok(hosts) => match serde_json::to_value(&hosts) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_REGISTER => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let params: RegisterHostParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid register host params: {e}"),
                        )
                    }
                };
                let auth_method = if let Some(key_path) = &params.private_key_path {
                    SshAuthMethod::KeyPair {
                        private_key_path: key_path.clone(),
                        passphrase: None,
                    }
                } else {
                    SshAuthMethod::Agent
                };
                let mut node = match RemoteHostNode::new(
                    params.name,
                    params.host,
                    params.port,
                    params.user,
                    auth_method,
                ) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                node.labels = params.labels;
                match fleet.save_remote_host(&node) {
                    Ok(()) => match serde_json::to_value(&node) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_PING => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; SSH probe is fail-closed",
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
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let mut host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Host '{host_id}' not found"))
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin =
                    resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("ConnectTimeout=3")
                    .arg("-o")
                    .arg("StrictHostKeyChecking=yes")
                    .arg("-p")
                    .arg(host.port.to_string());

                if let SshAuthMethod::KeyPair {
                    private_key_path, ..
                } = &host.auth_method
                {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg("echo -n ping");

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        if out.status.success() {
                            host.status = RemoteHostStatus::Online;
                            host.last_ping_ms = Some(duration_ms);
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "host_id": host.id,
                                    "status": "online",
                                    "ping_ms": duration_ms
                                }),
                            )
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            host.status = RemoteHostStatus::Offline;
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "host_id": host.id,
                                    "status": "offline",
                                    "error": stderr.trim()
                                }),
                            )
                        }
                    }
                    Err(e) => {
                        host.status = RemoteHostStatus::Offline;
                        host.updated_at = now;
                        let _ = fleet.save_remote_host(&host);
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "host_id": host.id,
                                "status": "offline",
                                "error": e.to_string()
                            }),
                        )
                    }
                }
            }
            METHOD_FLEET_EXEC => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; remote command is fail-closed",
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
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let command = match req.params.get("command").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing command parameter"),
                };
                let host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Host '{host_id}' not found"))
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin =
                    resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("ConnectTimeout=10")
                    .arg("-o")
                    .arg("StrictHostKeyChecking=yes")
                    .arg("-p")
                    .arg(host.port.to_string());

                if let SshAuthMethod::KeyPair {
                    private_key_path, ..
                } = &host.auth_method
                {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg(command);

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let receipt = FleetExecReceipt {
                            execution_id: new_id("fexec"),
                            host_id: host.id.clone(),
                            command: command.to_string(),
                            exit_code: out.status.code(),
                            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                            duration_ms,
                            // Process exit is an operational observation, not
                            // independent verification of the remote effect.
                            verified: false,
                            timestamp: now,
                        };
                        let _ = fleet.save_fleet_execution(&receipt);
                        match serde_json::to_value(&receipt) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn SSH process: {e}"))
                    }
                }
            }
            METHOD_AUTOMATION_JOBS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let status_filter = req
                    .params
                    .get("status")
                    .and_then(|v| serde_json::from_value::<HeadlessJobStatus>(v.clone()).ok());
                match fleet.list_headless_jobs() {
                    Ok(jobs) => {
                        let filtered: Vec<HeadlessAutomationJob> =
                            if let Some(filter) = status_filter {
                                jobs.into_iter().filter(|j| j.status == filter).collect()
                            } else {
                                jobs
                            };
                        match serde_json::to_value(&filtered) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_CREATE => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let params: CreateHeadlessJobParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid job parameters: {e}"))
                    }
                };
                let trigger = params
                    .trigger
                    .unwrap_or(custos_domain::HeadlessTrigger::Manual);
                let job = match HeadlessAutomationJob::new(params.name, params.spec, trigger) {
                    Ok(j) => j,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                match fleet.save_headless_job(&job) {
                    Ok(()) => match serde_json::to_value(&job) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_AUTOMATION_JOBS_RUN => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let job_id = match req.params.get("job_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => return ApiResponse::error(req.id, "Missing 'job_id' parameter"),
                };

                let has_permit = req.params.get("permit_id").is_some();
                let force_execute = req
                    .params
                    .get("force_execute")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !has_permit && !force_execute {
                    return ApiResponse::error(
                        req.id,
                        "Headless execution is unavailable until an executor and scoped authority checks are connected",
                    );
                }

                match fleet.get_headless_job(job_id) {
                    Ok(Some(mut job)) => {
                        job.status = custos_domain::HeadlessJobStatus::Running;
                        job.started_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        let mut cmd = tokio::process::Command::new("sh");
                        cmd.arg("-c").arg(&job.spec.command_or_script);
                        let timeout_secs = job.spec.timeout_secs.clamp(1, 300);
                        let out = tokio::time::timeout(
                            std::time::Duration::from_secs(timeout_secs),
                            cmd.output(),
                        )
                        .await;

                        let (stdout, stderr, exit_code, success) = match out {
                            Ok(Ok(o)) => (
                                String::from_utf8_lossy(&o.stdout).to_string(),
                                String::from_utf8_lossy(&o.stderr).to_string(),
                                o.status.code().unwrap_or(0),
                                o.status.success(),
                            ),
                            Ok(Err(e)) => (String::new(), e.to_string(), 1, false),
                            Err(_) => {
                                (String::new(), "Execution timed out".to_string(), 124, false)
                            }
                        };

                        job.status = if success {
                            custos_domain::HeadlessJobStatus::Completed
                        } else {
                            custos_domain::HeadlessJobStatus::Failed
                        };
                        job.exit_code = Some(exit_code);
                        job.output_log = if stderr.is_empty() {
                            stdout
                        } else {
                            format!("{stdout}\n{stderr}")
                        };
                        job.completed_at = Some(chrono::Utc::now().timestamp_millis());
                        let _ = fleet.save_headless_job(&job);

                        match serde_json::to_value(&job) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Job '{job_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
