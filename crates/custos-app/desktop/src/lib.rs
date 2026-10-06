use custos_daemon::local_api::LocalApiClient;
use custos_daemon::{ensure_daemon_client, ProfileResolver};
use std::sync::Arc;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn custos_dispatch(
    client: tauri::State<'_, Arc<LocalApiClient>>,
    raw_json: String,
) -> Result<String, String> {
    let resp = client.dispatch_raw(&raw_json).await;
    Ok(resp)
}

#[tauri::command]
async fn custos_request(
    client: tauri::State<'_, Arc<LocalApiClient>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method,
        params,
    };
    let resp = client.send_request(req).await.map_err(|e| e.to_string())?;
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let profile = ProfileResolver::from_env();
    let client = ensure_daemon_client(&profile)
        .expect("Failed to connect to custos-daemon for Tauri desktop");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(client)
        .invoke_handler(tauri::generate_handler![
            greet,
            custos_dispatch,
            custos_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
