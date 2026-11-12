use crate::conversation::message::Message;
use crate::images::ImageFormat;
use crate::model::{is_goose_internal_request_param, ModelConfig};
use anyhow::{anyhow, Error};
use rmcp::model::Tool;
use serde_json::{json, Value};

use super::messages::format_messages_with_options;
use super::options::{
    extract_reasoning_effort, is_openai_responses_model, is_reserved_request_param_key,
    openai_reasoning_effort_for_thinking, supports_xai_reasoning_effort,
    xai_reasoning_effort_for_thinking, OpenAiFormatOptions,
};
use super::tools::{format_tools, validate_tool_schemas};

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    for_streaming: bool,
) -> anyhow::Result<Value, Error> {
    create_request_with_options(
        model_config,
        system,
        messages,
        tools,
        image_format,
        for_streaming,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: model_config.supports_vision.unwrap_or_default(),
            ..Default::default()
        },
    )
}

pub fn create_request_with_options(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    for_streaming: bool,
    format_options: OpenAiFormatOptions,
) -> anyhow::Result<Value, Error> {
    let (wire_model_name, _) = extract_reasoning_effort(&model_config.model_name);
    create_request_for_model_with_options(
        model_config,
        &wire_model_name,
        &model_config.model_name,
        system,
        messages,
        tools,
        image_format,
        for_streaming,
        format_options,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_request_for_model_with_options(
    model_config: &ModelConfig,
    wire_model_name: &str,
    capability_model_name: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    for_streaming: bool,
    format_options: OpenAiFormatOptions,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let (model_name, legacy_reasoning_effort) = extract_reasoning_effort(capability_model_name);
    let is_reasoning_model = is_openai_responses_model(&model_name);
    let supports_xai_effort = supports_xai_reasoning_effort(&model_name);
    let reasoning_effort = if is_reasoning_model {
        model_config
            .thinking_effort()
            .map_or(legacy_reasoning_effort, |effort| {
                openai_reasoning_effort_for_thinking(&model_name, effort)
            })
    } else if supports_xai_effort {
        model_config
            .thinking_effort()
            .and_then(|effort| xai_reasoning_effort_for_thinking(&model_name, effort))
    } else {
        None
    };

    let system_message = json!({
        "role": if is_reasoning_model { "developer" } else { "system" },
        "content": system
    });

    let messages_spec = format_messages_with_options(messages, image_format, format_options);
    let mut tools_spec = format_tools(tools)?;

    validate_tool_schemas(&mut tools_spec);

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": wire_model_name,
        "messages": messages_array
    });

    if let Some(effort) = reasoning_effort {
        payload["reasoning_effort"] = json!(effort);
    }

    if !tools_spec.is_empty() {
        payload["tools"] = json!(tools_spec);
    }

    if !is_reasoning_model && !supports_xai_effort {
        if let Some(temp) = model_config.temperature {
            payload["temperature"] = json!(temp);
        }
    }

    // Only emit max_tokens / max_completion_tokens when the user (via
    // GOOSE_MAX_TOKENS) or a canonical model record has supplied a value.
    // For unknown models on OpenAI-compatible endpoints (e.g. llama_swap,
    // lmstudio) sending the historic 4096 default truncates non-trivial
    // responses; omitting the field lets the server use its own max.
    if let Some(max_tokens) = model_config.max_tokens {
        let key = if is_reasoning_model {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        payload
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), json!(max_tokens));
    }

    if for_streaming {
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({"include_usage": true});
    }

    if let Some(params) = &model_config.request_params {
        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in params {
                if !is_goose_internal_request_param(key) && !is_reserved_request_param_key(key) {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    Ok(payload)
}
