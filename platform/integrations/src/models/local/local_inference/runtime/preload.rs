use custos_provider::errors::ProviderError;

use super::resolution::resolve_model_path;
use super::runtime::{InferenceRuntime, ModelCacheKey, ModelSlotState};
use super::sidecar;

pub async fn preload_model(
    model_name: &str,
    context_size: Option<usize>,
    gpu_layers: Option<u32>,
) -> Result<custos_domain::LoadedModelInstance, ProviderError> {
    let profile = custos_domain::LocalModelLoadProfile {
        model_path: model_name
            .strip_prefix("native:")
            .unwrap_or(model_name)
            .to_string(),
        revision: 0,
        context_size: context_size.unwrap_or(8192) as u64,
        gpu_mode: if gpu_layers == Some(0) {
            "off"
        } else {
            "manual"
        }
        .to_string(),
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
        .ok_or_else(|| {
            ProviderError::ExecutionError(format!("Model path not found: {clean_model}"))
        })?;

    let context_size = usize::try_from(profile.context_size).map_err(|_| {
        ProviderError::ExecutionError("Local model context size exceeds this platform".to_string())
    })?;
    resolved.context_limit = context_size;
    resolved.settings.context_size = Some(profile.context_size.min(u32::MAX as u64) as u32);
    resolved.settings.n_gpu_layers = match profile.gpu_mode.as_str() {
        "off" => Some(0),
        "manual" => profile.gpu_layers,
        "auto" => None,
        "max" => Some(u32::MAX),
        other => {
            return Err(ProviderError::ExecutionError(format!(
                "Unsupported GPU mode: {other}"
            )))
        }
    };
    resolved.settings.n_batch = profile.eval_batch_size;
    resolved.settings.n_threads = profile
        .cpu_threads
        .map(|value| value.min(i32::MAX as u32) as i32);
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
    if let ModelSlotState::Loaded {
        resolved: loaded_resolved,
        loaded_at,
        profile_revision,
        effective_gpu_layers,
        ..
    } = &*state
    {
        if *profile_revision == profile.revision {
            let instance_id = format!(
                "inst_{}",
                &custos_domain::digest(clean_model.as_bytes())[..12]
            );
            return Ok(custos_domain::LoadedModelInstance {
                instance_id,
                model_name: resolved
                    .model_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(clean_model)
                    .to_string(),
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

    if let Some(ref admission) = runtime.get_admission_coordinator() {
        let admission_req = custos_domain::WorkloadAdmissionRequest {
            workload_kind: custos_domain::ComputeWorkloadKind::LocalInference,
            owner_id: format!("infer_{}", clean_model),
            preferred_device_id: None,
            require_exclusive: false,
            required_memory_mb: Some(4096),
            lane: Some(custos_domain::ExecutionLane::Interactive),
        };
        let verdict = admission
            .admit_workload(&admission_req)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        match verdict {
            custos_domain::WorkloadAdmissionVerdict::Admitted { lease } => {
                let mut l_guard = model_slot.lease_id.lock().unwrap();
                *l_guard = Some(lease.lease_id);
            }
            custos_domain::WorkloadAdmissionVerdict::Queued { reason, .. } => {
                drop(cold_load_guard);
                return Err(ProviderError::ExecutionError(format!(
                    "Local inference queued: accelerator busy - {reason}"
                )));
            }
            custos_domain::WorkloadAdmissionVerdict::Rejected { reason } => {
                drop(cold_load_guard);
                return Err(ProviderError::ExecutionError(format!(
                    "Local inference rejected: {reason}"
                )));
            }
        }
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
                tracing::info!(
                    "Evicting previous model to free memory for {clean_model} (single-model mode)"
                );
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
    })
    .await
    {
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
        effective_gpu_layers: if matches!(profile.gpu_mode.as_str(), "manual" | "off") {
            resolved.settings.n_gpu_layers
        } else {
            None
        },
    };
    model_slot.notify.notify_waiters();
    drop(cold_load_guard);

    let instance_id = format!(
        "inst_{}",
        &custos_domain::digest(clean_model.as_bytes())[..12]
    );
    Ok(custos_domain::LoadedModelInstance {
        instance_id,
        model_name: resolved
            .model_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(clean_model)
            .to_string(),
        model_path: resolved.model_path.to_string_lossy().to_string(),
        backend_id: backend.id().to_string(),
        context_limit: resolved.context_limit as u64,
        memory_used_bytes: 0,
        loaded_at,
        profile_revision: profile.revision,
        effective_gpu_layers: if matches!(profile.gpu_mode.as_str(), "manual" | "off") {
            resolved.settings.n_gpu_layers
        } else {
            None
        },
        generation_verified: false,
    })
}
