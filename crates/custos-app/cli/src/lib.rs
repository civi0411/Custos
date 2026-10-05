use custos_daemon::CustosRuntime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub epoch: u64,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<serde_json::Value>,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
    pub rationale: Option<String>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Greetings from Custos SADE CLI Backend!", name)
}

#[tauri::command]
async fn custos_dispatch(
    runtime: State<'_, Arc<CustosRuntime>>,
    raw_json: String,
) -> Result<String, String> {
    let resp = runtime.local_api.dispatch_raw(&raw_json).await;
    Ok(resp)
}

#[tauri::command]
async fn custos_request(
    runtime: State<'_, Arc<CustosRuntime>>,
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

#[tauri::command]
async fn list_tasks(runtime: State<'_, Arc<CustosRuntime>>) -> Result<Vec<serde_json::Value>, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method: "v1.tasks.list".into(),
        params: serde_json::json!({}),
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        Err(err)
    } else if let Some(serde_json::Value::Array(arr)) = resp.result {
        Ok(arr)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn get_task(
    runtime: State<'_, Arc<CustosRuntime>>,
    id: String,
) -> Result<Option<serde_json::Value>, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method: "v1.tasks.get".into(),
        params: serde_json::json!({ "task_id": id }),
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        if err.contains("not found") {
            Ok(None)
        } else {
            Err(err)
        }
    } else {
        Ok(resp.result)
    }
}

#[tauri::command]
async fn create_task(
    runtime: State<'_, Arc<CustosRuntime>>,
    title: String,
    metadata: Option<String>,
) -> Result<serde_json::Value, String> {
    let meta_json = metadata.and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok());
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method: "v1.tasks.create".into(),
        params: serde_json::json!({
            "title": title,
            "metadata": meta_json,
        }),
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[tauri::command]
async fn advance_task(
    runtime: State<'_, Arc<CustosRuntime>>,
    id: String,
    status: String,
    rationale: Option<String>,
) -> Result<serde_json::Value, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method: "v1.tasks.advance".into(),
        params: serde_json::json!({
            "task_id": id,
            "status": status,
            "rationale": rationale,
        }),
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[tauri::command]
async fn cancel_task(
    runtime: State<'_, Arc<CustosRuntime>>,
    id: String,
    reason: Option<String>,
) -> Result<serde_json::Value, String> {
    let req = custos_daemon::ApiRequest {
        id: custos_domain::new_id("ipc"),
        method: "v1.tasks.cancel".into(),
        params: serde_json::json!({
            "task_id": id,
            "reason": reason,
        }),
    };
    let resp = runtime.local_api.handle_request(req).await;
    if let Some(err) = resp.error {
        Err(err)
    } else {
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}

#[tauri::command]
fn explain_architecture(query: Option<String>) -> String {
    format!(
        "=== Custos SADE Operating System Architecture ===\n\
        • Invariant Gate: Zero-IO verification and proof closure\n\
        • Durable Orchestration: SQLite task ledger, outbox events, and fencing\n\
        • OmniRoute 3-Tier Cascade: Fast model -> Standard -> Frontier (Claude/Codex/Gemini)\n\
        • Integrated AIDE Studio: Live Monaco editor, multi-tab terminal, ArXiv research graph\n\
        Query context: {}",
        query.unwrap_or_else(|| "Overview".to_string())
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = std::env::var("CUSTOS_DATABASE").unwrap_or_else(|_| "custos.db".into());
    let runtime = Arc::new(
        CustosRuntime::bootstrap(&db_path)
            .expect("Failed to bootstrap CustosRuntime for Tauri CLI app"),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![
            greet,
            custos_dispatch,
            custos_request,
            list_tasks,
            get_task,
            create_task,
            advance_task,
            cancel_task,
            explain_architecture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
