use crate::conversation::message::{Message, MessageContentBlock};
use crate::cloud::codecs::openai::is_valid_function_name;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde_json::Value;
use std::borrow::Cow;

/// Convert Databricks' API response to internal Message format
#[allow(clippy::too_many_lines)]
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let original = &response["choices"][0]["message"];
    let mut content = Vec::new();

    // Handle array-based content
    if let Some(content_array) = original.get("content").and_then(|c| c.as_array()) {
        for content_item in content_array {
            match content_item.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                        content.push(MessageContentBlock::text(text));
                    }
                }
                Some("reasoning") => {
                    if let Some(summary_array) =
                        content_item.get("summary").and_then(|s| s.as_array())
                    {
                        for summary in summary_array {
                            match summary.get("type").and_then(|t| t.as_str()) {
                                Some("summary_text") => {
                                    let text = summary
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or_default();
                                    let signature = summary
                                        .get("signature")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or_default();
                                    content.push(MessageContentBlock::thinking(text, signature));
                                }
                                Some("summary_encrypted_text") => {
                                    if let Some(data) = summary.get("data").and_then(|d| d.as_str())
                                    {
                                        content.push(MessageContentBlock::redacted_thinking(data));
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    } else if let Some(text) = original.get("content").and_then(|t| t.as_str()) {
        // Handle legacy single string content
        content.push(MessageContentBlock::text(text));
    }

    // Handle tool calls
    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // Get the raw arguments string from the LLM.
                let arguments_str = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // If arguments_str is empty, default to an empty JSON object string.
                let arguments_str = if arguments_str.is_empty() {
                    "{}".to_string()
                } else {
                    arguments_str
                };

                if !is_valid_function_name(&function_name) {
                    let error = ErrorData {
                        code: ErrorCode::INVALID_REQUEST,
                        message: Cow::from(format!(
                            "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                            function_name
                        )),
                        data: None,
                    };
                    content.push(MessageContentBlock::tool_request(id, Err(error)));
                } else {
                    match crate::json::parse_tool_arguments(&arguments_str) {
                        Some(params) if params.is_object() => {
                            content.push(MessageContentBlock::tool_request(
                                id,
                                Ok(CallToolRequestParams::new(function_name)
                                    .with_arguments(object(params))),
                            ));
                        }
                        // Valid JSON but NOT an object (a bare array/string/number).
                        // Surface a tool error so the model retries instead of
                        // crashing the run (rmcp's `object()` debug-asserts).
                        Some(_) => {
                            let error = ErrorData {
                                code: ErrorCode::INVALID_PARAMS,
                                message: Cow::from(format!(
                                    "Tool arguments for {} (id {}) must be a JSON object. Raw arguments: '{}'",
                                    function_name, id, arguments_str
                                )),
                                data: None,
                            };
                            content.push(MessageContentBlock::tool_request(id, Err(error)));
                        }
                        None => {
                            let message_text =
                                crate::json::truncation_error_message(&arguments_str)
                                    .unwrap_or_else(|| {
                                        format!(
                                            "Could not interpret tool use parameters for id {id}"
                                        )
                                    });
                            let error = ErrorData {
                                code: ErrorCode::INVALID_PARAMS,
                                message: Cow::from(message_text),
                                data: None,
                            };
                            content.push(MessageContentBlock::tool_request(id, Err(error)));
                        }
                    }
                }
            }
        }
    }

    Ok(Message::new(
        Role::Assistant,
        chrono::Utc::now().timestamp(),
        content,
    ))
}
