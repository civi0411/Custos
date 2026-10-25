pub mod config_resolver;
#[cfg(feature = "hf-hub")]
pub use custos_download_manager as download_manager;
#[cfg(feature = "hf-hub")]
pub mod huggingface_auth;
pub mod paths;
pub mod prompt_template;
pub mod provider_utils;
pub mod metal_probe;
pub mod gguf_inspector;
pub mod folder_scanner;
pub mod adapter;
pub use adapter::NativeLocalModelAdapter;
pub mod sidecar;
pub use sidecar::{
    get_loaded_sidecar_instance, get_sidecar_port, is_sidecar_healthy,
    set_sidecar_generation_verified, start_sidecar, stop_sidecar, verify_sidecar_generation,
};

pub mod backend;
#[cfg(feature = "hf-hub")]
pub mod hf_models;
mod llamacpp;
#[cfg(feature = "hf-hub")]
pub mod management;
mod mlx;
pub mod model;
pub(crate) mod multimodal;
#[cfg(feature = "mlx")]
mod native_tool_parsing;
pub(crate) mod thinking_output;
pub mod tool_emulation;
pub mod tool_parsing;

use anyhow::{bail, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use backend::{BackendLoadedModel, LocalInferenceBackend};
use custos_provider::base::{MessageStream, Provider, ProviderDescriptor, ProviderMetadata};
use custos_provider::conversation::message::{Message, MessageContent, SystemNotificationType};
use custos_provider::conversation::token_usage::{ProviderUsage, Usage};
use custos_provider::errors::ProviderError;
use custos_provider::images::ImageFormat;
use custos_provider::model::ModelConfig;
use custos_provider::request_log::{start_log, LoggerHandleExt, RequestLogHandle};
use llamacpp::{LlamaCppBackend, LLAMACPP_BACKEND_ID};
use mlx::{MlxBackend, MLX_BACKEND_ID};
use model::ChatTemplate;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex, Weak};
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

type ModelSlotHandle = Arc<ModelSlot>;

struct ModelSlot {
    state: Mutex<ModelSlotState>,
    notify: Notify,
}

enum ModelSlotState {
    Empty,
    Loading,
    Loaded {
        model: Box<dyn BackendLoadedModel>,
        resolved: Box<ResolvedModelPaths>,
        loaded_at: i64,
        profile_revision: u64,
        effective_gpu_layers: Option<u32>,
    },
}

impl ModelSlot {
    fn new() -> Self {
        Self {
            state: Mutex::new(ModelSlotState::Empty),
            notify: Notify::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ModelCacheKey {
    backend_id: &'static str,
    model_id: String,
    chat_template: ChatTemplate,
}

impl ModelCacheKey {
    fn new(
        backend_id: &'static str,
        model_id: impl Into<String>,
        chat_template: ChatTemplate,
    ) -> Self {
        Self {
            backend_id,
            model_id: model_id.into(),
            chat_template,
        }
    }
}

pub struct InferenceRuntime {
    models: StdMutex<HashMap<ModelCacheKey, ModelSlotHandle>>,
    cold_load_lock: Mutex<()>,
    backends: HashMap<&'static str, Arc<dyn LocalInferenceBackend>>,
}

pub fn builtin_chat_template_names() -> Vec<String> {
    llamacpp::builtin_chat_template_names()
}

static RUNTIME: StdMutex<Weak<InferenceRuntime>> = StdMutex::new(Weak::new());

fn current_runtime() -> Option<Arc<InferenceRuntime>> {
    RUNTIME.lock().expect("runtime lock poisoned").upgrade()
}

impl InferenceRuntime {
    pub fn get_or_init() -> Result<Arc<Self>> {
        let mut guard = RUNTIME.lock().expect("runtime lock poisoned");
        if let Some(runtime) = guard.upgrade() {
            return Ok(runtime);
        }
        let llamacpp_backend: Arc<dyn LocalInferenceBackend> = Arc::new(LlamaCppBackend::new()?);
        let mlx_backend: Arc<dyn LocalInferenceBackend> = Arc::new(MlxBackend::new());
        let mut backends = HashMap::new();
        backends.insert(LLAMACPP_BACKEND_ID, llamacpp_backend);
        backends.insert(MLX_BACKEND_ID, mlx_backend);
        let runtime = Arc::new(Self {
            models: StdMutex::new(HashMap::new()),
            cold_load_lock: Mutex::new(()),
            backends,
        });
        *guard = Arc::downgrade(&runtime);
        Ok(runtime)
    }

    fn default_backend(&self) -> &dyn LocalInferenceBackend {
        self.backends
            .get(LLAMACPP_BACKEND_ID)
            .expect("default local inference backend registered")
            .as_ref()
    }

    fn backend_for_model(
        &self,
        resolved: &ResolvedModelPaths,
    ) -> Result<Arc<dyn LocalInferenceBackend>, ProviderError> {
        let backend_id = resolved
            .backend_id
            .as_deref()
            .unwrap_or(LLAMACPP_BACKEND_ID);
        self.backends.get(backend_id).cloned().ok_or_else(|| {
            ProviderError::ExecutionError(format!(
                "Local inference backend '{}' unavailable",
                backend_id
            ))
        })
    }

    fn get_or_create_model_slot(&self, key: ModelCacheKey) -> ModelSlotHandle {
        let mut map = self.models.lock().expect("model cache lock poisoned");
        map.entry(key)
            .or_insert_with(|| Arc::new(ModelSlot::new()))
            .clone()
    }

    fn other_model_slots(&self, keep_key: &ModelCacheKey) -> Vec<ModelSlotHandle> {
        let map = self.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(key, _)| *key != keep_key)
            .map(|(_, slot)| slot.clone())
            .collect()
    }

    async fn loaded_model_paths(
        &self,
        model_id: &str,
        chat_template: &ChatTemplate,
    ) -> Option<ResolvedModelPaths> {
        let slots = {
            let map = self.models.lock().expect("model cache lock poisoned");
            map.iter()
                .filter(|(key, _)| key.model_id == model_id && &key.chat_template == chat_template)
                .map(|(_, slot)| slot.clone())
                .collect::<Vec<_>>()
        };
        for slot in slots {
            let state = slot.state.lock().await;
            if let ModelSlotState::Loaded { resolved, .. } = &*state {
                return Some(resolved.as_ref().clone());
            }
        }
        None
    }
}

pub async fn is_model_loaded(model_name: &str) -> Result<bool, ProviderError> {
    Ok(loaded_model_ids().await?.contains(model_name))
}

pub async fn loaded_model_ids() -> Result<HashSet<String>, ProviderError> {
    let Some(runtime) = current_runtime() else {
        return Ok(HashSet::new());
    };
    let slots = {
        let map = runtime.models.lock().expect("model cache lock poisoned");
        map.iter()
            .map(|(key, slot)| (key.model_id.clone(), slot.clone()))
            .collect::<Vec<_>>()
    };

    let mut loaded = HashSet::new();
    for (model_id, slot) in slots {
        if let Ok(state) = slot.state.try_lock() {
            if matches!(*state, ModelSlotState::Loaded { .. }) {
                loaded.insert(model_id);
            }
        } else {
            loaded.insert(model_id);
        }
    }
    Ok(loaded)
}

pub async fn evict_model(model_name: &str) -> Result<bool, ProviderError> {
    let mut evicted = false;
    if let Ok(stopped) = sidecar::stop_sidecar().await {
        if stopped {
            evicted = true;
        }
    }

    let Some(runtime) = current_runtime() else {
        return Ok(evicted);
    };
    let slots = {
        let map = runtime.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(key, _)| key.model_id == model_name)
            .map(|(_, slot)| slot.clone())
            .collect::<Vec<_>>()
    };

    for slot in slots {
        let mut state = slot.state.lock().await;
        if matches!(*state, ModelSlotState::Loaded { .. }) {
            *state = ModelSlotState::Empty;
            evicted = true;
            slot.notify.notify_waiters();
        }
    }
    Ok(evicted)
}

pub async fn list_loaded_instances() -> Result<Vec<custos_domain::LoadedModelInstance>, ProviderError> {
    let mut instances = Vec::new();
    if let Some(sidecar_inst) = sidecar::get_loaded_sidecar_instance().await {
        instances.push(sidecar_inst);
    }

    let Some(runtime) = current_runtime() else {
        return Ok(instances);
    };
    let slots = {
        let map = runtime.models.lock().expect("model cache lock poisoned");
        map.iter()
            .map(|(key, slot)| (key.model_id.clone(), key.backend_id, slot.clone()))
            .collect::<Vec<_>>()
    };

    for (model_id, backend_id, slot) in slots {
        if let Ok(state) = slot.state.try_lock() {
            if let ModelSlotState::Loaded { resolved, loaded_at, profile_revision, effective_gpu_layers, .. } = &*state {
                let instance_id = format!("inst_{}", &custos_domain::digest(model_id.as_bytes())[..12]);
                instances.push(custos_domain::LoadedModelInstance {
                    instance_id,
                    model_name: resolved.model_path.file_name().and_then(|n| n.to_str()).unwrap_or(&model_id).to_string(),
                    model_path: resolved.model_path.to_string_lossy().to_string(),
                    backend_id: backend_id.to_string(),
                    context_limit: resolved.context_limit as u64,
                    memory_used_bytes: 0,
                    loaded_at: *loaded_at,
                    profile_revision: *profile_revision,
                    effective_gpu_layers: *effective_gpu_layers,
                    generation_verified: false,
                });
            }
        }
    }
    Ok(instances)
}

pub async fn preload_model(
    model_name: &str,
    context_size: Option<usize>,
    gpu_layers: Option<u32>,
) -> Result<custos_domain::LoadedModelInstance, ProviderError> {
    let profile = custos_domain::LocalModelLoadProfile {
        model_path: model_name.strip_prefix("native:").unwrap_or(model_name).to_string(),
        revision: 0,
        context_size: context_size.unwrap_or(8192) as u64,
        gpu_mode: if gpu_layers == Some(0) { "off" } else { "manual" }.to_string(),
        gpu_layers,
        eval_batch_size: None,
        cpu_threads: None,
        flash_attention: None,
        mmap: true,
        mlock: false,
        idle_timeout_seconds: None,
        updated_at: 0,
    };
    preload_model_with_profile(model_name, &profile).await
}

pub async fn preload_model_with_profile(
    model_name: &str,
    profile: &custos_domain::LocalModelLoadProfile,
) -> Result<custos_domain::LoadedModelInstance, ProviderError> {
    let runtime = InferenceRuntime::get_or_init()
        .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

    let clean_model = model_name.strip_prefix("native:").unwrap_or(model_name);
    let mut resolved = resolve_model_path(clean_model)
        .await
        .map_err(|e| ProviderError::ExecutionError(e.to_string()))?
        .ok_or_else(|| ProviderError::ExecutionError(format!("Model path not found: {clean_model}")))?;

    let context_size = usize::try_from(profile.context_size).map_err(|_| ProviderError::ExecutionError("Local model context size exceeds this platform".to_string()))?;
    resolved.context_limit = context_size;
    resolved.settings.context_size = Some(profile.context_size.min(u32::MAX as u64) as u32);
    resolved.settings.n_gpu_layers = match profile.gpu_mode.as_str() {
        "off" => Some(0),
        "manual" => profile.gpu_layers,
        "auto" => None,
        "max" => Some(u32::MAX),
        other => return Err(ProviderError::ExecutionError(format!("Unsupported GPU mode: {other}"))),
    };
    resolved.settings.n_batch = profile.eval_batch_size;
    resolved.settings.n_threads = profile.cpu_threads.map(|value| value.min(i32::MAX as u32) as i32);
    resolved.settings.flash_attention = profile.flash_attention;
    resolved.settings.use_mlock = profile.mlock;
    resolved.settings.use_mmap = profile.mmap;

    let backend = runtime.backend_for_model(&resolved)?;
    let cache_key = ModelCacheKey::new(
        backend.id(),
        clean_model.to_string(),
        resolved.settings.chat_template.clone(),
    );
    let model_slot = runtime.get_or_create_model_slot(cache_key.clone());

    let cold_load_guard = runtime.cold_load_lock.lock().await;
    let mut state = model_slot.state.lock().await;
    if let ModelSlotState::Loaded { resolved: loaded_resolved, loaded_at, profile_revision, effective_gpu_layers, .. } = &*state {
        if *profile_revision == profile.revision {
            let instance_id = format!("inst_{}", &custos_domain::digest(clean_model.as_bytes())[..12]);
            return Ok(custos_domain::LoadedModelInstance {
                instance_id,
                model_name: resolved.model_path.file_name().and_then(|n| n.to_str()).unwrap_or(clean_model).to_string(),
                model_path: resolved.model_path.to_string_lossy().to_string(),
                backend_id: backend.id().to_string(),
                context_limit: loaded_resolved.context_limit as u64,
                memory_used_bytes: 0,
                loaded_at: *loaded_at,
                profile_revision: *profile_revision,
                effective_gpu_layers: *effective_gpu_layers,
                generation_verified: false,
            });
        }
        *state = ModelSlotState::Empty;
    }

    *state = ModelSlotState::Loading;
    drop(state);

    let force_single_model = std::env::var("CUSTOS_SINGLE_MODEL_LOCAL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if force_single_model {
        let other_slots = runtime.other_model_slots(&cache_key);
        for slot in other_slots {
            let mut other = slot.state.lock().await;
            if matches!(*other, ModelSlotState::Loaded { .. }) {
                tracing::info!("Evicting previous model to free memory for {clean_model} (single-model mode)");
                *other = ModelSlotState::Empty;
            }
        }
    }

    let model_id_clone = clean_model.to_string();
    let resolved_clone = resolved.clone();
    let settings_clone = resolved.settings.clone();
    let backend_clone = backend.clone();

    let loaded = match tokio::task::spawn_blocking(move || {
        backend_clone.load_model(&model_id_clone, &resolved_clone, &settings_clone)
    }).await {
        Ok(Ok(loaded)) => loaded,
        Ok(Err(err)) => {
            if sidecar::find_llama_server_binary().is_some() {
                tracing::info!(
                    model = %clean_model,
                    "In-process load failed ({err}), falling back to modern llama-server sidecar..."
                );
                match sidecar::start_sidecar(&resolved.model_path, profile).await {
                    Ok(instance) => {
                        let mut state = model_slot.state.lock().await;
                        *state = ModelSlotState::Empty;
                        drop(cold_load_guard);
                        return Ok(instance);
                    }
                    Err(sidecar_err) => {
                        let mut state = model_slot.state.lock().await;
                        *state = ModelSlotState::Empty;
                        model_slot.notify.notify_waiters();
                        return Err(ProviderError::ExecutionError(format!(
                            "In-process load failed ({err}) and sidecar fallback failed: {sidecar_err}"
                        )));
                    }
                }
            }
            let mut state = model_slot.state.lock().await;
            *state = ModelSlotState::Empty;
            model_slot.notify.notify_waiters();
            return Err(err);
        }
        Err(err) => {
            if sidecar::find_llama_server_binary().is_some() {
                tracing::info!(
                    model = %clean_model,
                    "In-process task spawn failed ({err}), falling back to modern llama-server sidecar..."
                );
                if let Ok(instance) = sidecar::start_sidecar(&resolved.model_path, profile).await {
                    let mut state = model_slot.state.lock().await;
                    *state = ModelSlotState::Empty;
                    drop(cold_load_guard);
                    return Ok(instance);
                }
            }
            let mut state = model_slot.state.lock().await;
            *state = ModelSlotState::Empty;
            model_slot.notify.notify_waiters();
            return Err(ProviderError::ExecutionError(err.to_string()));
        }
    };

    let mut state = model_slot.state.lock().await;
    let loaded_at = chrono::Utc::now().timestamp_millis();
    *state = ModelSlotState::Loaded {
        model: loaded,
        resolved: Box::new(resolved.clone()),
        loaded_at,
        profile_revision: profile.revision,
        effective_gpu_layers: if matches!(profile.gpu_mode.as_str(), "manual" | "off") { resolved.settings.n_gpu_layers } else { None },
    };
    model_slot.notify.notify_waiters();
    drop(cold_load_guard);

    let instance_id = format!("inst_{}", &custos_domain::digest(clean_model.as_bytes())[..12]);
    Ok(custos_domain::LoadedModelInstance {
        instance_id,
        model_name: resolved.model_path.file_name().and_then(|n| n.to_str()).unwrap_or(clean_model).to_string(),
        model_path: resolved.model_path.to_string_lossy().to_string(),
        backend_id: backend.id().to_string(),
        context_limit: resolved.context_limit as u64,
        memory_used_bytes: 0,
        loaded_at,
        profile_revision: profile.revision,
        effective_gpu_layers: if matches!(profile.gpu_mode.as_str(), "manual" | "off") { resolved.settings.n_gpu_layers } else { None },
        generation_verified: false,
    })
}

const PROVIDER_NAME: &str = "local";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

#[derive(Clone)]
pub(crate) struct ResolvedModelPaths {
    pub model_path: PathBuf,
    pub context_limit: usize,
    pub settings: crate::model::ModelSettings,
    pub mmproj_path: Option<PathBuf>,
    pub backend_id: Option<String>,
    pub draft_model_path: Option<PathBuf>,
}

struct ExplicitModelPath {
    model_path: PathBuf,
    backend_id: &'static str,
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn is_mlx_model_directory(path: &Path) -> Result<bool> {
    if !path.join("config.json").is_file() || !path.join("tokenizer.json").is_file() {
        return Ok(false);
    }

    for entry in std::fs::read_dir(path)? {
        if has_extension(&entry?.path(), "safetensors") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn explicit_model_path(model_id: &str) -> Result<Option<ExplicitModelPath>> {
    let clean_id = model_id.strip_prefix("native:").unwrap_or(model_id);
    if clean_id.is_empty() {
        return Ok(None);
    }

    let path = PathBuf::from(clean_id);
    if !path.exists() {
        return Ok(None);
    }

    if path.is_file() && has_extension(&path, "gguf") {
        return Ok(Some(ExplicitModelPath {
            model_path: path,
            backend_id: LLAMACPP_BACKEND_ID,
        }));
    }

    let mlx_directory = if path.is_dir() {
        Some(path.as_path())
    } else if path.is_file() && has_extension(&path, "safetensors") {
        path.parent()
    } else {
        None
    };
    if let Some(directory) = mlx_directory {
        if is_mlx_model_directory(directory)? {
            mlx::validate_model_directory(directory)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            return Ok(Some(ExplicitModelPath {
                model_path: directory.to_path_buf(),
                backend_id: MLX_BACKEND_ID,
            }));
        }
    }

    bail!(
        "Unsupported local model path '{}': expected a GGUF file or an MLX directory containing config.json, tokenizer.json, and SafeTensors weights",
        path.display()
    )
}

async fn resolve_draft_model_path(model_id: &str) -> Result<Option<PathBuf>> {
    if let Some(explicit) = explicit_model_path(model_id)? {
        return Ok(Some(explicit.model_path));
    }

    #[cfg(feature = "hf-hub")]
    let model_path = hf_models::cached_local_model(model_id)
        .await?
        .map(|model| model.model_path);

    #[cfg(not(feature = "hf-hub"))]
    let model_path = None;

    Ok(model_path)
}

pub fn local_context_limit(model_id: &str) -> Option<usize> {
    config_resolver::model_settings(model_id)
        .ok()
        .and_then(|settings| settings.context_size)
        .map(|limit| limit as usize)
        .filter(|limit| *limit > 0)
}

fn configured_draft_model(
    model_id: &str,
    settings: &crate::model::ModelSettings,
) -> Option<String> {
    settings
        .draft_model
        .clone()
        .or_else(|| {
            config_resolver::string_param("GOOSE_LOCAL_DRAFT_MODEL")
                .ok()
                .flatten()
        })
        .filter(|draft_model| draft_model != model_id)
}

async fn resolve_loaded_model_path(model_id: &str) -> Result<Option<ResolvedModelPaths>> {
    let Some(runtime) = current_runtime() else {
        return Ok(None);
    };
    let mut settings = config_resolver::model_settings(model_id)?;
    let Some(mut resolved) = runtime
        .loaded_model_paths(model_id, &settings.chat_template)
        .await
    else {
        return Ok(None);
    };

    let draft_model = configured_draft_model(model_id, &settings);
    if draft_model != resolved.settings.draft_model {
        resolved.draft_model_path = if let Some(draft_model) = &draft_model {
            resolve_draft_model_path(draft_model).await?
        } else {
            None
        };
    }
    settings.draft_model = draft_model;
    settings.vision_capable = resolved.mmproj_path.is_some();
    settings.mmproj_size_bytes = resolved.settings.mmproj_size_bytes;
    resolved.context_limit = settings.context_size.unwrap_or(0) as usize;
    resolved.settings = settings;
    Ok(Some(resolved))
}

async fn resolve_model_path(model_id: &str) -> Result<Option<ResolvedModelPaths>> {
    if let Some(explicit) = explicit_model_path(model_id)? {
        let mut settings = config_resolver::model_settings(model_id)?;
        settings.vision_capable = false;
        settings.mmproj_size_bytes = 0;
        let draft_model = configured_draft_model(model_id, &settings);
        let draft_model_path = if let Some(draft_model) = &draft_model {
            resolve_draft_model_path(draft_model).await?
        } else {
            None
        };
        settings.draft_model = draft_model;

        return Ok(Some(ResolvedModelPaths {
            model_path: explicit.model_path,
            context_limit: settings.context_size.unwrap_or(0) as usize,
            settings,
            mmproj_path: None,
            backend_id: Some(explicit.backend_id.to_string()),
            draft_model_path,
        }));
    }

    if let Some(resolved) = resolve_loaded_model_path(model_id).await? {
        return Ok(Some(resolved));
    }

    #[cfg(feature = "hf-hub")]
    let resolved = {
        let cached_models = hf_models::cached_local_models().await?;
        let Some(cached) = cached_models.iter().find(|model| model.id == model_id) else {
            return Ok(None);
        };
        let mut settings = config_resolver::model_settings(model_id)?;
        settings.vision_capable = cached.mmproj_path.is_some();
        settings.mmproj_size_bytes = cached.mmproj_size_bytes;
        let draft_model = configured_draft_model(model_id, &settings);
        let draft_model_path = if let Some(draft_model) = &draft_model {
            if let Some(explicit_draft) = explicit_model_path(draft_model)? {
                Some(explicit_draft.model_path)
            } else {
                cached_models
                    .iter()
                    .find(|model| model.id == draft_model.as_str())
                    .map(|model| model.model_path.clone())
            }
        } else {
            None
        };
        settings.draft_model = draft_model;

        Some(ResolvedModelPaths {
            model_path: cached.model_path.clone(),
            context_limit: settings.context_size.unwrap_or(0) as usize,
            settings,
            mmproj_path: cached.mmproj_path.clone(),
            backend_id: Some(cached.backend_id.clone()),
            draft_model_path,
        })
    };

    #[cfg(not(feature = "hf-hub"))]
    let resolved = None;

    Ok(resolved)
}

pub fn available_inference_memory_bytes(runtime: &InferenceRuntime) -> u64 {
    runtime.default_backend().available_memory_bytes()
}

#[cfg(feature = "hf-hub")]
pub fn recommend_local_model(
    runtime: &InferenceRuntime,
    models: &[hf_models::CachedLocalModel],
) -> Option<String> {
    let available_memory = available_inference_memory_bytes(runtime);
    let mut models: Vec<_> = models.iter().filter(|model| model.size_bytes > 0).collect();
    models.sort_by_key(|model| std::cmp::Reverse(model.size_bytes));
    models
        .iter()
        .find(|model| available_memory >= model.size_bytes)
        .or_else(|| models.last())
        .map(|model| model.id.clone())
}

fn build_openai_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    use custos_provider::formats::openai::format_messages;

    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(format_messages(messages, &ImageFormat::OpenAi));
    strip_image_parts_from_messages(&mut arr);
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn build_openai_text_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(messages.iter().filter_map(|m| {
        let content = extract_text_content(m);
        if content.trim().is_empty() {
            return None;
        }
        let role = match m.role {
            rmcp::model::Role::User => "user",
            rmcp::model::Role::Assistant => "assistant",
        };
        Some(json!({"role": role, "content": content}))
    }));
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn convert_text_media_markers(messages: &mut [Value], marker: &str) {
    if marker.is_empty() {
        return;
    }

    for msg in messages {
        let Some(content) = msg.get_mut("content") else {
            continue;
        };

        if let Some(text) = content.as_str() {
            if let Some(parts) = split_media_marker_text(text, marker) {
                *content = json!(parts);
            }
            continue;
        }

        let Some(content_parts) = content.as_array_mut() else {
            continue;
        };
        let mut updated = Vec::new();
        let mut changed = false;
        for part in content_parts.iter() {
            if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    if let Some(parts) = split_media_marker_text(text, marker) {
                        updated.extend(parts);
                        changed = true;
                        continue;
                    }
                }
            }
            updated.push(part.clone());
        }
        if changed {
            *content_parts = updated;
        }
    }
}

fn split_media_marker_text(text: &str, marker: &str) -> Option<Vec<Value>> {
    let mut parts = Vec::new();
    let mut rest = text;
    let mut found_marker = false;
    while let Some((before, after)) = rest.split_once(marker) {
        found_marker = true;
        let before = before.strip_suffix('\n').unwrap_or(before);
        if !before.is_empty() {
            parts.push(json!({"type": "text", "text": before}));
        }
        parts.push(json!({"type": "media_marker", "text": marker}));
        rest = after;
        rest = rest.strip_prefix('\n').unwrap_or(rest);
    }
    if !found_marker {
        return None;
    }
    if !rest.is_empty() {
        parts.push(json!({"type": "text", "text": rest}));
    }
    Some(parts)
}

/// Remove `image_url` content parts from OpenAI-format messages JSON, replacing
/// each with a text note. This prevents an FFI crash in llama.cpp which does not
/// accept `image_url` content-part types.
fn strip_image_parts_from_messages(messages: &mut [Value]) {
    let mut stripped = false;
    for msg in messages.iter_mut() {
        if let Some(content) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            for part in content.iter_mut() {
                if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                    *part = json!({
                        "type": "text",
                        "text": "[Image attached — image input is not supported with the currently selected model]"
                    });
                    stripped = true;
                }
            }
        }
    }
    if stripped {
        tracing::warn!("Stripped image content parts from messages — vision encoder not available for this model");
    }
}

/// Convert a message into plain text for the emulator path's chat history.
///
/// This is the emulator-path counterpart of [`format_messages`] used by the native
/// path. It reconstructs the text-based tool syntax that the emulator prompt teaches
/// the model:
///
/// - `ToolRequest` with a `"command"` argument → `$ command`
/// - `ToolRequest` with a `"code"` argument → `` ```execute_typescript\n…\n``` ``
/// - `ToolResponse` → `Command output:\n…`
///
/// Only `developer__shell` and `code_execution__execute_typescript` style tool calls are
/// recognized (by argument shape, not tool name). Tool calls from other extensions
/// (e.g. custom MCP tools made by a native-tool-calling model earlier in the
/// conversation) are silently dropped, since the emulator path has no syntax to
/// represent them.
fn extract_text_content(msg: &Message) -> String {
    let mut parts = Vec::new();

    for content in &msg.content {
        match content {
            MessageContent::Text(text) => {
                let text = text.text.to_string();
                if !text.trim().is_empty() {
                    parts.push(text);
                }
            }
            MessageContent::ToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    if let Some(cmd) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("command"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("$ {}", cmd));
                    } else if let Some(code) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("code"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("```execute_typescript\n{}\n```", code));
                    }
                }
            }
            MessageContent::ToolResponse(response) => match &response.tool_result {
                Ok(result) => {
                    let mut output_parts = Vec::new();
                    for content_item in &result.content {
                        if let Some(text_content) = content_item.as_text() {
                            output_parts.push(text_content.text.to_string());
                        }
                    }
                    if !output_parts.is_empty() {
                        parts.push(format!("Command output:\n{}", output_parts.join("\n")));
                    }
                }
                Err(e) => {
                    parts.push(format!("Command error: {}", e));
                }
            },
            MessageContent::Image(_) => {
                parts.push(
                    "[Image attached — image input is not supported with the currently selected model]"
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    parts.join("\n")
}

/// Build a `ProviderUsage` and write the request log entry.
fn finalize_usage(
    log: &mut Option<Box<dyn RequestLogHandle>>,
    model_name: String,
    path_label: &str,
    prompt_token_count: usize,
    output_token_count: i32,
    extra_log_fields: Option<(&str, &str)>,
) -> ProviderUsage {
    let input_tokens = prompt_token_count as i32;
    let total_tokens = input_tokens + output_token_count;
    let usage = Usage::new(
        Some(input_tokens),
        Some(output_token_count),
        Some(total_tokens),
    );
    let mut log_json = serde_json::json!({
        "path": path_label,
        "prompt_tokens": input_tokens,
        "output_tokens": output_token_count,
    });
    if let Some((key, value)) = extra_log_fields {
        log_json[key] = serde_json::json!(value);
    }
    let _ = log.write(&log_json, Some(&usage));
    ProviderUsage::new(model_name, usage)
}

type StreamSender =
    tokio::sync::mpsc::Sender<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>;

pub struct LocalInferenceProvider {
    runtime: Arc<InferenceRuntime>,
    name: String,
}

impl LocalInferenceProvider {
    pub fn new(runtime: Arc<InferenceRuntime>) -> Self {
        Self {
            runtime,
            name: PROVIDER_NAME.to_string(),
        }
    }

    pub fn runtime(&self) -> Arc<InferenceRuntime> {
        self.runtime.clone()
    }

    pub async fn from_env() -> Result<Self> {
        let runtime = InferenceRuntime::get_or_init()?;
        Ok(Self::new(runtime))
    }
}

impl ProviderDescriptor for LocalInferenceProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            PROVIDER_NAME,
            "Local Inference",
            "Local inference using GGUF (llama.cpp) and MLX models",
            "",
            vec![],
            "https://github.com/utilityai/llama-cpp-rs",
            vec![],
        )
    }
}

#[async_trait]
impl Provider for LocalInferenceProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        custos_provider::context_limit::ContextLimitResolver::new(&self.name)
            .resolve(model, override_limit, || async {
                Ok(resolve_model_path(model)
                    .await
                    .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
                    .map(|resolved| resolved.context_limit))
            })
            .await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        #[cfg(feature = "hf-hub")]
        let models = hf_models::cached_local_models()
            .await
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
            .into_iter()
            .map(|model| model.id)
            .collect();

        #[cfg(not(feature = "hf-hub"))]
        let models = Vec::new();

        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let resolved = resolve_model_path(&model_config.model_name)
            .await
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
            .ok_or_else(|| {
                #[cfg(feature = "hf-hub")]
                let message = format!(
                    "Model not found in the Hugging Face cache or at the specified path: {}",
                    model_config.model_name
                );
                #[cfg(not(feature = "hf-hub"))]
                let message = format!(
                    "Model not found at the specified path: {}",
                    model_config.model_name
                );
                ProviderError::ExecutionError(message)
            })?;
        let backend = self.runtime.backend_for_model(&resolved)?;
        let model_context_limit = resolved.context_limit;

        // Allow request_params to override thinking
        let mut model_settings = resolved.settings.clone();
        if let Some(false) = model_config
            .request_param::<bool>("enable_thinking")
            .or_else(|| {
                config_resolver::bool_param("GOOSE_LOCAL_ENABLE_THINKING")
                    .ok()
                    .flatten()
            })
        {
            model_settings.enable_thinking = false;
        }

        let cache_key = ModelCacheKey::new(
            backend.id(),
            model_config.model_name.clone(),
            model_settings.chat_template.clone(),
        );
        let model_slot = self.runtime.get_or_create_model_slot(cache_key.clone());
        let runtime = self.runtime.clone();

        let cache_key = cache_key.clone();
        let model_arc = model_slot.clone();
        let backend = backend.clone();
        let model_name = model_config.model_name.clone();
        let temperature = model_config.temperature;
        let max_tokens = model_config.max_tokens;
        let context_limit = model_context_limit;
        let settings = model_settings;
        let resolved_model = resolved.clone();
        let system = system.to_string();
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        let log_payload = serde_json::json!({
            "system": &system,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role { rmcp::model::Role::User => "user", rmcp::model::Role::Assistant => "assistant" },
                    "content": extract_text_content(m),
                })
            }).collect::<Vec<_>>(),
            "tools": tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "settings": {
                "tool_calling": settings.tool_calling,
                "chat_template": settings.chat_template,
                "context_size": settings.context_size,
                "sampling": settings.sampling,
            },
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel::<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >(32);
        let mut log = start_log(model_config, &log_payload)?;

        tokio::spawn(async move {
            // 1. Check if model is already running in active sidecar
            if sidecar::is_sidecar_healthy().await {
                if let Some(active) = sidecar::get_loaded_sidecar_instance().await {
                    let clean_req = model_name.strip_prefix("native:").unwrap_or(&model_name);
                    if active.model_name == clean_req || active.model_path.contains(clean_req) {
                        tracing::info!(model = %model_name, "Routing local inference directly to active sidecar engine");
                        if let Err(e) = sidecar::stream_sidecar_chat(
                            &active.model_name,
                            &system,
                            &messages,
                            temperature,
                            max_tokens,
                            tx.clone(),
                        ).await {
                            let _ = log.error(&e);
                            let _ = tx.send(Err(e)).await;
                        }
                        return;
                    }
                }
            }

            let mut model_load_ms = None;

            // Ensure model is loaded — unload any other models first to free memory.
            loop {
                let state = model_slot.state.lock().await;
                match &*state {
                    ModelSlotState::Loaded { .. } => break,
                    ModelSlotState::Loading => {
                        let notified = model_slot.notify.notified();
                        drop(state);
                        notified.await;
                    }
                    ModelSlotState::Empty => {
                        drop(state);

                        let cold_load_guard = runtime.cold_load_lock.lock().await;
                        let mut state = model_slot.state.lock().await;
                        match &*state {
                            ModelSlotState::Loaded { .. } => break,
                            ModelSlotState::Loading => {
                                let notified = model_slot.notify.notified();
                                drop(state);
                                drop(cold_load_guard);
                                notified.await;
                                continue;
                            }
                            ModelSlotState::Empty => {}
                        }
                        *state = ModelSlotState::Loading;
                        drop(state);

                        let loading_message = Message::assistant().with_system_notification(
                            SystemNotificationType::ProgressMessage,
                            format!("Loading local model {model_name}..."),
                        );
                        if tx.send(Ok((Some(loading_message), None))).await.is_err() {
                            let mut state = model_slot.state.lock().await;
                            *state = ModelSlotState::Empty;
                            model_slot.notify.notify_waiters();
                            return;
                        }

                        let force_single_model = std::env::var("CUSTOS_SINGLE_MODEL_LOCAL")
                            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                            .unwrap_or(false);

                        if force_single_model {
                            let other_model_slots = runtime.other_model_slots(&cache_key);
                            for slot in other_model_slots {
                                let mut other = slot.state.lock().await;
                                if matches!(*other, ModelSlotState::Loaded { .. }) {
                                    tracing::info!("Unloading previous model to free memory (single-model mode)");
                                    *other = ModelSlotState::Empty;
                                }
                            }
                        }

                        let model_id = model_name.clone();
                        let resolved_for_load = resolved_model.clone();
                        let settings_for_load = settings.clone();
                        let backend_for_load = backend.clone();
                        let load_started = std::time::Instant::now();

                        let model_id_arg = model_id.clone();
                        let resolved_arg = resolved_for_load.clone();
                        let settings_arg = settings_for_load.clone();
                        let backend_arg = backend_for_load.clone();

                        let loaded = match tokio::task::spawn_blocking(move || {
                            backend_arg.load_model(
                                &model_id_arg,
                                &resolved_arg,
                                &settings_arg,
                            )
                        })
                        .await
                        {
                            Ok(Ok(loaded)) => loaded,
                            Ok(Err(err)) => {
                                // Fallback to modern llama-server sidecar if in-process fails
                                if sidecar::find_llama_server_binary().is_some() {
                                    tracing::info!(
                                        model = %model_id,
                                        "In-process load failed ({err}), falling back to modern llama-server sidecar..."
                                    );
                                    let sidecar_profile = custos_domain::LocalModelLoadProfile {
                                        model_path: resolved_for_load.model_path.to_string_lossy().to_string(),
                                        revision: 0,
                                        context_size: resolved_for_load.context_limit as u64,
                                        gpu_mode: if resolved_for_load.settings.n_gpu_layers == Some(0) { "off" } else if resolved_for_load.settings.n_gpu_layers == Some(u32::MAX) { "max" } else if resolved_for_load.settings.n_gpu_layers.is_some() { "manual" } else { "auto" }.to_string(),
                                        gpu_layers: resolved_for_load.settings.n_gpu_layers.filter(|layers| *layers != u32::MAX),
                                        eval_batch_size: resolved_for_load.settings.n_batch,
                                        cpu_threads: resolved_for_load.settings.n_threads.and_then(|value| u32::try_from(value).ok()),
                                        flash_attention: resolved_for_load.settings.flash_attention,
                                        mmap: resolved_for_load.settings.use_mmap,
                                        mlock: resolved_for_load.settings.use_mlock,
                                        idle_timeout_seconds: None,
                                        updated_at: 0,
                                    };
                                    match sidecar::start_sidecar(&resolved_for_load.model_path, &sidecar_profile).await {
                                        Ok(instance) => {
                                            let mut state = model_slot.state.lock().await;
                                            *state = ModelSlotState::Empty;
                                            drop(cold_load_guard);
                                            if let Err(e) = sidecar::stream_sidecar_chat(
                                                &instance.model_name,
                                                &system,
                                                &messages,
                                                temperature,
                                                max_tokens,
                                                tx.clone(),
                                            ).await {
                                                let _ = log.error(&e);
                                                let _ = tx.send(Err(e)).await;
                                            }
                                            return;
                                        }
                                        Err(sidecar_err) => {
                                            tracing::error!("Sidecar fallback also failed: {sidecar_err}");
                                        }
                                    }
                                }

                                let mut state = model_slot.state.lock().await;
                                *state = ModelSlotState::Empty;
                                model_slot.notify.notify_waiters();
                                let _ = log.error(&err);
                                let _ = tx.send(Err(err)).await;
                                return;
                            }
                            Err(err) => {
                                let mut state = model_slot.state.lock().await;
                                *state = ModelSlotState::Empty;
                                model_slot.notify.notify_waiters();
                                let err = ProviderError::ExecutionError(err.to_string());
                                let _ = log.error(&err);
                                let _ = tx.send(Err(err)).await;
                                return;
                            }
                        };
                        let elapsed_ms =
                            u64::try_from(load_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        model_load_ms = Some(elapsed_ms);
                        tracing::info!(
                            backend = backend.id(),
                            model = %model_name,
                            model_load_ms = elapsed_ms,
                            "Loaded local inference model"
                        );
                        let _ = log.write(
                            &json!({
                                "path": "model_load",
                                "backend": backend.id(),
                                "model": &model_name,
                                "model_load_ms": elapsed_ms,
                            }),
                            None,
                        );

                        let mut state = model_slot.state.lock().await;
                        *state = ModelSlotState::Loaded {
                            model: loaded,
                            resolved: Box::new(resolved_model.clone()),
                            loaded_at: chrono::Utc::now().timestamp_millis(),
                            profile_revision: 0,
                            effective_gpu_layers: resolved_model.settings.n_gpu_layers,
                        };
                        model_slot.notify.notify_waiters();
                        drop(cold_load_guard);
                        break;
                    }
                }
            }

            tokio::task::spawn_blocking(move || {
                // Macro to log errors before sending them through the channel
                macro_rules! send_err {
                    ($err:expr) => {{
                        let err = $err;
                        let msg = match &err {
                            ProviderError::ExecutionError(s) => s.as_str(),
                            ProviderError::ContextLengthExceeded(s) => s.as_str(),
                            _ => "unknown error",
                        };
                        let _ = log.error(msg);
                        let _ = tx.blocking_send(Err(err));
                        return;
                    }};
                }

                let mut model_guard = model_arc.state.blocking_lock();
                let loaded = match &mut *model_guard {
                    ModelSlotState::Loaded { model, .. } => model.as_mut(),
                    ModelSlotState::Empty | ModelSlotState::Loading => {
                        send_err!(ProviderError::ExecutionError(
                            "Model not loaded".to_string()
                        ));
                    }
                };

                let message_id = Uuid::new_v4().to_string();

                let request = backend::LocalGenerationRequest {
                    model_name,
                    system: &system,
                    messages: &messages,
                    tools: &tools,
                    settings: &settings,
                    temperature,
                    max_tokens,
                    context_limit,
                    model_load_ms,
                    resolved_model: &resolved_model,
                    draft_model_path: resolved_model.draft_model_path.clone(),
                    message_id: &message_id,
                    tx: &tx,
                    log: &mut log,
                };

                let result = backend.generate(loaded, request);

                if let Err(err) = result {
                    let msg = match &err {
                        ProviderError::ExecutionError(s) => s.as_str(),
                        ProviderError::ContextLengthExceeded(s) => s.as_str(),
                        _ => "unknown error",
                    };
                    let _ = log.error(msg);
                    let _ = tx.blocking_send(Err(err));
                }
            });
        });

        Ok(Box::pin(try_stream! {
            while let Some(result) = rx.recv().await {
                let item = result?;
                yield item;
            }

        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn real_chat_gguf_loads_and_generates_when_fixture_is_configured() {
        let Ok(model_path) = std::env::var("CUSTOS_TEST_CHAT_GGUF") else {
            eprintln!("skipping native generation fixture: set CUSTOS_TEST_CHAT_GGUF");
            return;
        };
        assert!(
            std::path::Path::new(&model_path).is_file(),
            "CUSTOS_TEST_CHAT_GGUF must point to a real chat GGUF file"
        );

        let loaded = preload_model(&model_path, Some(2048), Some(0))
            .await
            .expect("GGUF weights must load through the Custos backend");
        assert_eq!(loaded.model_path, model_path);

        let provider = LocalInferenceProvider::from_env()
            .await
            .expect("native inference runtime must initialize");
        let mut config = ModelConfig::new(&model_path);
        config.max_tokens = Some(16);
        let (answer, _) = provider
            .complete(
                &config,
                "Answer briefly.",
                &[Message::user().with_text("Say hello")],
                &[],
            )
            .await
            .expect("loaded GGUF must generate a chat response");
        assert!(
            !extract_text_content(&answer).trim().is_empty(),
            "chat model generated no text"
        );
        evict_model(&model_path)
            .await
            .expect("test model must unload cleanly");
    }

    #[test]
    fn converts_marker_in_string_content_to_media_marker_part() {
        let mut messages = vec![json!({
            "role": "user",
            "content": "look\n<__media__>\nclosely",
        })];

        convert_text_media_markers(&mut messages, "<__media__>");

        assert_eq!(
            messages[0]["content"],
            json!([
                {"type": "text", "text": "look"},
                {"type": "media_marker", "text": "<__media__>"},
                {"type": "text", "text": "closely"},
            ])
        );
    }

    #[test]
    fn converts_marker_inside_text_content_parts() {
        let mut messages = vec![json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "<__media__>describe"},
                {"type": "text", "text": "next"},
                {"type": "media_marker", "text": "<__media__>"},
            ],
        })];

        convert_text_media_markers(&mut messages, "<__media__>");

        assert_eq!(
            messages[0]["content"],
            json!([
                {"type": "media_marker", "text": "<__media__>"},
                {"type": "text", "text": "describe"},
                {"type": "text", "text": "next"},
                {"type": "media_marker", "text": "<__media__>"},
            ])
        );
    }

    #[test]
    fn preserves_balanced_info_tags_in_text_content() {
        let message =
            Message::user().with_text("before <info-msg>executed payload</info-msg> after");

        assert_eq!(
            extract_text_content(&message),
            "before <info-msg>executed payload</info-msg> after"
        );
    }

    #[test]
    fn preserves_unterminated_info_tags_in_text_content() {
        let message = Message::user().with_text("before <info-msg>executed payload");

        assert_eq!(
            extract_text_content(&message),
            "before <info-msg>executed payload"
        );
    }

    #[test]
    fn preserves_legitimate_text_content() {
        let message = Message::user().with_text("ordinary user content");

        assert_eq!(extract_text_content(&message), "ordinary user content");
    }
}
