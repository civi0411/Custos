//! Managed Native Sidecar Engine for Local Model Inference
//!
//! Spawns and manages a separately installed `llama-server` process.
//! Compatibility is established by a real load attempt; discovery never claims
//! that every GGUF architecture is supported.

use custos_domain::LoadedModelInstance;
use custos_provider::errors::ProviderError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

static SIDECAR_PORT: AtomicU16 = AtomicU16::new(0);

/// Finds an explicitly configured or system-installed `llama-server`.
/// Private runtime directories owned by other applications are intentionally
/// excluded: they are not a stable or licensed Custos runtime boundary.
pub fn find_llama_server_binary() -> Option<PathBuf> {
    if let Ok(configured) = std::env::var("CUSTOS_LLAMA_SERVER") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }

    let brew_path = PathBuf::from("/opt/homebrew/bin/llama-server");
    if brew_path.is_file() {
        return Some(brew_path);
    }
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
    pub gpu_layers: Option<u32>,
    pub profile_revision: u64,
    pub generation_verified: bool,
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
    if port == 0 {
        return false;
    }
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
    let (model_name, model_path, context_size, loaded_at, profile_revision, gpu_layers, generation_verified) = {
        let guard = manager.lock().await;
        let active = guard.as_ref()?;
        (
            active.model_name.clone(), active.model_path.clone(), active.context_size,
            active.loaded_at, active.profile_revision, active.gpu_layers,
            active.generation_verified,
        )
    };

    if !is_sidecar_healthy().await {
        return None;
    }

    let instance_id = format!(
        "sidecar_{}",
        &custos_domain::digest(model_name.as_bytes())[..12]
    );

    Some(LoadedModelInstance {
        instance_id,
        model_name,
        model_path: model_path.to_string_lossy().to_string(),
        backend_id: "llama-server-sidecar".to_string(),
        context_limit: context_size as u64,
        memory_used_bytes: 0,
        loaded_at,
        profile_revision,
        effective_gpu_layers: gpu_layers,
        generation_verified,
    })
}

/// Spawns a dedicated `llama-server` process for the specified model.
/// A healthy existing process is replaced only after the candidate is healthy.
pub async fn start_sidecar(
    model_path: &Path,
    profile: &custos_domain::LocalModelLoadProfile,
) -> Result<LoadedModelInstance, ProviderError> {
    let context_size = usize::try_from(profile.context_size).map_err(|_| {
        ProviderError::ExecutionError("Local model context size exceeds this platform".to_string())
    })?;
    let gpu_layers = match profile.gpu_mode.as_str() {
        "auto" => None,
        "max" => Some(u32::MAX),
        "off" => Some(0),
        "manual" => profile.gpu_layers,
        other => return Err(ProviderError::ExecutionError(format!("Unsupported GPU mode: {other}"))),
    };
    let binary = find_llama_server_binary().ok_or_else(|| {
        ProviderError::ExecutionError(
            "llama-server was not found. Install llama.cpp or set CUSTOS_LLAMA_SERVER to an audited executable.".to_string(),
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

    let listener =
        std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to allocate a loopback port: {e}"))
        })?;
    let port = listener
        .local_addr()
        .map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to inspect loopback port: {e}"))
        })?
        .port();
    drop(listener);
    let manager = get_manager();
    let mut guard = manager.lock().await;

    info!(
        model = %model_name,
        path = %model_path.display(),
        binary = %binary.display(),
        port = port,
        context_size = context_size,
        gpu_layers = ?gpu_layers,
        "Spawning managed llama-server sidecar"
    );

    // Configure the independent llama.cpp process using the requested profile.
    let mut cmd = Command::new(&binary);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-c")
        .arg(context_size.to_string())
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("-np")
        .arg("1")
        .kill_on_drop(true);
    if let Some(layers) = gpu_layers {
        cmd.arg("-ngl").arg(if layers == u32::MAX { "999".to_string() } else { layers.to_string() });
    }
    if let Some(batch) = profile.eval_batch_size {
        cmd.arg("-b").arg(batch.to_string());
    }
    if let Some(threads) = profile.cpu_threads {
        cmd.arg("-t").arg(threads.to_string());
    }
    if let Some(flash_attention) = profile.flash_attention {
        cmd.arg("-fa").arg(if flash_attention { "on" } else { "off" });
    }
    if !profile.mmap {
        cmd.arg("--no-mmap");
    }
    if profile.mlock {
        cmd.arg("--mlock");
    }

    // Stdout/Stderr inheritance or piping
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        ProviderError::ExecutionError(format!("Failed to spawn llama-server sidecar: {e}"))
    })?;

    let stderr_buffer = Arc::new(Mutex::new(std::collections::VecDeque::<String>::new()));
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(drain_diagnostics(stdout, "stdout", None));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(drain_diagnostics(stderr, "stderr", Some(stderr_buffer.clone())));
    }

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
        if let Ok(Some(status)) = child.try_wait() {
            let last_errors: Vec<String> = stderr_buffer.lock().await.iter().cloned().collect();
            let details = if last_errors.is_empty() {
                "No stderr output recorded from llama-server process.".to_string()
            } else {
                last_errors.join("\n")
            };
            return Err(ProviderError::ExecutionError(format!(
                "llama-server exited during model load with status {status}. Engine diagnostics:\n{details}"
            )));
        }
        if let Ok(res) = client.get(&health_url).send().await {
            if res.status().is_success() {
                let models_url = format!("http://127.0.0.1:{port}/v1/models");
                if let Ok(models_res) = client.get(&models_url).send().await {
                    if let Ok(json) = models_res.json::<serde_json::Value>().await {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            let matches_model = data.iter().any(|item| {
                                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                id.eq_ignore_ascii_case(&model_name)
                                    || model_path.to_string_lossy().contains(id)
                                    || id.contains(&model_name)
                            });
                            if matches_model || !data.is_empty() {
                                healthy = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if !healthy {
        let mut failed_child = child;
        let _ = failed_child.kill().await;
        let last_errors: Vec<String> = stderr_buffer.lock().await.iter().cloned().collect();
        let details = if last_errors.is_empty() {
            "No stderr recorded.".to_string()
        } else {
            last_errors.join("\n")
        };
        return Err(ProviderError::ExecutionError(format!(
            "llama-server timed out after {}s waiting for model to load. Last engine diagnostics:\n{}",
            max_wait.as_secs(), details
        )));
    }

    let loaded_at = chrono::Utc::now().timestamp_millis();
    let instance_id = format!(
        "sidecar_{}",
        &custos_domain::digest(model_name.as_bytes())[..12]
    );

    let reported_gpu_layers = if matches!(profile.gpu_mode.as_str(), "manual" | "off") { gpu_layers } else { None };
    let instance = LoadedModelInstance {
        instance_id,
        model_name: model_name.clone(),
        model_path: model_path.to_string_lossy().to_string(),
        backend_id: "llama-server-sidecar".to_string(),
        context_limit: context_size as u64,
        memory_used_bytes: 0,
        loaded_at,
        profile_revision: profile.revision,
        effective_gpu_layers: reported_gpu_layers,
        generation_verified: false,
    };

    // Only replace the active instance after the candidate has proved healthy.
    // A failed replacement therefore leaves the previous model available.
    if let Some(mut existing) = guard.take() {
        info!(previous = %existing.model_name, "Replacing healthy llama-server sidecar");
        let _ = existing.child.kill().await;
        let _ = existing.child.wait().await;
    }
    SIDECAR_PORT.store(port, Ordering::Relaxed);

    *guard = Some(ActiveSidecar {
        child,
        model_path: model_path.to_path_buf(),
        model_name,
        port,
        context_size,
        loaded_at,
        gpu_layers: reported_gpu_layers,
        profile_revision: profile.revision,
        generation_verified: false,
    });

    info!(
        model = %instance.model_name,
        elapsed_ms = start_wait.elapsed().as_millis(),
        "llama-server sidecar successfully initialized and healthy"
    );

    Ok(instance)
}

async fn drain_diagnostics<R>(
    reader: R,
    stream: &'static str,
    buffer: Option<Arc<Mutex<std::collections::VecDeque<String>>>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut lines = BufReader::new(reader).lines();
    let mut remaining = 2048usize;
    while remaining > 0 {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let truncated: String = line.chars().take(2048).collect();
                tracing::debug!(sidecar_stream = stream, message = %truncated, "llama-server diagnostic");
                if let Some(buf) = &buffer {
                    let mut b = buf.lock().await;
                    if b.len() >= 30 {
                        b.pop_front();
                    }
                    b.push_back(truncated);
                }
                remaining -= 1;
            }
            Ok(None) | Err(_) => break,
        }
    }
}

/// Marks the resident sidecar model generation as verified after first token output.
pub async fn set_sidecar_generation_verified(verified: bool) {
    let manager = get_manager();
    let mut guard = manager.lock().await;
    if let Some(active) = guard.as_mut() {
        active.generation_verified = verified;
    }
}

/// Runs a fast single-token generation probe to verify tensor compatibility and prompt templates.
pub async fn verify_sidecar_generation(model_name: &str) -> Result<String, ProviderError> {
    let port = get_sidecar_port();
    if port == 0 || !is_sidecar_healthy().await {
        return Err(ProviderError::ExecutionError(
            "No active sidecar instance is healthy".to_string(),
        ));
    }
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let body = serde_json::json!({
        "model": model_name,
        "messages": [{ "role": "user", "content": "ping" }],
        "max_tokens": 2,
        "stream": false
    });
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to connect to sidecar for verification: {e}")))?;
    let status = res.status();
    if !status.is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ProviderError::ExecutionError(format!(
            "Sidecar verification probe failed (HTTP {status}): {err_text}"
        )));
    }
    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to parse sidecar verification response: {e}")))?;
    let content = json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("ok")
        .to_string();

    set_sidecar_generation_verified(true).await;
    Ok(content)
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
        SIDECAR_PORT.store(0, Ordering::Relaxed);
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
    tx: tokio::sync::mpsc::Sender<
        Result<
            (
                Option<custos_provider::conversation::message::Message>,
                Option<custos_provider::conversation::token_usage::ProviderUsage>,
            ),
            ProviderError,
        >,
    >,
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
    let res =
        client.post(&url).json(&body).send().await.map_err(|e| {
            ProviderError::ExecutionError(format!("Sidecar HTTP request failed: {e}"))
        })?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ProviderError::ExecutionError(format!(
            "Sidecar returned error: {err_text}"
        )));
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
                                let text = delta
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| {
                                        delta.get("reasoning_content").and_then(|v| v.as_str())
                                    })
                                    .unwrap_or("");
                                if !text.is_empty() {
                                    let msg =
                                        custos_provider::conversation::message::Message::assistant(
                                        )
                                        .with_text(text);
                                    let _ = tx.send(Ok((Some(msg), None))).await;
                                }
                            }
                        }
                    }
                    if let Some(usage_val) = val.get("usage") {
                        let prompt_tokens = usage_val
                            .get("prompt_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let completion_tokens = usage_val
                            .get("completion_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let total_tokens = usage_val
                            .get("total_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let usage = custos_provider::conversation::token_usage::Usage::new(
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                        );
                        let provider_usage =
                            custos_provider::conversation::token_usage::ProviderUsage::new(
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
