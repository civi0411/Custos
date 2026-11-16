use custos_provider::errors::ProviderError;
use custos_provider::request_log::{LoggerHandleExt, RequestLogHandle};
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::{AddBos, ChatTemplateResult, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::mtmd::{MtmdBitmap, MtmdContext, MtmdInputText};
use llama_cpp_2::openai::OpenAIChatTemplateParams;

use super::context::{
    build_context_params, context_cap, effective_context_size, estimate_max_context_for_memory,
    validate_and_compute_context,
};
use super::sampler::build_sampler;
use crate::local::local_inference::llamacpp::LlamaCppBackend;
use crate::local::local_inference::model::ModelSettings;
use crate::local::local_inference::multimodal::ExtractedImage;
use crate::local::local_inference::StreamSender;

pub(crate) struct GenerationContext<'a> {
    pub loaded: &'a LoadedModel,
    pub backend: &'a LlamaCppBackend,
    pub template: &'a LlamaChatTemplate,
    pub settings: &'a ModelSettings,
    pub context_limit: usize,
    pub model_name: String,
    pub message_id: &'a str,
    pub tx: &'a StreamSender,
    pub log: &'a mut Option<Box<dyn RequestLogHandle>>,
    pub images: &'a [ExtractedImage],
}

pub(crate) struct LoadedModel {
    pub model: LlamaModel,
    pub templates: LoadedChatTemplates,
    /// Multimodal context for vision models. None for text-only models.
    pub mtmd_ctx: Option<MtmdContext>,
}

pub(crate) struct LoadedChatTemplates {
    pub default: Option<LlamaChatTemplate>,
    pub tool_use: Option<LlamaChatTemplate>,
    pub force_default: bool,
}

pub(crate) struct PreparedGeneration<'model> {
    pub template_result: ChatTemplateResult,
    pub llama_ctx: llama_cpp_2::context::LlamaContext<'model>,
    pub prompt_token_count: usize,
    pub effective_ctx: usize,
}

/// Create a llama context and prefill (decode) all prompt tokens.
pub(crate) fn create_and_prefill_context<'model>(
    loaded: &'model LoadedModel,
    backend: &LlamaCppBackend,
    tokens: &[llama_cpp_2::token::LlamaToken],
    effective_ctx: usize,
    settings: &crate::model::ModelSettings,
) -> Result<llama_cpp_2::context::LlamaContext<'model>, ProviderError> {
    let ctx_params = build_context_params(effective_ctx as u32, settings);
    let mut ctx = loaded
        .model
        .new_context(backend.llama_backend(), ctx_params)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to create context: {}", e)))?;

    let n_batch = ctx.n_batch() as usize;
    for chunk in tokens.chunks(n_batch) {
        let mut batch = LlamaBatch::get_one(chunk)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to create batch: {}", e)))?;
        ctx.decode(&mut batch)
            .map_err(|e| ProviderError::ExecutionError(format!("Prefill decode failed: {}", e)))?;
    }

    Ok(ctx)
}

/// Tokenize text + images via mtmd and prefill the context.
///
/// Returns the llama context, the number of prompt tokens consumed,
/// and the effective context size.
pub(crate) fn create_and_prefill_multimodal<'model>(
    loaded: &'model LoadedModel,
    backend: &LlamaCppBackend,
    prompt_text: &str,
    images: &[ExtractedImage],
    context_limit: usize,
    settings: &ModelSettings,
) -> Result<(llama_cpp_2::context::LlamaContext<'model>, usize, usize), ProviderError> {
    let mtmd_ctx = loaded.mtmd_ctx.as_ref().ok_or_else(|| {
        ProviderError::ExecutionError(
            "This model does not have vision support. Download the vision encoder from \
             Settings > Local Inference, or use a text-only message."
                .to_string(),
        )
    })?;

    let bitmaps: Vec<MtmdBitmap> = images
        .iter()
        .map(|img| {
            MtmdBitmap::from_buffer(mtmd_ctx, &img.bytes)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode image: {e}")))
        })
        .collect::<Result<_, _>>()?;

    let bitmap_refs: Vec<&MtmdBitmap> = bitmaps.iter().collect();

    let input_text = MtmdInputText {
        text: prompt_text.to_string(),
        add_special: true,
        parse_special: true,
    };
    let chunks = mtmd_ctx.tokenize(input_text, &bitmap_refs).map_err(|e| {
        ProviderError::ExecutionError(format!("Multimodal tokenization failed: {e}"))
    })?;

    let prompt_token_count = chunks.total_tokens();

    let n_ctx_train = loaded.model.n_ctx_train() as usize;
    let mmproj_overhead = settings.mmproj_size_bytes;
    let memory_max_ctx = estimate_max_context_for_memory(&loaded.model, backend, mmproj_overhead);
    let effective_ctx = effective_context_size(
        prompt_token_count,
        settings,
        context_limit,
        n_ctx_train,
        memory_max_ctx,
    );

    let min_generation_headroom = 512;
    if prompt_token_count + min_generation_headroom > effective_ctx {
        return Err(ProviderError::ContextLengthExceeded(format!(
            "Multimodal prompt ({prompt_token_count} tokens including images) exceeds \
             context limit ({effective_ctx} tokens)",
        )));
    }

    let ctx_params = build_context_params(effective_ctx as u32, settings);
    let llama_ctx = loaded
        .model
        .new_context(backend.llama_backend(), ctx_params)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to create context: {e}")))?;

    let n_batch = llama_ctx.n_batch() as i32;
    let _n_past = chunks
        .eval_chunks(mtmd_ctx, &llama_ctx, 0, 0, n_batch, true)
        .map_err(|e| ProviderError::ExecutionError(format!("Multimodal eval failed: {e}")))?;

    Ok((llama_ctx, prompt_token_count, effective_ctx))
}

pub(crate) fn prepare_generation<'model>(
    ctx: &mut GenerationContext<'model>,
    oai_messages_json: &str,
    full_tools_json: Option<&str>,
    compact_tools_json: Option<&str>,
) -> Result<PreparedGeneration<'model>, ProviderError> {
    let apply_template = |tools: Option<&str>| {
        let params = OpenAIChatTemplateParams {
            messages_json: oai_messages_json,
            tools_json: tools,
            tool_choice: None,
            json_schema: None,
            grammar: None,
            reasoning_format: if ctx.settings.enable_thinking {
                Some("auto")
            } else {
                None
            },
            chat_template_kwargs: None,
            add_generation_prompt: true,
            use_jinja: true,
            parallel_tool_calls: false,
            enable_thinking: ctx.settings.enable_thinking,
            add_bos: false,
            add_eos: false,
            parse_tool_calls: true,
        };
        ctx.loaded
            .model
            .apply_chat_template_oaicompat(ctx.template, &params)
    };

    let min_generation_headroom = 512;
    let n_ctx_train = ctx.loaded.model.n_ctx_train() as usize;
    let mmproj_overhead = if ctx.loaded.mtmd_ctx.is_some() {
        ctx.settings.mmproj_size_bytes
    } else {
        0
    };
    let memory_max_ctx =
        estimate_max_context_for_memory(&ctx.loaded.model, ctx.backend, mmproj_overhead);
    let cap = context_cap(ctx.settings, ctx.context_limit, n_ctx_train, memory_max_ctx);
    let token_budget = cap.saturating_sub(min_generation_headroom);
    let estimated_image_tokens = ctx.images.len() * ctx.settings.image_token_estimate;

    let template_result = match apply_template(full_tools_json) {
        Ok(r) => {
            let token_count = ctx
                .loaded
                .model
                .str_to_token(&r.prompt, AddBos::Never)
                .map(|t| t.len())
                .unwrap_or(0);
            if token_count + estimated_image_tokens > token_budget {
                apply_template(compact_tools_json).unwrap_or(r)
            } else {
                r
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to apply llama.cpp OpenAI-compatible chat template"
            );
            match apply_template(compact_tools_json) {
                Ok(r) => r,
                Err(compact_err) => {
                    return Err(ProviderError::ExecutionError(format!(
                        "Failed to apply chat template with llama.cpp's Jinja renderer. This usually means the selected built-in template name does not exist, the embedded or custom template is invalid, or the template is incompatible with the current message shape. Select a valid llama.cpp built-in template name, configure a custom inline Jinja template, or use a GGUF with valid tokenizer.chat_template metadata. Full tools error: {e}; compact tools error: {compact_err}"
                    )));
                }
            }
        }
    };

    let _ = ctx.log.write(
        &serde_json::json!({"applied_prompt": &template_result.prompt}),
        None,
    );

    let (llama_ctx, prompt_token_count, effective_ctx) = if !ctx.images.is_empty() {
        create_and_prefill_multimodal(
            ctx.loaded,
            ctx.backend,
            &template_result.prompt,
            ctx.images,
            ctx.context_limit,
            ctx.settings,
        )?
    } else {
        let tokens = ctx
            .loaded
            .model
            .str_to_token(&template_result.prompt, AddBos::Never)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        let (ptc, ectx) = validate_and_compute_context(
            ctx.loaded,
            ctx.backend,
            tokens.len(),
            ctx.context_limit,
            ctx.settings,
        )?;
        let lctx =
            create_and_prefill_context(ctx.loaded, ctx.backend, &tokens, ectx, ctx.settings)?;
        (lctx, ptc, ectx)
    };

    Ok(PreparedGeneration {
        template_result,
        llama_ctx,
        prompt_token_count,
        effective_ctx,
    })
}

/// Action to take after processing a generated token piece.
pub(crate) enum TokenAction {
    Continue,
    Stop,
}

/// Run the autoregressive generation loop. Calls `on_piece` for each non-empty
/// token piece. The callback returns `TokenAction::Stop` to break early.
/// Returns the total number of generated tokens, or `ContextLengthExceeded`
/// if the model exhausted the available context window.
pub(crate) fn generation_loop(
    model: &LlamaModel,
    ctx: &mut llama_cpp_2::context::LlamaContext<'_>,
    settings: &crate::model::ModelSettings,
    prompt_token_count: usize,
    effective_ctx: usize,
    mut on_piece: impl FnMut(&str) -> Result<TokenAction, ProviderError>,
) -> Result<i32, ProviderError> {
    let mut sampler = build_sampler(settings);
    let context_headroom = effective_ctx.saturating_sub(prompt_token_count);
    let max_output = if let Some(max) = settings.max_output_tokens {
        context_headroom.min(max)
    } else {
        context_headroom
    };
    let hit_context_limit = settings
        .max_output_tokens
        .is_none_or(|max| context_headroom <= max);
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut output_token_count: i32 = 0;
    let mut exhausted_loop = true;

    for _ in 0..max_output {
        let token = sampler.sample(ctx, -1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            exhausted_loop = false;
            break;
        }

        output_token_count += 1;

        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

        if !piece.is_empty() && matches!(on_piece(&piece)?, TokenAction::Stop) {
            exhausted_loop = false;
            break;
        }

        let next_tokens = [token];
        let mut next_batch = LlamaBatch::get_one(&next_tokens)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to create batch: {}", e)))?;
        ctx.decode(&mut next_batch)
            .map_err(|e| ProviderError::ExecutionError(format!("Decode failed: {}", e)))?;
    }

    if exhausted_loop && hit_context_limit {
        return Err(ProviderError::ContextLengthExceeded(format!(
            "Generation exhausted context window ({} prompt + {} generated = {} of {} limit)",
            prompt_token_count,
            output_token_count,
            prompt_token_count as i32 + output_token_count,
            effective_ctx,
        )));
    }

    Ok(output_token_count)
}
