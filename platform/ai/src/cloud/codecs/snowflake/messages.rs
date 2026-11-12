use crate::conversation::message::{Message, MessageContentBlock};
use crate::documents::{unsupported_document_text, UNSUPPORTED_PROVIDER_REASON};
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::{Role, Tool};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Convert internal Message format to Snowflake's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    let mut snowflake_messages = Vec::new();

    for message in messages {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut text_content = String::new();

        for msg_content in &message.content {
            match msg_content {
                MessageContentBlock::Text(text) => {
                    if !text_content.is_empty() {
                        text_content.push('\n');
                    }
                    text_content.push_str(&text.text);
                }
                MessageContentBlock::ToolRequest(_tool_request) => {
                    // Skip tool requests in message formatting - tools are handled separately
                    // through the tools parameter in the API request
                    continue;
                }
                MessageContentBlock::ToolResponse(tool_response) => {
                    if let Ok(result) = &tool_response.tool_result {
                        let text = result
                            .content
                            .iter()
                            .filter_map(|c| {
                                if let Some(t) = c.as_text() {
                                    return Some(t.text.clone());
                                }
                                if let Some(r) = c.as_resource() {
                                    let text = extract_text_from_resource(&r.resource);
                                    if !text.is_empty() {
                                        return Some(text);
                                    }
                                }
                                None
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        if !text_content.is_empty() {
                            text_content.push('\n');
                        }
                        if !text.is_empty() {
                            text_content.push_str(&format!("Tool result: {}", text));
                        }
                    }
                }
                MessageContentBlock::ToolConfirmationRequest(_) => {}
                MessageContentBlock::ActionRequired(_) => {}
                MessageContentBlock::SystemNotification(_) | MessageContentBlock::Error(_) => {
                    // Skip
                }
                MessageContentBlock::Thinking(_thinking) => {
                    // Skip thinking for now
                }
                MessageContentBlock::RedactedThinking(_redacted) => {
                    // Skip redacted thinking for now
                }
                MessageContentBlock::Image(_) => continue, // Snowflake doesn't support image content yet
                MessageContentBlock::Document(document) => {
                    if !text_content.is_empty() {
                        text_content.push('\n');
                    }
                    text_content.push_str(&unsupported_document_text(
                        document,
                        UNSUPPORTED_PROVIDER_REASON,
                    ));
                }
            }
        }

        // Add message if it has text content
        if !text_content.is_empty() {
            snowflake_messages.push(json!({
                "role": role,
                "content": text_content
            }));
        }
    }

    // Only add default message if we truly have no messages at all
    // This should be rare and only for edge cases
    if snowflake_messages.is_empty() {
        snowflake_messages.push(json!({
            "role": "user",
            "content": "Continue the conversation"
        }));
    }

    snowflake_messages
}

/// Convert internal Tool format to Snowflake's API tool specification
pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    let mut unique_tools = HashSet::new();
    let mut tool_specs = Vec::new();

    for tool in tools.iter() {
        if unique_tools.insert(tool.name.clone()) {
            let tool_spec = json!({
                "type": "generic",
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema
            });

            tool_specs.push(json!({"tool_spec": tool_spec}));
        }
    }

    tool_specs
}

/// Convert system message to Snowflake's API system specification
pub fn format_system(system: &str) -> Value {
    json!({
        "role": "system",
        "content": system,
    })
}
