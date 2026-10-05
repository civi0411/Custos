use custos_daemon::CustosRuntime;
use std::sync::Arc;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn custos_dispatch(
    runtime: tauri::State<'_, Arc<CustosRuntime>>,
    raw_json: String,
) -> Result<String, String> {
    let resp = runtime.local_api.dispatch_raw(&raw_json).await;
    Ok(resp)
}

#[tauri::command]
async fn custos_request(
    runtime: tauri::State<'_, Arc<CustosRuntime>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method,
        params,
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = std::env::var("CUSTOS_DATABASE").unwrap_or_else(|_| "custos.db".into());
    let runtime = Arc::new(
        CustosRuntime::bootstrap(&db_path)
            .expect("Failed to bootstrap CustosRuntime for Tauri desktop"),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![
            greet,
            custos_dispatch,
            custos_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
