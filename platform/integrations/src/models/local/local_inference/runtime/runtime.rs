use anyhow::Result;
use custos_provider::errors::ProviderError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, Notify};

#[cfg(feature = "local-inference-native")]
use super::llamacpp::{self, LlamaCppBackend};
pub const LLAMACPP_BACKEND_ID: &str = "llamacpp";
use super::backend::{BackendLoadedModel, LocalInferenceBackend};
use super::mlx::{MlxBackend, MLX_BACKEND_ID};
use super::model::ChatTemplate;
use super::resolution::ResolvedModelPaths;
use super::sidecar;

pub(crate) type ModelSlotHandle = Arc<ModelSlot>;

pub(crate) struct ModelSlot {
    pub(crate) state: Mutex<ModelSlotState>,
    pub(crate) notify: Notify,
    pub(crate) active_requests: std::sync::atomic::AtomicUsize,
    pub(crate) lease_id: StdMutex<Option<String>>,
}

pub(crate) enum ModelSlotState {
    Empty,
    Loading,
    Loaded {
        model: Box<dyn BackendLoadedModel>,
        #[allow(dead_code)]
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
            active_requests: std::sync::atomic::AtomicUsize::new(0),
            lease_id: StdMutex::new(None),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ModelCacheKey {
    pub(crate) backend_id: &'static str,
    pub(crate) model_id: String,
    pub(crate) chat_template: ChatTemplate,
}

impl ModelCacheKey {
    pub(crate) fn new(
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
    pub(crate) cold_load_lock: Mutex<()>,
    mlx: Arc<dyn LocalInferenceBackend>,
    #[cfg(feature = "local-inference-native")]
    llamacpp: std::sync::OnceLock<Arc<dyn LocalInferenceBackend>>,
    admission: StdMutex<Option<Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>>>,
}

pub fn builtin_chat_template_names() -> Vec<String> {
    #[cfg(feature = "local-inference-native")]
    {
        llamacpp::builtin_chat_template_names()
    }
    #[cfg(not(feature = "local-inference-native"))]
    {
        Vec::new()
    }
}

static RUNTIME: StdMutex<Option<Arc<InferenceRuntime>>> = StdMutex::new(None);

pub(crate) fn current_runtime() -> Option<Arc<InferenceRuntime>> {
    match RUNTIME.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub fn set_admission_coordinator(
    admission: Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>,
) {
    if let Ok(runtime) = InferenceRuntime::get_or_init() {
        runtime.set_admission_coordinator(admission);
    }
}

impl InferenceRuntime {
    pub fn get_or_init() -> Result<Arc<Self>> {
        let mut guard = match RUNTIME.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref runtime) = *guard {
            return Ok(runtime.clone());
        }
        // Defer GPU/Metal/llama.cpp backend initialization until a local model actually loads.
        let mlx: Arc<dyn LocalInferenceBackend> = Arc::new(MlxBackend::new());
        let runtime = Arc::new(Self {
            models: StdMutex::new(HashMap::new()),
            cold_load_lock: Mutex::new(()),
            mlx,
            #[cfg(feature = "local-inference-native")]
            llamacpp: std::sync::OnceLock::new(),
            admission: StdMutex::new(None),
        });
        *guard = Some(runtime.clone());
        Ok(runtime)
    }

    /// Explicit dependency injection constructor wiring WorkloadAdmissionPort (SADE R8)
    pub fn with_admission(
        admission: Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>,
    ) -> Result<Arc<Self>> {
        let runtime = Self::get_or_init()?;
        runtime.set_admission_coordinator(admission);
        Ok(runtime)
    }

    pub fn set_admission_coordinator(
        &self,
        admission: Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>,
    ) {
        let mut guard = match self.admission.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = Some(admission);
    }

    pub fn get_admission_coordinator(
        &self,
    ) -> Option<Arc<dyn custos_core::contracts::compute::WorkloadAdmissionPort>> {
        let guard = match self.admission.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.clone()
    }

    fn ensure_llamacpp_backend(&self) -> Result<Arc<dyn LocalInferenceBackend>, ProviderError> {
        #[cfg(feature = "local-inference-native")]
        {
            if let Some(backend) = self.llamacpp.get() {
                return Ok(backend.clone());
            }
            let backend: Arc<dyn LocalInferenceBackend> = Arc::new(
                LlamaCppBackend::new()
                    .map_err(|e| ProviderError::ExecutionError(e.to_string()))?,
            );
            let _ = self.llamacpp.set(backend.clone());
            Ok(self.llamacpp.get().cloned().unwrap_or(backend))
        }
        #[cfg(not(feature = "local-inference-native"))]
        {
            Err(ProviderError::ExecutionError(
                "Local inference backend 'llamacpp' unavailable".to_string(),
            ))
        }
    }

    pub(crate) fn default_backend(&self) -> Result<Arc<dyn LocalInferenceBackend>, ProviderError> {
        self.ensure_llamacpp_backend()
    }

    pub(crate) fn backend_for_model(
        &self,
        resolved: &ResolvedModelPaths,
    ) -> Result<Arc<dyn LocalInferenceBackend>, ProviderError> {
        let backend_id = resolved
            .backend_id
            .as_deref()
            .unwrap_or(LLAMACPP_BACKEND_ID);
        match backend_id {
            MLX_BACKEND_ID => Ok(self.mlx.clone()),
            LLAMACPP_BACKEND_ID => self.ensure_llamacpp_backend(),
            other => Err(ProviderError::ExecutionError(format!(
                "Local inference backend '{}' unavailable",
                other
            ))),
        }
    }

    pub(crate) fn get_or_create_model_slot(&self, key: ModelCacheKey) -> ModelSlotHandle {
        let mut map = self.models.lock().expect("model cache lock poisoned");
        map.entry(key)
            .or_insert_with(|| Arc::new(ModelSlot::new()))
            .clone()
    }

    pub(crate) fn other_model_slots(&self, keep_key: &ModelCacheKey) -> Vec<ModelSlotHandle> {
        let map = self.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(key, _)| *key != keep_key)
            .map(|(_, slot)| slot.clone())
            .collect()
    }

    pub(crate) async fn loaded_model_paths(
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
        let in_flight = slot
            .active_requests
            .load(std::sync::atomic::Ordering::SeqCst);
        if in_flight > 0 {
            return Err(ProviderError::ExecutionError(format!(
                "Cannot evict model '{}': {} inference request(s) currently in-flight",
                model_name, in_flight
            )));
        }

        let mut state = slot.state.lock().await;
        if matches!(*state, ModelSlotState::Loaded { .. }) {
            *state = ModelSlotState::Empty;
            evicted = true;
            slot.notify.notify_waiters();
        }

        // Release hardware lease if registered
        if let Some(ref admission) = runtime.get_admission_coordinator() {
            let lease_opt = slot.lease_id.lock().unwrap().take();
            if let Some(lease_id) = lease_opt {
                let _ = admission.release_lease(&lease_id).await;
            }
        }
    }
    Ok(evicted)
}

pub async fn list_loaded_instances(
) -> Result<Vec<custos_domain::LoadedModelInstance>, ProviderError> {
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
            if let ModelSlotState::Loaded {
                resolved,
                loaded_at,
                profile_revision,
                effective_gpu_layers,
                ..
            } = &*state
            {
                let instance_id =
                    format!("inst_{}", &custos_domain::digest(model_id.as_bytes())[..12]);
                instances.push(custos_domain::LoadedModelInstance {
                    instance_id,
                    model_name: resolved
                        .model_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&model_id)
                        .to_string(),
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
