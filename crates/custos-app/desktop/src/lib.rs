mod browser;
use custos_daemon::local_api::LocalApiClient;
use custos_daemon::{ensure_daemon_client, ProfileResolver};
use std::sync::Arc;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(caller: tauri::Webview, name: &str) -> Result<String, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

#[tauri::command]
async fn custos_dispatch(
    caller: tauri::Webview,
    client: tauri::State<'_, Arc<LocalApiClient>>,
    raw_json: String,
) -> Result<String, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    let resp = client.dispatch_raw(&raw_json).await;
    Ok(resp)
}

#[tauri::command]
async fn custos_request(
    caller: tauri::Webview,
    client: tauri::State<'_, Arc<LocalApiClient>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method,
        params,
    };
    let resp = match client.send_request(req.clone()).await {
        Ok(r) => r,
        Err(e) => {
            let profile = ProfileResolver::from_env();
            if let Ok(new_client) = ensure_daemon_client(&profile) {
                new_client.send_request(req).await.map_err(|e2| format!("{e}; retry failed: {e2}"))?
            } else {
                return Err(e);
            }
        }
    };
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[tauri::command]
fn custos_http_endpoint(caller: tauri::Webview) -> Result<String, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    let profile = ProfileResolver::from_env();
    let port = profile
        .read_http_port()
        .ok_or_else(|| "Daemon HTTP endpoint is unavailable".to_string())?;
    Ok(format!("http://127.0.0.1:{port}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let profile = ProfileResolver::from_env();
    let client = ensure_daemon_client(&profile)
        .expect("Failed to connect to custos-daemon for Tauri desktop");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|view, payload| {
            if view.label() == "main" && payload.event() == tauri::webview::PageLoadEvent::Started {
                use tauri::Manager;
                if let Some(registry) = view.try_state::<browser::BrowserRegistry>() {
                    if let Ok(mut tabs) = registry.0.try_lock() { tabs.clear(); }
                }
                for (label, child) in view.app_handle().webviews() {
                    if label.starts_with("browser-") {
                        if let Err(error) = child.close() {
                            tracing::debug!(%label, %error, "Browser cleanup during frontend reload failed");
                        }
                    }
                }
            }
        })
        .manage(client)
        .manage(browser::BrowserRegistry::default())
        .invoke_handler(tauri::generate_handler![
            browser::browser_command,
            greet,
            custos_http_endpoint,
            custos_dispatch,
            custos_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
