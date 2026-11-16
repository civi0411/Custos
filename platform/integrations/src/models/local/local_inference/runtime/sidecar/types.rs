use custos_domain::LoadedModelInstance;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tokio::sync::Mutex;

pub(crate) static SIDECAR_PORT: AtomicU16 = AtomicU16::new(0);

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

pub(crate) fn get_manager() -> Arc<Mutex<Option<ActiveSidecar>>> {
    SIDECAR_MANAGER
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

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
    let (
        model_name,
        model_path,
        context_size,
        loaded_at,
        profile_revision,
        gpu_layers,
        generation_verified,
    ) = {
        let guard = manager.lock().await;
        let active = guard.as_ref()?;
        (
            active.model_name.clone(),
            active.model_path.clone(),
            active.context_size,
            active.loaded_at,
            active.profile_revision,
            active.gpu_layers,
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
