use super::messages::{format_messages, DatabricksMessage};
use super::tools::format_tools;
use crate::cache_semantics::{apply_chat_payload_breakpoints, CacheSemantics};
use crate::conversation::message::Message;
use crate::cloud::codecs::anthropic::{
    adaptive_output_effort, model_supports_temperature, requires_explicit_thinking_disable,
    thinking_budget_tokens, thinking_type_for_provider, ThinkingType,
};
use crate::cloud::codecs::openai::{
    extract_reasoning_effort, is_openai_responses_model, openai_reasoning_effort_for_thinking,
    validate_tool_schemas,
};
use crate::images::ImageFormat;
use crate::model::{is_goose_internal_request_param, ModelConfig};
use anyhow::{anyhow, Error};
use rmcp::model::Tool;
use serde_json::{json, Value};

pub const DATABRICKS_PROVIDER_NAME: &str = "databricks";

fn apply_claude_thinking_config(
    payload: &mut Value,
    provider_name: &str,
    model_config: &ModelConfig,
) {
    let obj = payload.as_object_mut().unwrap();

    match thinking_type_for_provider(provider_name, model_config) {
        ThinkingType::Adaptive => {
            obj.insert("thinking".to_string(), json!({ "type": "adaptive" }));
            obj.insert(
                "output_config".to_string(),
                json!({ "effort": adaptive_output_effort(model_config).to_string() }),
            );
            obj.insert(
                "max_completion_tokens".to_string(),
                json!(model_config.max_output_tokens()),
            );
        }
        ThinkingType::Enabled => {
            let budget_tokens = thinking_budget_tokens(model_config);
            let max_tokens = model_config.max_output_tokens() + budget_tokens;
            obj.insert("max_tokens".to_string(), json!(max_tokens));
            obj.insert(
                "thinking".to_string(),
                json!({
                    "type": "enabled",
                    "budget_tokens": budget_tokens
                }),
            );
            obj.insert("temperature".to_string(), json!(2));
        }
        ThinkingType::Disabled => {
            if requires_explicit_thinking_disable(provider_name, &model_config.model_name) {
                obj.insert("thinking".to_string(), json!({ "type": "disabled" }));
            }
            if model_supports_temperature(provider_name, model_config) {
                if let Some(temp) = model_config.temperature {
                    obj.insert("temperature".to_string(), json!(temp));
                }
            }
            obj.insert(
                "max_completion_tokens".to_string(),
                json!(model_config.max_output_tokens()),
            );
        }
    }
}

/// Check if the model name indicates a Claude/Anthropic model that supports cache control.
pub(crate) fn is_claude_model(model_name: &str) -> bool {
    model_name.contains("claude")
}

#[allow(clippy::too_many_lines)]
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
) -> anyhow::Result<Value, Error> {
    create_request_for_provider(
        DATABRICKS_PROVIDER_NAME,
        model_config,
        system,
        messages,
        tools,
        image_format,
    )
}

pub fn create_request_for_provider(
    provider_name: &str,
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let (model_name, legacy_reasoning_effort) = extract_reasoning_effort(&model_config.model_name);
    let is_openai_reasoning_model = is_openai_responses_model(&model_name);
    let reasoning_effort = if is_openai_reasoning_model {
        model_config
            .thinking_effort()
            .map_or(legacy_reasoning_effort, |effort| {
                openai_reasoning_effort_for_thinking(&model_name, effort)
            })
    } else {
        None
    };

    let system_message = DatabricksMessage {
        role: "system".to_string(),
        content: system.into(),
        tool_calls: None,
        tool_call_id: None,
    };

    let model_supports_vision = model_config.supports_vision.unwrap_or_default();
    let messages_spec = format_messages(
        messages,
        image_format,
        Some(&model_config.model_name),
        model_supports_vision,
    );
    let mut tools_spec = if !tools.is_empty() {
        format_tools(tools, &model_config.model_name)?
    } else {
        vec![]
    };

    // Validate tool schemas
    validate_tool_schemas(&mut tools_spec);

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_name,
        "messages": messages_array
    });

    if let Some(effort) = reasoning_effort {
        payload
            .as_object_mut()
            .unwrap()
            .insert("reasoning_effort".to_string(), json!(effort));
    }

    if !tools_spec.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if is_claude_model(&model_config.model_name) {
        apply_claude_thinking_config(&mut payload, provider_name, model_config);
    } else {
        // open ai reasoning models currently don't support temperature
        if !is_openai_reasoning_model && model_supports_temperature(provider_name, model_config) {
            if let Some(temp) = model_config.temperature {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("temperature".to_string(), json!(temp));
            }
        }

        payload.as_object_mut().unwrap().insert(
            "max_completion_tokens".to_string(),
            json!(model_config.max_output_tokens()),
        );
    }

    if CacheSemantics::for_model("databricks", &model_config.model_name).uses_explicit_breakpoints()
        && !model_config.prompt_cache_disabled()
    {
        apply_chat_payload_breakpoints(&mut payload);
    }

    // Add request_params to the payload (e.g., anthropic_beta for extended context)
    if let Some(params) = &model_config.request_params {
        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in params {
                if is_goose_internal_request_param(key) {
                    continue;
                }
                obj.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(payload)
}
