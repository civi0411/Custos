use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use custos_provider::base::{MessageStream, Provider, ProviderDescriptor, ProviderMetadata};
use custos_provider::conversation::message::{Message, SystemNotificationType};
use custos_provider::conversation::token_usage::ProviderUsage;
use custos_provider::errors::ProviderError;
use custos_provider::model::ModelConfig;
use custos_provider::request_log::{start_log, LoggerHandleExt};
use rmcp::model::Tool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::local_inference::runtime::{
    backend, config_resolver,
    messages::extract_text_content,
    resolution::resolve_model_path,
    runtime::{InferenceRuntime, ModelCacheKey, ModelSlotHandle, ModelSlotState},
    sidecar,
};

pub(crate) const PROVIDER_NAME: &str = "local";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

#[allow(dead_code)]
pub(crate) type StreamSender =
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
        Ok(Vec::new())
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
                ProviderError::ExecutionError(format!(
                    "Model not found at the specified path: {}",
                    model_config.model_name
                ))
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
        model_slot
            .active_requests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
            struct InFlightGuard(ModelSlotHandle);
            impl Drop for InFlightGuard {
                fn drop(&mut self) {
                    self.0
                        .active_requests
                        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
            let _in_flight_guard = InFlightGuard(model_arc.clone());

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
                        )
                        .await
                        {
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

                        if let Some(ref admission) = runtime.get_admission_coordinator() {
                            let admission_req = custos_domain::WorkloadAdmissionRequest {
                                workload_kind: custos_domain::ComputeWorkloadKind::LocalInference,
                                owner_id: format!("infer_{}", model_name),
                                preferred_device_id: None,
                                require_exclusive: false,
                                required_memory_mb: Some(4096),
                                lane: Some(custos_domain::ExecutionLane::Interactive),
                            };
                            match admission.admit_workload(&admission_req).await {
                                Ok(custos_domain::WorkloadAdmissionVerdict::Admitted { lease }) => {
                                    let mut l_guard = model_slot.lease_id.lock().unwrap();
                                    *l_guard = Some(lease.lease_id);
                                }
                                Ok(custos_domain::WorkloadAdmissionVerdict::Queued {
                                    reason,
                                    ..
                                }) => {
                                    drop(cold_load_guard);
                                    let err = ProviderError::ExecutionError(format!(
                                        "Local inference queued: accelerator busy with exclusive workload - {reason}"
                                    ));
                                    let _ = log.error(&err);
                                    let _ = tx.send(Err(err)).await;
                                    return;
                                }
                                Ok(custos_domain::WorkloadAdmissionVerdict::Rejected {
                                    reason,
                                }) => {
                                    drop(cold_load_guard);
                                    let err = ProviderError::ExecutionError(format!(
                                        "Local inference rejected: {reason}"
                                    ));
                                    let _ = log.error(&err);
                                    let _ = tx.send(Err(err)).await;
                                    return;
                                }
                                Err(e) => {
                                    drop(cold_load_guard);
                                    let err = ProviderError::ExecutionError(e.to_string());
                                    let _ = log.error(&err);
                                    let _ = tx.send(Err(err)).await;
                                    return;
                                }
                            }
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
                            backend_arg.load_model(&model_id_arg, &resolved_arg, &settings_arg)
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
                                        model_path: resolved_for_load
                                            .model_path
                                            .to_string_lossy()
                                            .to_string(),
                                        revision: 0,
                                        context_size: resolved_for_load.context_limit as u64,
                                        gpu_mode: if resolved_for_load.settings.n_gpu_layers
                                            == Some(0)
                                        {
                                            "off"
                                        } else if resolved_for_load.settings.n_gpu_layers
                                            == Some(u32::MAX)
                                        {
                                            "max"
                                        } else if resolved_for_load.settings.n_gpu_layers.is_some()
                                        {
                                            "manual"
                                        } else {
                                            "auto"
                                        }
                                        .to_string(),
                                        gpu_layers: resolved_for_load
                                            .settings
                                            .n_gpu_layers
                                            .filter(|layers| *layers != u32::MAX),
                                        eval_batch_size: resolved_for_load.settings.n_batch,
                                        cpu_threads: resolved_for_load
                                            .settings
                                            .n_threads
                                            .and_then(|value| u32::try_from(value).ok()),
                                        flash_attention: resolved_for_load.settings.flash_attention,
                                        mmap: resolved_for_load.settings.use_mmap,
                                        mlock: resolved_for_load.settings.use_mlock,
                                        idle_timeout_seconds: None,
                                        updated_at: 0,
                                    };
                                    match sidecar::start_sidecar(
                                        &resolved_for_load.model_path,
                                        &sidecar_profile,
                                    )
                                    .await
                                    {
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
                                            )
                                            .await
                                            {
                                                let _ = log.error(&e);
                                                let _ = tx.send(Err(e)).await;
                                            }
                                            return;
                                        }
                                        Err(sidecar_err) => {
                                            tracing::error!(
                                                "Sidecar fallback also failed: {sidecar_err}"
                                            );
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
