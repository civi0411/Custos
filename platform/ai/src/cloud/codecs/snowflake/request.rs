use super::messages::{format_messages, format_system, format_tools};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use anyhow::{anyhow, Result};
use rmcp::model::Tool;
use serde_json::{json, Value};

/// Create a complete request payload for Snowflake's API
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let mut snowflake_messages = format_messages(messages);
    let system_spec = format_system(system);

    // Add system message to the beginning of the messages
    snowflake_messages.insert(0, system_spec);

    // Check if we have any messages to send
    if snowflake_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Snowflake API"));
    }

    // Detect description generation requests and exclude tools to prevent interference
    // with normal tool execution flow
    let is_description_request =
        system.contains("Reply with only a description in four words or less");

    let tool_specs = if is_description_request {
        // For description generation, don't include any tools to avoid confusion
        format_tools(&[])
    } else {
        format_tools(tools)
    };

    let mut payload = json!({
        "model": model_config.model_name,
        "messages": snowflake_messages,
        "max_tokens": model_config.max_output_tokens(),
    });

    // Add tools if present and not a description request
    if !tool_specs.is_empty() {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("tools".to_string(), json!(tool_specs));
        } else {
            return Err(anyhow!(
                "Failed to create request payload: payload is not a JSON object"
            ));
        }
    }

    Ok(payload)
}
