use std::any::Any;
use std::path::PathBuf;

use anyhow::Result;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};

use super::hardware::{
    check_cpu_supports_local_inference, is_accelerator_device, log_inference_backend_devices,
};
use super::inference_emulated_tools::{
    build_emulator_tool_description, generate_with_emulated_tools, load_tiny_model_prompt,
};
use super::inference_engine::{GenerationContext, LoadedModel};
use super::inference_native_tools::generate_with_native_tools;
use super::templates::{
    load_chat_templates, select_generation_template, should_use_native_tool_calling,
    supports_native_tool_calling,
};
use crate::local::local_inference::backend::{
    BackendLoadedModel, LocalGenerationRequest, LocalInferenceBackend,
};
use crate::local::local_inference::messages::{
    build_openai_messages_json, build_openai_text_messages_json,
};
use crate::local::local_inference::model::ToolCallingMode;
use crate::local::local_inference::multimodal::ExtractedImage;
use crate::local::local_inference::tool_parsing::compact_tools_json;
use crate::local::local_inference::ResolvedModelPaths;
use custos_provider::errors::ProviderError;
use custos_provider::codecs::openai::format_tools;

pub(crate) const LLAMACPP_BACKEND_ID: &str = "llamacpp";

const CODE_EXECUTION_TOOL: &str = "code_execution__execute_typescript";

pub(crate) struct LlamaCppBackend {
    backend: LlamaBackend,
}

impl LlamaCppBackend {
    pub(crate) fn new() -> Result<Self> {
        check_cpu_supports_local_inference()?;

        let backend = match LlamaBackend::init() {
            Ok(backend) => backend,
            Err(e) => {
                tracing::error!(error = %e, "failed to initialize local inference runtime");
                return Err(anyhow::anyhow!("Failed to init llama backend: {}", e));
            }
        };

        log_inference_backend_devices();

        Ok(Self { backend })
    }

    pub(crate) fn llama_backend(&self) -> &LlamaBackend {
        &self.backend
    }

    fn init_mtmd_context(
        model: &LlamaModel,
        mmproj_path: &Option<PathBuf>,
        settings: &crate::model::ModelSettings,
    ) -> Option<llama_cpp_2::mtmd::MtmdContext> {
        use llama_cpp_2::mtmd::{MtmdContext, MtmdContextParams};

        let mmproj_path = mmproj_path.as_ref().filter(|p| p.exists())?;

        let params = MtmdContextParams {
            use_gpu: true,
            n_threads: settings
                .n_threads
                .unwrap_or_else(|| MtmdContextParams::default().n_threads),
            ..MtmdContextParams::default()
        };

        match MtmdContext::init_from_file(mmproj_path.to_str().unwrap_or_default(), model, &params)
        {
            Ok(ctx) => {
                tracing::info!(
                    vision = ctx.support_vision(),
                    audio = ctx.support_audio(),
                    "Multimodal context initialized"
                );
                Some(ctx)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to init multimodal context");
                None
            }
        }
    }
}

impl LocalInferenceBackend for LlamaCppBackend {
    fn id(&self) -> &'static str {
        LLAMACPP_BACKEND_ID
    }

    fn load_model(
        &self,
        model_id: &str,
        resolved: &ResolvedModelPaths,
        settings: &crate::model::ModelSettings,
    ) -> Result<Box<dyn BackendLoadedModel>, ProviderError> {
        let model_path = &resolved.model_path;

        if !model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {}. Please download it from Settings > Local Inference.",
                model_id
            )));
        }

        tracing::info!(
            backend = self.id(),
            "Loading {} from: {}",
            model_id,
            model_path.display()
        );

        let mut params = LlamaModelParams::default();
        if let Some(n_gpu_layers) = settings.n_gpu_layers {
            params = params.with_n_gpu_layers(n_gpu_layers);
        }
        if settings.use_mlock {
            params = params.with_use_mlock(true);
        }
        params = params.with_use_mmap(settings.use_mmap);
        let model = LlamaModel::load_from_file(&self.backend, model_path, &params)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        let templates = load_chat_templates(&model, settings)?;

        let mtmd_ctx = Self::init_mtmd_context(&model, &resolved.mmproj_path, settings);

        tracing::info!(
            backend = self.id(),
            model_id = model_id,
            "Model loaded successfully"
        );

        Ok(Box::new(LoadedModel {
            model,
            templates,
            mtmd_ctx,
        }))
    }

    fn generate(
        &self,
        loaded: &mut dyn BackendLoadedModel,
        request: LocalGenerationRequest<'_>,
    ) -> Result<(), ProviderError> {
        let loaded = loaded
            .as_any_mut()
            .downcast_mut::<LoadedModel>()
            .ok_or_else(|| {
                ProviderError::ExecutionError("Loaded model backend mismatch".to_string())
            })?;

        let has_vision = request.resolved_model.mmproj_path.is_some();
        let marker = llama_cpp_2::mtmd::mtmd_default_marker();
        let (images, vision_messages): (Vec<ExtractedImage>, Option<Vec<_>>) = if has_vision {
            let (imgs, msgs) =
                crate::local::local_inference::multimodal::extract_images_from_messages(
                    request.messages,
                    marker,
                );
            (imgs, Some(msgs))
        } else {
            (Vec::new(), None)
        };
        let has_media = !images.is_empty();
        let effective_messages = vision_messages.as_deref().unwrap_or(request.messages);

        let code_mode_enabled = request.tools.iter().any(|t| t.name == CODE_EXECUTION_TOOL);
        let (full_tools_json, compact_tools) = if !request.tools.is_empty() {
            let full = format_tools(request.tools)
                .ok()
                .and_then(|spec| serde_json::to_string(&spec).ok());
            let compact = compact_tools_json(request.tools);
            (full, compact)
        } else {
            (None, None)
        };

        let has_native_tool_payload = full_tools_json
            .as_deref()
            .is_some_and(|tools| !tools.trim().is_empty());

        let template_supports_native =
            if matches!(request.settings.tool_calling, ToolCallingMode::Auto)
                && has_native_tool_payload
            {
                let messages_json = build_openai_messages_json(
                    request.system,
                    effective_messages,
                    has_media.then_some(marker),
                );
                if let Some(template) = loaded.templates.tool_use.as_ref() {
                    supports_native_tool_calling(
                        loaded,
                        request.settings,
                        template,
                        &messages_json,
                        full_tools_json.as_deref(),
                    )
                } else {
                    loaded.templates.default.as_ref().is_some_and(|template| {
                        supports_native_tool_calling(
                            loaded,
                            request.settings,
                            template,
                            &messages_json,
                            full_tools_json.as_deref(),
                        )
                    })
                }
            } else {
                false
            };
        let native_tool_calling = should_use_native_tool_calling(
            request.settings.tool_calling,
            !request.tools.is_empty(),
            template_supports_native,
        );
        let use_emulator = !native_tool_calling && !request.tools.is_empty();
        let system_prompt = if use_emulator {
            let tool_desc = build_emulator_tool_description(request.tools, code_mode_enabled);
            format!("{}{}", load_tiny_model_prompt(), tool_desc)
        } else {
            request.system.to_string()
        };

        let oai_messages_json = if use_emulator {
            build_openai_text_messages_json(
                &system_prompt,
                effective_messages,
                has_media.then_some(marker),
            )
        } else {
            build_openai_messages_json(
                &system_prompt,
                effective_messages,
                has_media.then_some(marker),
            )
        };

        if !images.is_empty() && loaded.mtmd_ctx.is_none() {
            loaded.mtmd_ctx = Self::init_mtmd_context(
                &loaded.model,
                &request.resolved_model.mmproj_path,
                request.settings,
            );
        }

        let template = select_generation_template(
            &request.model_name,
            &loaded.model,
            &loaded.templates,
            native_tool_calling,
            !request.tools.is_empty(),
        )?;

        let mut gen_ctx = GenerationContext {
            loaded,
            backend: self,
            template,
            settings: request.settings,
            context_limit: request.context_limit,
            model_name: request.model_name,
            message_id: request.message_id,
            tx: request.tx,
            log: request.log,
            images: &images,
        };

        if use_emulator {
            generate_with_emulated_tools(&mut gen_ctx, code_mode_enabled, &oai_messages_json)
        } else {
            generate_with_native_tools(
                &mut gen_ctx,
                &oai_messages_json,
                full_tools_json.as_deref(),
                compact_tools.as_deref(),
            )
        }
    }

    fn available_memory_bytes(&self) -> u64 {
        let devices = list_llama_ggml_backend_devices();

        let accel_memory = devices
            .iter()
            .filter(|d| is_accelerator_device(d.device_type))
            .map(|d| d.memory_free as u64)
            .max()
            .unwrap_or(0);

        if accel_memory > 0 {
            accel_memory
        } else {
            devices
                .iter()
                .filter(|d| d.device_type == LlamaBackendDeviceType::Cpu)
                .map(|d| d.memory_free as u64)
                .max()
                .unwrap_or(0)
        }
    }
}

impl BackendLoadedModel for LoadedModel {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
