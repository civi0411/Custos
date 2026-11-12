use crate::conversation::message::Message;
use crate::conversation::token_usage::Usage;
use crate::errors::ProviderError;
use anyhow::{anyhow, Result};
use rmcp::model::{object, CallToolRequestParams};
use serde_json::Value;

/// Convert Snowflake's API response to internal Message format
pub fn response_to_message(response: &Value) -> Result<Message> {
    let mut message = Message::assistant();

    let content_list = response.get("content_list").and_then(|cl| cl.as_array());

    // Handle case where content_list is missing or empty
    let content_list = match content_list {
        Some(list) if !list.is_empty() => list,
        _ => {
            // If no content_list or empty, check if there's a direct content field
            if let Some(direct_content) = response.get("content").and_then(|c| c.as_str()) {
                if !direct_content.is_empty() {
                    message = message.with_text(direct_content.to_string());
                }
                return Ok(message);
            } else {
                // Return empty assistant message for empty responses
                return Ok(message);
            }
        }
    };

    // Process all content items in the list
    for content in content_list {
        match content.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        message = message.with_text(text.to_string());
                    }
                }
            }
            Some("tool_use") => {
                let id = content
                    .get("tool_use_id")
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use id"))?;
                let name = content
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use name"))?
                    .to_string();

                let input = content
                    .get("input")
                    .ok_or_else(|| anyhow!("Missing tool input"))?
                    .clone();

                let tool_call = CallToolRequestParams::new(name).with_arguments(object(input));
                message = message.with_tool_request(id, Ok(tool_call));
            }
            Some("thinking") => {
                let thinking = content
                    .get("thinking")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow!("Missing thinking content"))?;
                let signature = content
                    .get("signature")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow!("Missing thinking signature"))?;
                message = message.with_thinking(thinking, signature);
            }
            Some("redacted_thinking") => {
                let data = content
                    .get("data")
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| anyhow!("Missing redacted_thinking data"))?;
                message = message.with_redacted_thinking(data);
            }
            _ => {
                // Ignore unrecognized content types
            }
        }
    }

    Ok(message)
}

/// Extract usage information from Snowflake's API response
pub fn get_usage(data: &Value) -> Result<Usage> {
    // Extract usage data if available
    if let Some(usage) = data.get("usage") {
        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);

        let total_tokens = match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        };

        Ok(Usage::new(input_tokens, output_tokens, total_tokens))
    } else {
        tracing::debug!(
            "Failed to get usage data: {}",
            ProviderError::UsageError("No usage data found in response".to_string())
        );
        // If no usage data, return None for all values
        Ok(Usage::new(None, None, None))
    }
}
