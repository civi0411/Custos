use std::any::Any;
use std::path::{Path, PathBuf};

use safemlx::{Device, DeviceType, Stream};
use safemlx_lm::gemma4_mtp::generate_gemma4_mtp;
use safemlx_lm::models::{gemma4_assistant::load_gemma4_assistant_model, LoadedModel, Model};
use safemlx_lm_utils::tokenizer::Tokenizer;

use crate::local_inference::backend::{
    BackendLoadedModel, LocalGenerationRequest, LocalInferenceBackend,
};
use crate::local_inference::model::{ModelSettings, ToolCallingMode};
use crate::local_inference::tool_emulation::CODE_EXECUTION_TOOL;
use crate::local_inference::ResolvedModelPaths;
use custos_provider::conversation::token_usage::{DraftStats, ProviderStats, ProviderUsage, Usage};
use custos_provider::errors::ProviderError;
use custos_provider::request_log::LoggerHandleExt;

use super::generation::{
    emit_generated_response, generate_single_model, mlx_error, mlx_max_tokens, prng_key,
    sampling, MlxGeneration, MlxStreamEmitter,
};
use super::prompt::{build_prompt, ToolMode};

pub(crate) const MLX_BACKEND_ID: &str = "mlx";

pub(crate) struct MlxBackend;

impl MlxBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

pub(crate) fn validate_model_directory(path: &Path) -> Result<(), ProviderError> {
    crate::local_inference::mlx::validation::validate_snapshot_files(path)?;
    let config_path = path.join("config.json");
    let config = std::fs::read(&config_path).map_err(mlx_error)?;
    let config: serde_json::Value = serde_json::from_slice(&config).map_err(mlx_error)?;
    if let Some(reason) = safemlx_lm::check_model_config(&config).unsupported_reason() {
        return Err(ProviderError::ExecutionError(format!(
            "Unsupported MLX model at '{}': {}",
            path.display(),
            reason
        )));
    }
    Ok(())
}

struct MlxLoadedModel {
    model: LoadedModel,
    tokenizer: Tokenizer,
    model_dir: PathBuf,
    stop_token_ids: Vec<u32>,
}

impl BackendLoadedModel for MlxLoadedModel {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn model_dir_from_path(path: &Path) -> Result<PathBuf, ProviderError> {
    if path.is_dir() {
        Ok(path.to_path_buf())
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| mlx_error("MLX model path has no parent directory"))
    }
}

fn mlx_stop_token_ids(model: &LoadedModel, model_dir: &Path) -> Vec<u32> {
    let mut ids = model.eos_token_ids().to_vec();
    for id in generation_config_eos_token_ids(model_dir) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

fn generation_config_eos_token_ids(model_dir: &Path) -> Vec<u32> {
    let Ok(config_json) = std::fs::read_to_string(model_dir.join("generation_config.json")) else {
        return Vec::new();
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_json) else {
        return Vec::new();
    };
    match config.get("eos_token_id") {
        Some(value) => token_id_or_ids(value),
        None => Vec::new(),
    }
}

pub(crate) fn token_id_or_ids(value: &serde_json::Value) -> Vec<u32> {
    if let Some(id) = value.as_u64().and_then(|id| u32::try_from(id).ok()) {
        return vec![id];
    }
    value
        .as_array()
        .map(|ids| {
            ids.iter()
                .filter_map(|id| id.as_u64().and_then(|id| u32::try_from(id).ok()))
                .collect()
        })
        .unwrap_or_default()
}

impl LocalInferenceBackend for MlxBackend {
    fn id(&self) -> &'static str {
        MLX_BACKEND_ID
    }

    fn load_model(
        &self,
        model_id: &str,
        resolved: &ResolvedModelPaths,
        _settings: &ModelSettings,
    ) -> Result<Box<dyn BackendLoadedModel>, ProviderError> {
        if !resolved.model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {}. Please download it from Settings > Local Inference.",
                model_id
            )));
        }

        let model_dir = model_dir_from_path(&resolved.model_path)?;
        let stream = Stream::new_with_device(&Device::new(DeviceType::Gpu, 0));
        let weights_stream = Stream::new_with_device(&Device::new(DeviceType::Cpu, 0));
        let model =
            LoadedModel::load(&model_dir, &stream, &weights_stream).map_err(mlx_error)?;
        let tokenizer =
            Tokenizer::from_file(model_dir.join("tokenizer.json")).map_err(mlx_error)?;
        tracing::info!(
            backend = self.id(),
            model_id,
            model_type = model.model_type(),
            "MLX model loaded successfully"
        );
        let stop_token_ids = mlx_stop_token_ids(&model, &model_dir);
        Ok(Box::new(MlxLoadedModel {
            model,
            tokenizer,
            model_dir,
            stop_token_ids,
        }))
    }

    fn generate(
        &self,
        loaded: &mut dyn BackendLoadedModel,
        request: LocalGenerationRequest<'_>,
    ) -> Result<(), ProviderError> {
        let loaded = loaded
            .as_any_mut()
            .downcast_mut::<MlxLoadedModel>()
            .ok_or_else(|| {
                ProviderError::ExecutionError("Loaded model backend mismatch".to_string())
            })?;

        let stream = Stream::new_with_device(&Device::new(DeviceType::Gpu, 0));
        let tool_mode = if request.tools.is_empty() {
            ToolMode::None
        } else {
            match request.settings.tool_calling {
                ToolCallingMode::ForceNative => ToolMode::Native,
                ToolCallingMode::Auto | ToolCallingMode::ForceEmulated => ToolMode::Emulated {
                    code_mode_enabled: request
                        .tools
                        .iter()
                        .any(|t| t.name == CODE_EXECUTION_TOOL),
                },
            }
        };
        let prompt = build_prompt(
            &mut loaded.model,
            &request.model_name,
            request.system,
            request.messages,
            request.tools,
            tool_mode,
        )?;
        let prompt_tokens = loaded.model.encode(&prompt, false).map_err(mlx_error)?;
        if prompt_tokens.len() >= request.context_limit && request.context_limit > 0 {
            return Err(ProviderError::ContextLengthExceeded(format!(
                "Prompt ({} tokens) exceeds context limit ({} tokens). Try reducing conversation length.",
                prompt_tokens.len(), request.context_limit
            )));
        }

        let prompt_array = loaded
            .model
            .encode_to_array(&prompt, false, &stream)
            .map_err(mlx_error)?;
        let max_tokens = mlx_max_tokens(
            request.settings,
            request.max_tokens,
            request.context_limit,
            prompt_tokens.len(),
        );
        let (settings_temp, seed) = sampling(request.settings);
        let temp = request.temperature.unwrap_or(settings_temp);
        let prng_key = prng_key(temp, seed)?;
        let eos_token_ids = loaded.stop_token_ids.clone();
        let generation_started = std::time::Instant::now();
        let MlxGeneration {
            generated_ids,
            generated_text,
            draft_stats,
            time_to_first_token_ms,
            streamed_response,
        } = if let Some(draft_model_path) = &request.draft_model_path {
            if matches!(loaded.model.model_mut(), Model::Gemma4(_)) {
                let weights_stream = Stream::new_with_device(&Device::new(DeviceType::Cpu, 0));
                let assistant_model =
                    load_gemma4_assistant_model(draft_model_path, &stream, &weights_stream)
                        .map_err(mlx_error)?;
                let output = generate_gemma4_mtp(
                    loaded.model.model_mut(),
                    &assistant_model,
                    &prompt_array,
                    &eos_token_ids,
                    max_tokens,
                    temp,
                    prng_key,
                    &stream,
                )
                .map_err(mlx_error)?;
                let generated_text = loaded
                    .tokenizer
                    .decode(&output.token_ids, true)
                    .map_err(mlx_error)?;
                MlxGeneration {
                    generated_ids: output.token_ids,
                    generated_text,
                    draft_stats: Some(DraftStats {
                        draft_tokens: output.num_drafted as i32,
                        accepted_tokens: output.num_accepted as i32,
                    }),
                    time_to_first_token_ms: None,
                    streamed_response: false,
                }
            } else {
                let emitter = MlxStreamEmitter::new(
                    request.message_id,
                    tool_mode,
                    request.settings.enable_thinking,
                    loaded.model.generation_prompt().unwrap_or(""),
                    request.tx,
                );
                generate_single_model(
                    &mut loaded.model,
                    &loaded.tokenizer,
                    &prompt_array,
                    &eos_token_ids,
                    max_tokens,
                    temp,
                    prng_key,
                    &stream,
                    generation_started,
                    emitter,
                )?
            }
        } else {
            let emitter = MlxStreamEmitter::new(
                request.message_id,
                tool_mode,
                request.settings.enable_thinking,
                loaded.model.generation_prompt().unwrap_or(""),
                request.tx,
            );
            generate_single_model(
                &mut loaded.model,
                &loaded.tokenizer,
                &prompt_array,
                &eos_token_ids,
                max_tokens,
                temp,
                prng_key,
                &stream,
                generation_started,
                emitter,
            )?
        };

        if !streamed_response {
            emit_generated_response(
                &generated_text,
                loaded.model.generation_prompt().unwrap_or(""),
                request.settings.enable_thinking,
                request.message_id,
                tool_mode,
                request.tx,
            )?;
        }

        let generation_ms =
            u64::try_from(generation_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let tokens_per_second = if generation_ms > 0 {
            Some((generated_ids.len() as f32 / generation_ms as f32) * 1000.0)
        } else {
            None
        };
        let input_tokens = prompt_tokens.len() as i32;
        let output_tokens = generated_ids.len() as i32;
        let mut usage = Usage::new(
            Some(input_tokens),
            Some(output_tokens),
            Some(input_tokens + output_tokens),
        );
        usage.stats = Some(ProviderStats {
            tokens_per_second,
            time_to_first_token_ms,
            generation_ms: Some(generation_ms),
            model_load_ms: request.model_load_ms,
            draft_stats,
        });

        let mut log_json = serde_json::json!({
            "path": "mlx",
            "prompt_tokens": input_tokens,
            "output_tokens": output_tokens,
            "generation_ms": generation_ms,
        });
        if let Some(tps) = tokens_per_second {
            log_json["tokens_per_second"] = serde_json::json!(tps);
        }
        if let Some(ttft) = time_to_first_token_ms {
            log_json["time_to_first_token_ms"] = serde_json::json!(ttft);
        }
        if let Some(load_ms) = request.model_load_ms {
            log_json["model_load_ms"] = serde_json::json!(load_ms);
        }
        let _ = request.log.write(&log_json, Some(&usage));
        let provider_usage = ProviderUsage::new(request.model_name.clone(), usage);
        let _ = request.tx.blocking_send(Ok((None, Some(provider_usage))));
        Ok(())
    }

    fn available_memory_bytes(&self) -> u64 {
        0
    }
}
