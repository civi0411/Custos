use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_local_models_lifecycle(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.local_models.profile.get" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.repo.as_ref() {
                    Some(repo) => repo,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => {
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        let hardware =
                            custos_provider::local::metal_probe::probe_local_hardware();
                        let profile = custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: if hardware.is_apple_silicon {
                                Some(true)
                            } else {
                                None
                            },
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        };
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.profile.save" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.repo.as_ref() {
                    Some(repo) => repo,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let descriptor = match repo.get_cached_model_by_path(model_path) {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return ApiResponse::error(req.id, "Model artifact is not registered")
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let context_size = req
                    .params
                    .get("context_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096);
                if context_size < 512 || context_size > descriptor.context_length {
                    return ApiResponse::error(
                        req.id,
                        format!(
                            "context_size must be between 512 and {} for this artifact",
                            descriptor.context_length
                        ),
                    );
                }
                let gpu_mode = req
                    .params
                    .get("gpu_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                if !matches!(gpu_mode, "auto" | "max" | "off" | "manual") {
                    return ApiResponse::error(
                        req.id,
                        "gpu_mode must be auto, max, off, or manual",
                    );
                }
                let gpu_layers = req
                    .params
                    .get("gpu_layers")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(u32::MAX as u64) as u32);
                if gpu_mode == "manual" && gpu_layers.is_none() {
                    return ApiResponse::error(req.id, "manual GPU mode requires gpu_layers");
                }
                let revision = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(current)) => current.revision.saturating_add(1),
                    Ok(None) => 1,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                let profile = custos_domain::LocalModelLoadProfile {
                    model_path: model_path.to_string(),
                    revision,
                    context_size,
                    gpu_mode: gpu_mode.to_string(),
                    gpu_layers,
                    eval_batch_size: req
                        .params
                        .get("eval_batch_size")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    cpu_threads: req
                        .params
                        .get("cpu_threads")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.clamp(1, u32::MAX as u64) as u32),
                    flash_attention: req.params.get("flash_attention").and_then(|v| v.as_bool()),
                    mmap: req
                        .params
                        .get("mmap")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    mlock: req
                        .params
                        .get("mlock")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    idle_timeout_seconds: req
                        .params
                        .get("idle_timeout_seconds")
                        .and_then(|v| v.as_u64()),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                match repo.save_local_model_load_profile(&profile) {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "profile": profile }))
                    }
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.local_models.preflight" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let path = std::path::Path::new(model_path);
                if !path.exists() || !path.is_file() {
                    return ApiResponse::error(
                        req.id,
                        format!("Model file does not exist: {}", model_path),
                    );
                }
                let hardware =
                    custos_provider::local::metal_probe::probe_local_hardware();
                let descriptor =
                    match custos_provider::local::gguf_inspector::inspect_gguf_file(
                        path,
                        "preflight",
                        &hardware,
                    ) {
                        Ok(d) => d,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to parse GGUF file: {}", e),
                            )
                        }
                    };
                let requested_context = req
                    .params
                    .get("requested_context")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096)
                    .min(descriptor.context_length);
                let inspected_context = descriptor.context_length.clamp(1, 8192);
                let inspected_kv_bytes = descriptor
                    .total_memory_required
                    .saturating_sub(descriptor.file_size_bytes);
                let estimated_kv_bytes = inspected_kv_bytes
                    .saturating_mul(requested_context)
                    .checked_div(inspected_context)
                    .unwrap_or(inspected_kv_bytes);
                let estimated_memory_bytes = descriptor
                    .file_size_bytes
                    .saturating_add(estimated_kv_bytes);
                let can_load = estimated_memory_bytes < hardware.total_ram_bytes;
                let mut issues = Vec::new();
                if !can_load {
                    issues.push(format!(
                        "Estimated model and KV-cache memory ({} bytes) is at least the detected physical RAM ({} bytes); loading is blocked by the current preflight policy",
                        estimated_memory_bytes, hardware.total_ram_bytes
                    ));
                }
                if descriptor.compatibility == custos_domain::ModelCompatibility::Red {
                    issues.push("Model memory requirements with large context may exceed memory. Offloading or reduced context recommended.".to_string());
                } else if descriptor.compatibility == custos_domain::ModelCompatibility::Yellow {
                    issues.push(
                        "Model may require partial CPU offloading and run with reduced speed"
                            .to_string(),
                    );
                }
                let response = custos_domain::LocalModelPreflightResponse {
                    model_path: model_path.to_string(),
                    can_load,
                    architecture: descriptor.architecture,
                    quantization: descriptor.quantization,
                    file_size_bytes: descriptor.file_size_bytes,
                    context_limit: requested_context,
                    estimated_memory_bytes,
                    free_ram_bytes: hardware.free_ram_bytes,
                    metal_vram_limit_bytes: hardware.metal_vram_limit_bytes,
                    compatibility: descriptor.compatibility,
                    recommended_gpu_layers: None,
                    issues,
                };
                match serde_json::to_value(&response) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.local_models.load" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let repo = match self.providers.repo.as_ref() {
                    Some(repo) => repo,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match custos_provider::local::preload_model_with_profile(
                    model_path, &profile,
                )
                .await
                {
                    Ok(instance) => match serde_json::to_value(&instance) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to load local model: {e}"))
                    }
                }
            }
            "v1.local_models.ensure" => {
                let model_path = match req.params.get("model_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p.trim(),
                    _ => return ApiResponse::error(req.id, "Missing model_path parameter"),
                };
                let force_reload = req
                    .params
                    .get("force_reload")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = match self.providers.repo.as_ref() {
                    Some(repo) => repo,
                    None => {
                        return ApiResponse::error(req.id, "ProviderRepository not configured")
                    }
                };
                let profile = match repo.get_local_model_load_profile(model_path) {
                    Ok(Some(profile)) => profile,
                    Ok(None) => {
                        let descriptor = match repo.get_cached_model_by_path(model_path) {
                            Ok(Some(value)) => value,
                            Ok(None) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Model artifact is not registered",
                                )
                            }
                            Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        };
                        custos_domain::LocalModelLoadProfile {
                            model_path: model_path.to_string(),
                            revision: 0,
                            context_size: descriptor.context_length.clamp(512, 4096),
                            gpu_mode: "auto".to_string(),
                            gpu_layers: None,
                            eval_batch_size: Some(512),
                            cpu_threads: None,
                            flash_attention: None,
                            mmap: true,
                            mlock: false,
                            idle_timeout_seconds: None,
                            updated_at: 0,
                        }
                    }
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };

                let instances = custos_provider::local::list_loaded_instances()
                    .await
                    .unwrap_or_default();
                let clean_model = model_path.strip_prefix("native:").unwrap_or(model_path);
                let existing = instances
                    .into_iter()
                    .find(|inst| inst.model_path == clean_model || inst.model_name == clean_model);

                if let Some(inst) = existing {
                    if !force_reload && inst.profile_revision == profile.revision {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "instance": inst,
                                "drift_detected": false,
                                "reloaded": false
                            }),
                        );
                    }
                }

                match custos_provider::local::preload_model_with_profile(
                    model_path, &profile,
                )
                .await
                {
                    Ok(instance) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "instance": instance,
                            "drift_detected": false,
                            "reloaded": true
                        }),
                    ),
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to ensure local model: {e}"))
                    }
                }
            }
            "v1.local_models.verify" => {
                let model_opt = req
                    .params
                    .get("model_path")
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str());

                let target_model = match model_opt {
                    Some(m) if !m.trim().is_empty() => m.trim().to_string(),
                    _ => {
                        let instances = custos_provider::local::list_loaded_instances()
                            .await
                            .unwrap_or_default();
                        match instances.first() {
                            Some(inst) => inst.model_name.clone(),
                            None => {
                                return ApiResponse::error(
                                    req.id,
                                    "No active local model instance is loaded to verify",
                                )
                            }
                        }
                    }
                };

                let clean_model = target_model
                    .strip_prefix("native:")
                    .unwrap_or(&target_model);
                let start = std::time::Instant::now();
                match custos_provider::local::verify_sidecar_generation(clean_model).await {
                    Ok(output) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "verified": true,
                                "model": clean_model,
                                "test_output": output,
                                "latency_ms": latency_ms
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Generation probe failed: {e}")),
                }
            }
            "v1.local_models.unload" => {
                let model_identifier = match req
                    .params
                    .get("model_path")
                    .or_else(|| req.params.get("instance_id"))
                    .or_else(|| req.params.get("model_name"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id.trim(),
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing model_path or instance_id parameter",
                        )
                    }
                };
                let clean_model = model_identifier
                    .strip_prefix("native:")
                    .unwrap_or(model_identifier);
                match custos_provider::local::evict_model(clean_model).await {
                    Ok(evicted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "evicted": evicted, "model": clean_model }),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to unload model: {}", e)),
                }
            }
            "v1.local_models.instances" => {
                match custos_provider::local::list_loaded_instances().await {
                    Ok(instances) => {
                        ApiResponse::success(req.id, serde_json::json!({ "instances": instances }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
