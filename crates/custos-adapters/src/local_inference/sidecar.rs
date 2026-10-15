//! Managed Native Sidecar Engine for Local Model Inference
//!
//! Spawns and manages a standalone modern `llama-server` process (with full Apple Metal GPU acceleration),
//! matching LM Studio's high-performance subprocess architecture.
//! This allows loading ALL GGUF models (including Qwopus 3.5 9B/27B MTP, Qwen 2.5/3.5 SSM, DeepSeek, etc.)
//! without tensor mismatch errors or in-process memory fragmentation.

use custos_domain::LoadedModelInstance;
use custos_provider::errors::ProviderError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

pub const DEFAULT_SIDECAR_PORT: u16 = 8089;
static SIDECAR_PORT: AtomicU16 = AtomicU16::new(DEFAULT_SIDECAR_PORT);

/// Finds the most modern and capable `llama-server` binary available on macOS.
pub fn find_llama_server_binary() -> Option<PathBuf> {
    // 1. Homebrew Metal build (up-to-date with latest model architectures & MTP)
    let brew_path = PathBuf::from("/opt/homebrew/bin/llama-server");
    if brew_path.exists() {
        return Some(brew_path);
    }

    // 2. LM Studio's internal bundled llama-server build
    let lms_paths = [
        "/Users/mac/.lmstudio/extensions/backends/llama.cpp-mac-arm64-apple-metal-advsimd-2.52.0/llama-server",
        "/Users/mac/.lmstudio/extensions/backends/llama.cpp-mac-arm64-apple-metal-advsimd-2.51.0/llama-server",
        "/Users/mac/.lmstudio/extensions/backends/llama.cpp-mac-arm64-apple-metal-advsimd-2.38.0/llama-server",
    ];
    for p in lms_paths {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }

    // 3. System PATH fallback
    if let Ok(path) = which::which("llama-server") {
        return Some(path);
    }

    None
}

/// Active sidecar process metadata and child handle.
pub struct ActiveSidecar {
    pub child: Child,
    pub model_path: PathBuf,
    pub model_name: String,
    pub port: u16,
    pub context_size: usize,
    pub loaded_at: i64,
}

static SIDECAR_MANAGER: std::sync::OnceLock<Arc<Mutex<Option<ActiveSidecar>>>> =
    std::sync::OnceLock::new();

fn get_manager() -> Arc<Mutex<Option<ActiveSidecar>>> {
    SIDECAR_MANAGER
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

/// Returns the port the sidecar is configured to run on.
pub fn get_sidecar_port() -> u16 {
    SIDECAR_PORT.load(Ordering::Relaxed)
}

/// Checks if an active sidecar is currently running and healthy.
pub async fn is_sidecar_healthy() -> bool {
    let port = get_sidecar_port();
    let url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .unwrap_or_default();

    match client.get(&url).send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    }
}

/// Returns the currently loaded sidecar instance, if active and running.
pub async fn get_loaded_sidecar_instance() -> Option<LoadedModelInstance> {
    let manager = get_manager();
    let guard = manager.lock().await;
    let active = guard.as_ref()?;

    if !is_sidecar_healthy().await {
        return None;
    }

    let instance_id = format!(
        "sidecar_{}",
        &custos_domain::digest(active.model_name.as_bytes())[..12]
    );

    Some(LoadedModelInstance {
        instance_id,
        model_name: active.model_name.clone(),
        model_path: active.model_path.to_string_lossy().to_string(),
        backend_id: "llama-server-sidecar".to_string(),
        context_limit: active.context_size as u64,
        memory_used_bytes: 0,
        loaded_at: active.loaded_at,
    })
}

/// Spawns a dedicated `llama-server` process for the specified model.
/// Automatically terminates any existing model to reclaim 100% of GPU VRAM.
pub async fn start_sidecar(
    model_path: &Path,
    context_size: usize,
    gpu_layers: u32,
) -> Result<LoadedModelInstance, ProviderError> {
    let binary = find_llama_server_binary().ok_or_else(|| {
        ProviderError::ExecutionError(
            "llama-server binary not found on system. Please install via `brew install llama.cpp` or LM Studio.".to_string(),
        )
    })?;

    if !model_path.exists() {
        return Err(ProviderError::ExecutionError(format!(
            "Model file not found at: {}",
            model_path.display()
        )));
    }

    let model_name = model_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("local-model")
        .to_string();

    let port = get_sidecar_port();
    let manager = get_manager();
    let mut guard = manager.lock().await;

    // 1. Terminate existing sidecar if running
    if let Some(mut existing) = guard.take() {
        info!(
            previous = %existing.model_name,
            "Evicting existing sidecar model to free GPU RAM"
        );
        let _ = existing.child.kill().await;
        let _ = existing.child.wait().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!(
        model = %model_name,
        path = %model_path.display(),
        binary = %binary.display(),
        port = port,
        context_size = context_size,
        gpu_layers = gpu_layers,
        "Spawning managed llama-server sidecar"
    );

    // 2. Configure Command matching LM Studio's high-performance Metal arguments
    let mut cmd = Command::new(&binary);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-c")
        .arg(context_size.to_string())
        .arg("-ngl")
        .arg(gpu_layers.to_string())
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("-np")
        .arg("4")
        .arg("--no-warmup")
        .kill_on_drop(true);

    // Stdout/Stderr inheritance or piping
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        ProviderError::ExecutionError(format!("Failed to spawn llama-server sidecar: {e}"))
    })?;

    // 3. Poll health check until model finishes loading into GPU
    let start_wait = Instant::now();
    let max_wait = Duration::from_secs(60);
    let health_url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .unwrap_or_default();

    let mut healthy = false;
    while start_wait.elapsed() < max_wait {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Ok(res) = client.get(&health_url).send().await {
            if res.status().is_success() {
                healthy = true;
                break;
            }
        }
    }

    if !healthy {
        let mut failed_child = child;
        let _ = failed_child.kill().await;
        return Err(ProviderError::ExecutionError(format!(
            "llama-server timed out after {}s waiting for model to load",
            max_wait.as_secs()
        )));
    }

    let loaded_at = chrono::Utc::now().timestamp_millis();
    let instance_id = format!(
        "sidecar_{}",
        &custos_domain::digest(model_name.as_bytes())[..12]
    );

    let instance = LoadedModelInstance {
        instance_id,
        model_name: model_name.clone(),
        model_path: model_path.to_string_lossy().to_string(),
        backend_id: "llama-server-sidecar".to_string(),
        context_limit: context_size as u64,
        memory_used_bytes: 0,
        loaded_at,
    };

    *guard = Some(ActiveSidecar {
        child,
        model_path: model_path.to_path_buf(),
        model_name,
        port,
        context_size,
        loaded_at,
    });

    info!(
        model = %instance.model_name,
        elapsed_ms = start_wait.elapsed().as_millis(),
        "llama-server sidecar successfully initialized and healthy"
    );

    Ok(instance)
}

/// Terminates the active sidecar process and completely frees GPU VRAM.
pub async fn stop_sidecar() -> Result<bool, ProviderError> {
    let manager = get_manager();
    let mut guard = manager.lock().await;

    if let Some(mut active) = guard.take() {
        info!(model = %active.model_name, "Stopping llama-server sidecar process");
        let _ = active.child.kill().await;
        let _ = active.child.wait().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Streams chat completions directly from the managed sidecar via its OpenAI-compatible endpoint.
pub async fn stream_sidecar_chat(
    model_name: &str,
    system: &str,
    messages: &[custos_provider::conversation::message::Message],
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    tx: tokio::sync::mpsc::Sender<Result<(Option<custos_provider::conversation::message::Message>, Option<custos_provider::conversation::token_usage::ProviderUsage>), ProviderError>>,
) -> Result<(), ProviderError> {
    use futures::StreamExt;

    let port = get_sidecar_port();
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");

    let mut msgs: Vec<serde_json::Value> = Vec::new();
    if !system.trim().is_empty() {
        msgs.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    for m in messages {
        let role = match m.role {
            rmcp::model::Role::User => "user",
            rmcp::model::Role::Assistant => "assistant",
        };
        let text = super::extract_text_content(m);
        if !text.trim().is_empty() {
            msgs.push(serde_json::json!({
                "role": role,
                "content": text
            }));
        }
    }

    let mut body = serde_json::json!({
        "model": model_name,
        "messages": msgs,
        "stream": true,
    });
    if let Some(t) = temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(m) = max_tokens {
        body["max_tokens"] = serde_json::json!(m);
    }

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Sidecar HTTP request failed: {e}")))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ProviderError::ExecutionError(format!("Sidecar returned error: {err_text}")));
    }

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_res) = stream.next().await {
        let chunk = match chunk_res {
            Ok(c) => c,
            Err(e) => return Err(ProviderError::ExecutionError(e.to_string())),
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer.drain(..=pos);

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    break;
                }

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                        if let Some(choice) = choices.first() {
                            if let Some(delta) = choice.get("delta") {
                                let text = delta.get("content").and_then(|v| v.as_str())
                                    .or_else(|| delta.get("reasoning_content").and_then(|v| v.as_str()))
                                    .unwrap_or("");
                                if !text.is_empty() {
                                    let msg = custos_provider::conversation::message::Message::assistant().with_text(text);
                                    let _ = tx.send(Ok((Some(msg), None))).await;
                                }
                            }
                        }
                    }
                    if let Some(usage_val) = val.get("usage") {
                        let prompt_tokens = usage_val.get("prompt_tokens").and_then(|v| v.as_i64()).map(|v| v as i32);
                        let completion_tokens = usage_val.get("completion_tokens").and_then(|v| v.as_i64()).map(|v| v as i32);
                        let total_tokens = usage_val.get("total_tokens").and_then(|v| v.as_i64()).map(|v| v as i32);
                        let usage = custos_provider::conversation::token_usage::Usage::new(
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                        );
                        let provider_usage = custos_provider::conversation::token_usage::ProviderUsage::new(
                            model_name.to_string(),
                            usage,
                        );
                        let _ = tx.send(Ok((None, Some(provider_usage)))).await;
                    }
                }
            }
        }
    }

    Ok(())
}
