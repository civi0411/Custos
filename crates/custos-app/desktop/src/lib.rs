mod browser;
use custos_sdk::local_api::LocalApiClient;
use custos_sdk::{
    ensure_daemon_client, ensure_daemon_client_async, ApiRequest, DaemonLifecycleState,
    ProfileResolver,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct DesktopLifecycleManager {
    pub state: Arc<RwLock<DaemonLifecycleState>>,
    pub profile: ProfileResolver,
}

impl DesktopLifecycleManager {
    pub fn new(profile: ProfileResolver, initial: DaemonLifecycleState) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            profile,
        }
    }

    pub async fn set_state(&self, new_state: DaemonLifecycleState) {
        let mut s = self.state.write().await;
        *s = new_state;
    }

    pub async fn get_state(&self) -> DaemonLifecycleState {
        *self.state.read().await
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(caller: tauri::Webview, name: &str) -> Result<String, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

#[tauri::command]
async fn custos_lifecycle_state(
    caller: tauri::Webview,
    lifecycle: tauri::State<'_, DesktopLifecycleManager>,
) -> Result<DaemonLifecycleState, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    Ok(lifecycle.get_state().await)
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
    lifecycle: tauri::State<'_, DesktopLifecycleManager>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    browser::authorize(&caller).map_err(|e| e.to_string())?;
    let req_id = params
        .get("command_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| custos_domain::new_id("ipc"));
    let req = ApiRequest {
        id: req_id,
        method,
        params,
    };
    let resp = match client.send_request(req.clone()).await {
        Ok(r) => {
            lifecycle.set_state(DaemonLifecycleState::Ready).await;
            r
        }
        Err(e) => {
            lifecycle.set_state(DaemonLifecycleState::Recovering).await;
            let profile = ProfileResolver::from_env();
            if let Ok(new_client) = ensure_daemon_client_async(&profile).await {
                let retry_resp = new_client
                    .send_request(req)
                    .await
                    .map_err(|e2| {
                        tokio::spawn({
                            let lc = lifecycle.inner().clone();
                            async move { lc.set_state(DaemonLifecycleState::Degraded).await; }
                        });
                        format!("{e}; retry failed: {e2}")
                    })?;
                lifecycle.set_state(DaemonLifecycleState::Ready).await;
                retry_resp
            } else {
                lifecycle.set_state(DaemonLifecycleState::Degraded).await;
                return Err(e);
            }
        }
    };
    if let Some(err) = resp.error {
        if err.contains("incompatible") || err.contains("schema mismatch") {
            lifecycle.set_state(DaemonLifecycleState::Incompatible).await;
        }
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
    let (client, initial_state) = match ensure_daemon_client(&profile) {
        Ok(c) => (c, DaemonLifecycleState::Ready),
        Err(e) => {
            tracing::warn!(error = %e, "Initial daemon connection failed; desktop will connect lazily on request");
            (
                Arc::new(LocalApiClient::from_port(profile.read_port().unwrap_or(3000))),
                DaemonLifecycleState::Connecting,
            )
        }
    };

    let lifecycle = DesktopLifecycleManager::new(profile, initial_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|view, payload| {
            if view.label() == "main" && payload.event() == tauri::webview::PageLoadEvent::Started {
                use tauri::Manager;
                if let Some(registry) = view.try_state::<browser::BrowserRegistry>() {
                    if let Ok(mut tabs) = registry.0.try_lock() {
                        tabs.clear();
                    }
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
        .manage(lifecycle)
        .manage(browser::BrowserRegistry::default())
        .invoke_handler(tauri::generate_handler![
            browser::browser_command,
            greet,
            custos_lifecycle_state,
            custos_http_endpoint,
            custos_dispatch,
            custos_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

