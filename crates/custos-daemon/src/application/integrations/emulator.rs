//! Android and mobile emulator device control handlers.

use crate::api::*;
use base64::Engine;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations_emulator(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
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
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
