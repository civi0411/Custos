use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpan {
    pub id: u64,
    pub task_id: String,
    pub name: String,
    pub stage: String,
    pub started_at: String,
    pub duration_ms: Option<u64>,
    pub details: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub tasks: Mutex<Vec<Task>>,
    pub spans: Mutex<Vec<TaskSpan>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Greetings from Custos CLI Runtime!", name)
}

#[tauri::command]
fn list_tasks(state: State<AppState>) -> Vec<Task> {
    let tasks = state.tasks.lock().unwrap();
    tasks.clone()
}

#[tauri::command]
fn get_task(state: State<AppState>, id: String) -> Option<Task> {
    let tasks = state.tasks.lock().unwrap();
    tasks.iter().find(|t| t.id == id).cloned()
}

#[tauri::command]
fn create_task(state: State<AppState>, title: String, metadata: Option<String>) -> Task {
    let mut tasks = state.tasks.lock().unwrap();
    let mut spans = state.spans.lock().unwrap();

    let id = format!("task_{}", &uuid_simple());
    let now = chrono_now();

    let meta_json = metadata.and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok());

    let task = Task {
        id: id.clone(),
        title,
        status: "Draft".to_string(),
        epoch: 0,
        created_at: now.clone(),
        updated_at: now.clone(),
        metadata: meta_json,
        result_summary: None,
        failure_reason: None,
        rationale: None,
    };

    let span_id = spans.len() as u64 + 1;
    spans.push(TaskSpan {
        id: span_id,
        task_id: id.clone(),
        name: "task_initialization".to_string(),
        stage: "Inspection".to_string(),
        started_at: now,
        duration_ms: Some(15),
        details: Some("Initialized task in Custos runtime".to_string()),
    });

    tasks.insert(0, task.clone());
    task
}

#[tauri::command]
fn advance_task(
    state: State<AppState>,
    id: String,
    status: String,
    rationale: Option<String>,
) -> Result<Task, String> {
    let mut tasks = state.tasks.lock().unwrap();
    let mut spans = state.spans.lock().unwrap();

    let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("Task with id '{id}' not found"))?;

    task.status = status.clone();
    task.epoch += 1;
    task.updated_at = chrono_now();
    if let Some(r) = rationale.clone() {
        task.rationale = Some(r);
    }

    let span_id = spans.len() as u64 + 1;
    spans.push(TaskSpan {
        id: span_id,
        task_id: id,
        name: format!("advance_to_{}", status.to_lowercase()),
        stage: status,
        started_at: chrono_now(),
        duration_ms: Some(30),
        details: rationale,
    });

    Ok(task.clone())
}

#[tauri::command]
fn complete_task(
    state: State<AppState>,
    id: String,
    summary: Option<String>,
) -> Result<Task, String> {
    let mut tasks = state.tasks.lock().unwrap();
    let mut spans = state.spans.lock().unwrap();

    let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("Task with id '{id}' not found"))?;

    task.status = "Succeeded".to_string();
    task.epoch += 1;
    task.updated_at = chrono_now();
    task.result_summary = summary.clone();

    let span_id = spans.len() as u64 + 1;
    spans.push(TaskSpan {
        id: span_id,
        task_id: id,
        name: "task_completion".to_string(),
        stage: "Succeeded".to_string(),
        started_at: chrono_now(),
        duration_ms: Some(40),
        details: summary,
    });

    Ok(task.clone())
}

#[tauri::command]
fn cancel_task(
    state: State<AppState>,
    id: String,
    reason: Option<String>,
) -> Result<Task, String> {
    let mut tasks = state.tasks.lock().unwrap();
    let mut spans = state.spans.lock().unwrap();

    let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("Task with id '{id}' not found"))?;

    task.status = "Cancelled".to_string();
    task.epoch += 1;
    task.updated_at = chrono_now();
    task.failure_reason = reason.clone();

    let span_id = spans.len() as u64 + 1;
    spans.push(TaskSpan {
        id: span_id,
        task_id: id,
        name: "task_cancellation".to_string(),
        stage: "Cancelled".to_string(),
        started_at: chrono_now(),
        duration_ms: Some(10),
        details: reason,
    });

    Ok(task.clone())
}

#[tauri::command]
fn list_spans(state: State<AppState>, task_id: String) -> Vec<TaskSpan> {
    let spans = state.spans.lock().unwrap();
    spans.iter().filter(|s| s.task_id == task_id).cloned().collect()
}

#[tauri::command]
fn explain_architecture(query: Option<String>) -> String {
    format!(
        "=== Custos Architecture & End-to-End Vertical Slice ===\n\
        • Domain Core: Immutable Tasks, Concurrency Epoch & Status State Machine\n\
        • Provider SDK: Pluggable LLM Connectors (OpenAI, Claude, Gemini, Local GGUF)\n\
        • Worktree Isolation: Sandboxed Bubblewrap patch verification\n\
        • Human Governance: Explicit ExecutionPermit approval requirement\n\
        • Audit Trail: SQLite Span & Task journals\n\
        Query context: {}",
        query.unwrap_or_else(|| "Overview".to_string())
    )
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{}", d.as_secs(), d.subsec_millis())
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)[..16].to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::default();

    {
        let mut tasks = state.tasks.lock().unwrap();
        tasks.push(Task {
            id: "task_demo_01".to_string(),
            title: "Triển khai xác thực JWT trong crates/runtime".to_string(),
            status: "Succeeded".to_string(),
            epoch: 3,
            created_at: chrono_now(),
            updated_at: chrono_now(),
            metadata: Some(serde_json::json!({ "mode": "code" })),
            result_summary: Some("Đã hoàn tất patch và verify thành công".to_string()),
            failure_reason: None,
            rationale: Some("Permit granted by operator".to_string()),
        });
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            greet,
            list_tasks,
            get_task,
            create_task,
            advance_task,
            complete_task,
            cancel_task,
            list_spans,
            explain_architecture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
