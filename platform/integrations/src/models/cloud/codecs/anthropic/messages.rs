use crate::conversation::message::{Message, MessageContentBlock};
use crate::documents::{
    convert_document, document_media_type_is_supported, unsupported_document_text, DocumentFormat,
    ASSISTANT_ROLE_REASON, UNSUPPORTED_MEDIA_TYPE_REASON,
};
use crate::images::{convert_image, ImageFormat};
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::{ContentBlock, JsonObject, ResourceContents, Role};
use serde_json::{json, Value};
use super::options::*;

pub fn args_to_input_value(arguments: Option<JsonObject>) -> Value {
    Value::Object(arguments.unwrap_or_default())
}

/// Convert internal Message format to Anthropic's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    format_messages_with_options(messages, &AnthropicFormatOptions::default())
}

pub fn format_messages_with_options(
    messages: &[Message],
    options: &AnthropicFormatOptions,
) -> Vec<Value> {
    let mut anthropic_messages = Vec::new();

    for message in messages {
        let role = match message.role {
            Role::User => USER_ROLE,
            Role::Assistant => ASSISTANT_ROLE,
        };

        let thinking_is_stale = thinking_block_is_stale(message, options.current_model.as_deref());
        let replay_thinking =
            !options.thinking_disabled && !options.strip_thinking_history && !thinking_is_stale;

        let mut content = Vec::new();
        for msg_content in &message.content {
            match msg_content {
                MessageContentBlock::Text(text) => {
                    if !text.text.trim().is_empty() {
                        content.push(json!({
                            TYPE_FIELD: TEXT_TYPE,
                            TEXT_TYPE: text.text
                        }));
                    }
                }
                MessageContentBlock::ToolRequest(tool_request) => {
                    match &tool_request.tool_call {
                        Ok(tool_call) => {
                            content.push(json!({
                                TYPE_FIELD: TOOL_USE_TYPE,
                                ID_FIELD: tool_request.id,
                                NAME_FIELD: tool_call.name,
                                INPUT_FIELD: args_to_input_value(tool_call.arguments.clone())
                            }));
                        }
                        Err(_tool_error) => {
                            // The paired tool response carries the parse error and
                            // serializes to a tool_result below; Anthropic rejects a
                            // tool_result without a preceding tool_use, so emit a
                            // placeholder tool_use with the same id to keep history valid.
                            content.push(json!({
                                TYPE_FIELD: TOOL_USE_TYPE,
                                ID_FIELD: tool_request.id,
                                NAME_FIELD: "unparseable_tool_call",
                                INPUT_FIELD: json!({})
                            }));
                        }
                    }
                }
                MessageContentBlock::ToolResponse(tool_response) => {
                    match &tool_response.tool_result {
                        Ok(result) => {
                            let mut blocks: Vec<Value> = Vec::new();
                            let mut text_parts: Vec<String> = Vec::new();
                            let mut has_media = false;

                            for c in result.content.iter() {
                                if let Some(t) = c.as_text() {
                                    text_parts.push(t.text.clone());
                                    if !t.text.is_empty() {
                                        blocks.push(json!({
                                            TYPE_FIELD: TEXT_TYPE,
                                            TEXT_TYPE: t.text.clone()
                                        }));
                                    }
                                    continue;
                                }
                                if let Some(r) = c.as_resource() {
                                    // Claude only accepts a fixed set of media types, so
                                    // unsupported blobs fall back to text below rather than
                                    // being rejected by the provider.
                                    if let ResourceContents::BlobResourceContents {
                                        blob,
                                        mime_type,
                                        ..
                                    } = &r.resource
                                    {
                                        let mime = mime_type.as_deref().unwrap_or("");
                                        if ANTHROPIC_IMAGE_MEDIA_TYPES.contains(&mime) {
                                            has_media = true;
                                            blocks.push(json!({
                                                TYPE_FIELD: IMAGE_TYPE,
                                                SOURCE_FIELD: {
                                                    TYPE_FIELD: BASE64_TYPE,
                                                    MEDIA_TYPE_FIELD: mime,
                                                    DATA_FIELD: blob,
                                                }
                                            }));
                                            continue;
                                        }
                                        if mime == "application/pdf" {
                                            has_media = true;
                                            blocks.push(json!({
                                                TYPE_FIELD: DOCUMENT_TYPE,
                                                SOURCE_FIELD: {
                                                    TYPE_FIELD: BASE64_TYPE,
                                                    MEDIA_TYPE_FIELD: mime,
                                                    DATA_FIELD: blob,
                                                }
                                            }));
                                            continue;
                                        }
                                    }
                                    let text = extract_text_from_resource(&r.resource);
                                    if !text.is_empty() {
                                        text_parts.push(text.clone());
                                        blocks.push(json!({
                                            TYPE_FIELD: TEXT_TYPE,
                                            TEXT_TYPE: text
                                        }));
                                    }
                                    continue;
                                }
                                if let ContentBlock::Image(image) = c {
                                    if ANTHROPIC_IMAGE_MEDIA_TYPES
                                        .contains(&image.mime_type.as_str())
                                    {
                                        has_media = true;
                                        blocks.push(convert_image(
                                            &image.clone(),
                                            &ImageFormat::Anthropic,
                                        ));
                                    } else {
                                        let marker = format!("[Image: {}]", image.mime_type);
                                        text_parts.push(marker.clone());
                                        blocks.push(json!({
                                            TYPE_FIELD: TEXT_TYPE,
                                            TEXT_TYPE: marker
                                        }));
                                    }
                                }
                            }

                            let content_value = if has_media {
                                Value::Array(blocks)
                            } else {
                                Value::String(text_parts.join("\n"))
                            };

                            content.push(json!({
                                TYPE_FIELD: TOOL_RESULT_TYPE,
                                TOOL_USE_ID_FIELD: tool_response.id,
                                CONTENT_FIELD: content_value
                            }));
                        }
                        Err(tool_error) => {
                            content.push(json!({
                                TYPE_FIELD: TOOL_RESULT_TYPE,
                                TOOL_USE_ID_FIELD: tool_response.id,
                                CONTENT_FIELD: format!("Error: {}", tool_error),
                                IS_ERROR_FIELD: true
                            }));
                        }
                    }
                }
                MessageContentBlock::ToolConfirmationRequest(_tool_confirmation_request) => {
                    // Skip tool confirmation requests
                }
                MessageContentBlock::ActionRequired(_action_required) => {
                    // Skip action required messages - they're for UI only
                }
                MessageContentBlock::SystemNotification(_) | MessageContentBlock::Error(_) => {
                    // Skip
                }
                MessageContentBlock::Thinking(thinking) => {
                    // Anthropic rejects thinking blocks sent without a matching thinking config.
                    if replay_thinking {
                        if !thinking.signature.is_empty() {
                            content.push(json!({
                                TYPE_FIELD: THINKING_TYPE,
                                THINKING_TYPE: thinking.thinking,
                                SIGNATURE_FIELD: thinking.signature
                            }));
                        } else if options.preserve_unsigned_thinking
                            && !thinking.thinking.is_empty()
                        {
                            content.push(json!({
                                TYPE_FIELD: THINKING_TYPE,
                                THINKING_TYPE: thinking.thinking
                            }));
                        }
                    }
                }
                MessageContentBlock::RedactedThinking(redacted) => {
                    if replay_thinking {
                        content.push(json!({
                            TYPE_FIELD: REDACTED_THINKING_TYPE,
                            DATA_FIELD: redacted.data
                        }));
                    }
                }
                MessageContentBlock::Image(image) => {
                    content.push(convert_image(image, &ImageFormat::Anthropic));
                }
                MessageContentBlock::Document(document) => {
                    if message.role != Role::User {
                        content.push(json!({
                            TYPE_FIELD: TEXT_TYPE,
                            TEXT_TYPE: unsupported_document_text(document, ASSISTANT_ROLE_REASON)
                        }));
                    } else if document_media_type_is_supported(&document.mime_type) {
                        content.push(convert_document(document, &DocumentFormat::Anthropic));
                    } else {
                        content.push(json!({
                            TYPE_FIELD: TEXT_TYPE,
                            TEXT_TYPE: unsupported_document_text(
                                document,
                                UNSUPPORTED_MEDIA_TYPE_REASON,
                            )
                        }));
                    }
                }
            }
        }

        // Skip messages with empty content
        if !content.is_empty() {
            anthropic_messages.push(json!({
                ROLE_FIELD: role,
                CONTENT_FIELD: content
            }));
        }
    }

    if anthropic_messages.is_empty() {
        anthropic_messages.push(json!({
            ROLE_FIELD: USER_ROLE,
            CONTENT_FIELD: [{
                TYPE_FIELD: TEXT_TYPE,
                TEXT_TYPE: "Ignore"
            }]
        }));
    }

    if options.prompt_cache_disabled {
        return anthropic_messages;
    }

    // The last two user messages extend the cached prefix each turn.
    let mut user_count = 0;
    for message in anthropic_messages.iter_mut().rev() {
        if message.get(ROLE_FIELD) != Some(&json!(USER_ROLE)) {
            continue;
        }
        if let Some(block) = message
            .get_mut(CONTENT_FIELD)
            .and_then(|content| content.as_array_mut())
            .and_then(|content_array| content_array.last_mut())
            .and_then(|b| b.as_object_mut())
        {
            block.insert(CACHE_CONTROL_FIELD.to_string(), options.cache_control());
            user_count += 1;
            if user_count >= 2 {
                break;
            }
        }
    }

    anthropic_messages
}

