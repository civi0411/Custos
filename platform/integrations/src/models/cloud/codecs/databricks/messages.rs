use crate::conversation::message::{Message, MessageContentBlock};
use crate::documents::{
    convert_document, document_media_type_is_supported, unsupported_document_text, DocumentFormat,
    ASSISTANT_ROLE_REASON, UNSUPPORTED_MEDIA_TYPE_REASON,
};
use crate::cloud::codecs::anthropic::thinking_block_is_stale;
use crate::cloud::codecs::openai::sanitize_function_name;
use crate::images::{convert_image, detect_image_path, load_image_file, ImageFormat};
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::{ContentBlock, Role};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
pub(crate) struct DatabricksMessage {
    pub(crate) content: Value,
    pub(crate) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
}

fn format_text_content(
    text: &str,
    image_format: &ImageFormat,
    supports_vision: bool,
) -> (Vec<Value>, bool) {
    let mut items = vec![json!({"type": "text", "text": text})];
    let has_image = if supports_vision {
        if let Some(path) = detect_image_path(text) {
            if let Ok(image) = load_image_file(path.as_ref()) {
                items.push(convert_image(&image, image_format));
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    (items, has_image)
}

fn format_tool_response(
    response: &crate::conversation::message::ToolResponse,
    image_format: &ImageFormat,
    supports_vision: bool,
) -> Vec<DatabricksMessage> {
    let mut result = Vec::new();

    match &response.tool_result {
        Ok(call_result) => {
            let abridged: Vec<_> = call_result.content.to_vec();

            let mut tool_content = Vec::new();
            let mut image_messages = Vec::new();

            for content in abridged {
                match content {
                    ContentBlock::Image(image) => {
                        if supports_vision {
                            tool_content.push(ContentBlock::text(
                            "This tool result included an image that is uploaded in the next message.",
                        ));
                            image_messages.push(DatabricksMessage {
                                role: "user".to_string(),
                                content: [convert_image(&image, image_format)].into(),
                                tool_calls: None,
                                tool_call_id: None,
                            });
                        } else {
                            tool_content.push(ContentBlock::text(
                                "This tool result included an image that was omitted as the model does not support vision.",
                            ));
                        }
                    }
                    ContentBlock::Resource(resource) => {
                        let text = extract_text_from_resource(&resource.resource);
                        tool_content.push(ContentBlock::text(text));
                    }
                    _ => tool_content.push(content),
                }
            }

            let tool_response_content: Value = json!(tool_content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<String>>()
                .join(" "));

            result.push(DatabricksMessage {
                content: tool_response_content,
                role: "tool".to_string(),
                tool_call_id: Some(response.id.clone()),
                tool_calls: None,
            });
            result.extend(image_messages);
        }
        Err(e) => {
            result.push(DatabricksMessage {
                role: "tool".to_string(),
                content: format!("The tool call returned the following error:\n{}", e).into(),
                tool_call_id: Some(response.id.clone()),
                tool_calls: None,
            });
        }
    }

    result
}

pub(crate) fn format_messages(
    messages: &[Message],
    image_format: &ImageFormat,
    current_model: Option<&str>,
    supports_vision: bool,
) -> Vec<DatabricksMessage> {
    let mut result = Vec::new();
    for message in messages {
        let thinking_is_stale = thinking_block_is_stale(message, current_model);
        let mut converted = DatabricksMessage {
            content: Value::Null,
            role: match message.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            tool_calls: None,
            tool_call_id: None,
        };

        let mut content_array = Vec::new();
        let mut has_tool_calls = false;
        let mut has_multiple_content = false;
        // Deferred so all tool-role messages stay consecutive (required by Claude via Databricks).
        let mut pending_image_messages: Vec<DatabricksMessage> = Vec::new();

        for content in &message.content {
            match content {
                MessageContentBlock::Text(text) => {
                    if !text.text.is_empty() {
                        let (items, multi) =
                            format_text_content(&text.text, image_format, supports_vision);
                        content_array.extend(items);
                        has_multiple_content |= multi;
                    }
                }
                MessageContentBlock::Thinking(content) => {
                    if !thinking_is_stale {
                        has_multiple_content = true;
                        content_array.push(json!({
                            "type": "reasoning",
                            "summary": [{
                                "type": "summary_text",
                                "text": content.thinking,
                                "signature": content.signature
                            }]
                        }));
                    }
                }
                MessageContentBlock::RedactedThinking(content) => {
                    if !thinking_is_stale {
                        has_multiple_content = true;
                        content_array.push(json!({
                            "type": "reasoning",
                            "summary": [{"type": "summary_encrypted_text", "data": content.data}]
                        }));
                    }
                }
                MessageContentBlock::ToolRequest(request) => {
                    has_tool_calls = true;
                    match &request.tool_call {
                        Ok(tool_call) => {
                            let sanitized_name = sanitize_function_name(&tool_call.name);
                            let arguments_str = tool_call
                                .arguments
                                .as_ref()
                                .map(|args| {
                                    serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                                })
                                .unwrap_or_else(|| "{}".to_string());

                            let tool_calls = converted.tool_calls.get_or_insert_with(Vec::new);
                            let mut tool_call_json = json!({
                                "id": request.id,
                                "type": "function",
                                "function": {
                                    "name": sanitized_name,
                                    "arguments": arguments_str,
                                }
                            });

                            if let Some(metadata) = &request.metadata {
                                for (key, value) in metadata {
                                    tool_call_json[key] = value.clone();
                                }
                            }

                            tool_calls.push(tool_call_json);
                        }
                        Err(_e) => {
                            // Mirror the OpenAI formatter: emitting the error as assistant
                            // text leaves no `tool_calls` entry, so the paired tool response
                            // orphans (a `role:"tool"` with no preceding assistant
                            // `tool_calls`) and strict APIs reject it. Emit a placeholder
                            // call with the same id; the error rides on the tool response.
                            let tool_calls = converted.tool_calls.get_or_insert_with(Vec::new);
                            tool_calls.push(json!({
                                "id": request.id,
                                "type": "function",
                                "function": {
                                    "name": "unparseable_tool_call",
                                    "arguments": "{}",
                                }
                            }));
                        }
                    }
                }
                MessageContentBlock::ToolResponse(response) => {
                    for msg in format_tool_response(response, image_format, supports_vision) {
                        if msg.role == "user" {
                            pending_image_messages.push(msg);
                        } else {
                            result.push(msg);
                        }
                    }
                }
                MessageContentBlock::Image(image) => {
                    if supports_vision {
                        content_array.push(convert_image(image, image_format));
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": "[image omitted: model does not support vision]"
                        }));
                    }
                }
                MessageContentBlock::Document(document) => {
                    if message.role != Role::User {
                        content_array.push(json!({
                            "type": "text",
                            "text": unsupported_document_text(document, ASSISTANT_ROLE_REASON)
                        }));
                    } else if document_media_type_is_supported(&document.mime_type) {
                        content_array.push(convert_document(document, &DocumentFormat::OpenAi));
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": unsupported_document_text(document, UNSUPPORTED_MEDIA_TYPE_REASON)
                        }));
                    }
                }
                MessageContentBlock::SystemNotification(_)
                | MessageContentBlock::Error(_)
                | MessageContentBlock::ToolConfirmationRequest(_)
                | MessageContentBlock::ActionRequired(_) => {}
            }
        }

        result.extend(pending_image_messages);

        if !content_array.is_empty() {
            converted.content = if content_array.len() == 1
                && !has_multiple_content
                && content_array[0]["type"] == "text"
            {
                json!(content_array[0]["text"])
            } else {
                json!(content_array)
            };
        }

        if !content_array.is_empty() || has_tool_calls {
            result.push(converted);
        }
    }

    result
}
