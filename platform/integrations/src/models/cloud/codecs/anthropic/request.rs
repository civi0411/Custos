use crate::model::ModelConfig;
use anyhow::{anyhow, Result};
use rmcp::model::Tool;
use serde_json::{json, Value};
use crate::conversation::message::Message;
use super::messages::format_messages_with_options;
use super::options::*;
use super::tools::{format_system, format_tools};

pub fn create_request(
    provider_name: &str,
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    options: AnthropicFormatOptions,
) -> Result<Value> {
    create_request_for_model(
        provider_name,
        model_config,
        &model_config.model_name,
        system,
        messages,
        tools,
        options,
    )
}

pub fn create_request_for_model(
    provider_name: &str,
    model_config: &ModelConfig,
    wire_model_name: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    options: AnthropicFormatOptions,
) -> Result<Value> {
    let options = options.for_model(model_config);
    let anthropic_messages = format_messages_with_options(messages, &options);
    let tool_specs = format_tools(tools, &options);
    let system_spec = format_system(system, &options);

    if anthropic_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Anthropic API"));
    }

    let max_tokens = model_config.max_output_tokens();
    let mut payload = json!({
        "model": wire_model_name,
        "messages": anthropic_messages,
        "max_tokens": max_tokens,
    });

    if !system.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("system".to_string(), json!(system_spec));
    }

    if !tool_specs.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tool_specs));
    }

    if model_supports_temperature(provider_name, model_config) {
        if let Some(temp) = model_config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }
    }

    apply_thinking_config(
        &mut payload,
        provider_name,
        model_config,
        max_tokens,
        options,
    );

    Ok(payload)
}
