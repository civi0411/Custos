use std::ffi::CStr;

use llama_cpp_2::model::{ChatTemplateResult, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::openai::OpenAIChatTemplateParams;

use super::inference_engine::{LoadedChatTemplates, LoadedModel};
use crate::local::local_inference::model::{ChatTemplate, ModelSettings, ToolCallingMode};
use custos_provider::errors::ProviderError;

pub(crate) fn builtin_chat_template_names() -> Vec<String> {
    let count = unsafe { llama_cpp_sys_2::llama_chat_builtin_templates(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }

    let mut templates = vec![std::ptr::null(); count as usize];
    let written = unsafe {
        llama_cpp_sys_2::llama_chat_builtin_templates(templates.as_mut_ptr(), templates.len())
    };
    templates.truncate(written.max(0) as usize);

    templates
        .into_iter()
        .filter(|ptr| !ptr.is_null())
        .filter_map(|ptr| {
            unsafe { CStr::from_ptr(ptr) }
                .to_str()
                .ok()
                .map(str::to_string)
        })
        .collect()
}

pub(crate) fn template_result_supports_native_tool_calling(result: &ChatTemplateResult) -> bool {
    result.parse_tool_calls
        && result
            .parser
            .as_deref()
            .is_some_and(|parser| !parser.trim().is_empty())
}

pub(crate) fn supports_native_tool_calling(
    loaded: &LoadedModel,
    settings: &ModelSettings,
    template: &LlamaChatTemplate,
    oai_messages_json: &str,
    tools_json: Option<&str>,
) -> bool {
    let Some(tools_json) = tools_json.filter(|tools| !tools.trim().is_empty()) else {
        return false;
    };

    let params = OpenAIChatTemplateParams {
        messages_json: oai_messages_json,
        tools_json: Some(tools_json),
        tool_choice: None,
        json_schema: None,
        grammar: None,
        reasoning_format: if settings.enable_thinking {
            Some("auto")
        } else {
            None
        },
        chat_template_kwargs: None,
        add_generation_prompt: true,
        use_jinja: true,
        parallel_tool_calls: false,
        enable_thinking: settings.enable_thinking,
        add_bos: false,
        add_eos: false,
        parse_tool_calls: true,
    };

    match loaded
        .model
        .apply_chat_template_oaicompat(template, &params)
    {
        Ok(result) => template_result_supports_native_tool_calling(&result),
        Err(e) => {
            tracing::debug!(
                error = %e,
                "llama.cpp chat template dry-run did not support native tool calling"
            );
            false
        }
    }
}

pub(crate) fn should_use_native_tool_calling(
    mode: ToolCallingMode,
    has_tools: bool,
    template_supports_native: bool,
) -> bool {
    has_tools
        && match mode {
            ToolCallingMode::Auto => template_supports_native,
            ToolCallingMode::ForceNative => true,
            ToolCallingMode::ForceEmulated => false,
        }
}

pub(crate) fn is_legacy_builtin_template_name(template: &str) -> bool {
    matches!(
        template.trim(),
        "bailing"
            | "bailing-think"
            | "bailing2"
            | "chatglm3"
            | "chatglm4"
            | "command-r"
            | "deepseek"
            | "deepseek-ocr"
            | "deepseek2"
            | "deepseek3"
            | "exaone-moe"
            | "exaone3"
            | "exaone4"
            | "falcon3"
            | "gemma"
            | "gigachat"
            | "glmedge"
            | "gpt-oss"
            | "granite"
            | "granite-4.0"
            | "grok-2"
            | "hunyuan-dense"
            | "hunyuan-moe"
            | "hunyuan-ocr"
            | "kimi-k2"
            | "llama2"
            | "llama2-sys"
            | "llama2-sys-bos"
            | "llama2-sys-strip"
            | "llama3"
            | "llama4"
            | "megrez"
            | "minicpm"
            | "mistral-v1"
            | "mistral-v3"
            | "mistral-v3-tekken"
            | "mistral-v7"
            | "mistral-v7-tekken"
            | "monarch"
            | "openchat"
            | "orion"
            | "pangu-embedded"
            | "phi3"
            | "phi4"
            | "rwkv-world"
            | "seed_oss"
            | "smolvlm"
            | "solar-open"
            | "vicuna"
            | "vicuna-orca"
            | "yandex"
            | "zephyr"
    )
}

pub(crate) fn missing_chat_template_error(
    model_id: &str,
    architecture: Option<&str>,
    context: &str,
    has_tool_use_template: bool,
) -> ProviderError {
    let architecture = architecture
        .map(str::trim)
        .filter(|arch| !arch.is_empty())
        .map(|arch| format!(" Detected GGUF general.architecture={arch}."))
        .unwrap_or_default();

    let tool_use_note = if has_tool_use_template {
        " A named tool_use chat template is present, but that template is only used for native tool calls with tools present."
    } else {
        ""
    };

    ProviderError::ExecutionError(format!(
        "Model {model_id} does not contain GGUF tokenizer.chat_template metadata required for {context}.{architecture}{tool_use_note} \
         Goose cannot safely infer the correct prompt format from architecture alone. Select a \
         llama.cpp built-in chat template name, configure a custom inline chat template containing \
         the full Jinja template source, or use a GGUF that includes tokenizer.chat_template metadata."
    ))
}

pub(crate) fn load_chat_templates(
    model: &LlamaModel,
    settings: &ModelSettings,
) -> Result<LoadedChatTemplates, ProviderError> {
    match &settings.chat_template {
        ChatTemplate::Embedded => Ok(LoadedChatTemplates {
            default: model.chat_template(None).ok(),
            tool_use: model.chat_template(Some("tool_use")).ok(),
            force_default: false,
        }),
        ChatTemplate::Builtin { name } => {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return Err(ProviderError::ExecutionError(
                    "Built-in chat template name is empty. Enter a llama.cpp built-in template name such as 'chatml', or use embedded chat template metadata.".to_string(),
                ));
            }
            LlamaChatTemplate::new(trimmed)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!(
                        "Built-in chat template name contains an invalid NUL byte: {e}"
                    ))
                })
                .map(|template| LoadedChatTemplates {
                    default: Some(template),
                    tool_use: None,
                    force_default: true,
                })
        }
        ChatTemplate::CustomInline { template } => {
            let trimmed = template.trim();
            if trimmed.is_empty() {
                return Err(ProviderError::ExecutionError(
                    "Custom inline chat template is empty. Paste the full Jinja chat template source, use a llama.cpp built-in template name, or use embedded chat template metadata.".to_string(),
                ));
            }
            if trimmed == "chatml" || is_legacy_builtin_template_name(trimmed) {
                return Err(ProviderError::ExecutionError(format!(
                    "Custom inline chat template is set to '{trimmed}', which is a llama.cpp template name rather than Jinja template source. Paste the full Jinja chat template source instead, or select Built-in and enter '{trimmed}' if that built-in template is intended."
                )));
            }
            LlamaChatTemplate::new(template)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!(
                        "Custom inline chat template contains an invalid NUL byte: {e}"
                    ))
                })
                .map(|template| LoadedChatTemplates {
                    default: Some(template),
                    tool_use: None,
                    force_default: true,
                })
        }
    }
}

pub(crate) fn select_generation_template<'a>(
    model_id: &str,
    model: &LlamaModel,
    templates: &'a LoadedChatTemplates,
    native_tool_calling: bool,
    has_tools: bool,
) -> Result<&'a LlamaChatTemplate, ProviderError> {
    if templates.force_default {
        return templates.default.as_ref().ok_or_else(|| {
            ProviderError::ExecutionError(
                "Configured chat template was not loaded correctly".to_string(),
            )
        });
    }

    if native_tool_calling && has_tools {
        if let Some(template) = templates.tool_use.as_ref() {
            return Ok(template);
        }
    }

    templates.default.as_ref().ok_or_else(|| {
        let architecture = model.meta_val_str("general.architecture").ok();
        let context = if has_tools && native_tool_calling {
            "native tool calling because no tool_use template is available"
        } else if has_tools {
            "emulated tool calling"
        } else {
            "chat without tools"
        };
        missing_chat_template_error(
            model_id,
            architecture.as_deref(),
            context,
            templates.tool_use.is_some(),
        )
    })
}
