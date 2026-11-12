use crate::conversation::message::Message;
use anyhow::Result;
use rmcp::model::{object, CallToolRequestParams};
use rmcp::object;
use serde_json::Value;

/// Convert Snowflake's streaming API response to internal Message format
pub fn parse_streaming_response(sse_data: &str) -> Result<Message> {
    let mut message = Message::assistant();
    let mut accumulated_text = String::new();
    let mut tool_use_id: Option<String> = None;
    let mut tool_name: Option<String> = None;
    let mut tool_input = String::new();

    // Parse each SSE event
    for line in sse_data.lines() {
        // SSE spec allows both "data: value" and "data:value" (space after colon is optional)
        if !line.starts_with("data:") {
            continue;
        }

        let json_str = line
            .strip_prefix("data: ")
            .or_else(|| line.strip_prefix("data:"))
            .unwrap(); // Remove "data:" prefix
        if json_str.trim().is_empty() || json_str.trim() == "[DONE]" {
            continue;
        }

        let event: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => {
                continue;
            }
        };

        if let Some(choices) = event.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    match delta.get("type").and_then(|t| t.as_str()) {
                        Some("text") => {
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                accumulated_text.push_str(content);
                            }
                        }
                        Some("tool_use") => {
                            if let Some(id) = delta.get("tool_use_id").and_then(|i| i.as_str()) {
                                tool_use_id = Some(id.to_string());
                            }
                            if let Some(name) = delta.get("name").and_then(|n| n.as_str()) {
                                tool_name = Some(name.to_string());
                            }
                            if let Some(input) = delta.get("input").and_then(|i| i.as_str()) {
                                tool_input.push_str(input);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Add accumulated text if any
    if !accumulated_text.is_empty() {
        message = message.with_text(accumulated_text);
    }

    // Add tool use if complete
    if let Some((id, name)) = tool_use_id.zip(tool_name) {
        if !tool_input.is_empty() {
            let input_value = serde_json::from_str::<Value>(&tool_input)
                .unwrap_or_else(|_| Value::String(tool_input.clone()));
            let tool_call = CallToolRequestParams::new(name).with_arguments(object(input_value));
            message = message.with_tool_request(&id, Ok(tool_call));
        } else {
            // Tool with no input - use empty object
            let tool_call = CallToolRequestParams::new(name).with_arguments(object!({}));
            message = message.with_tool_request(&id, Ok(tool_call));
        }
    }

    Ok(message)
}
