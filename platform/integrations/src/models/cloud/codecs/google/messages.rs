use crate::conversation::message::{Message, MessageContentBlock, ProviderMetadata};
use crate::documents::{
    document_media_type_is_supported, unsupported_document_text, UNSUPPORTED_MEDIA_TYPE_REASON,
};
use crate::cloud::codecs::openai::sanitize_function_name;
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::{ContentBlock, ResourceContents, Role};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub const THOUGHT_SIGNATURE_KEY: &str = "thoughtSignature";
pub(crate) const SYNTHETIC_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

pub fn metadata_with_signature(signature: &str) -> ProviderMetadata {
    let mut map = ProviderMetadata::new();
    map.insert(THOUGHT_SIGNATURE_KEY.to_string(), json!(signature));
    map
}

pub fn get_thought_signature(metadata: &Option<ProviderMetadata>) -> Option<&str> {
    metadata
        .as_ref()
        .and_then(|m| m.get(THOUGHT_SIGNATURE_KEY))
        .and_then(|v| v.as_str())
}

fn is_user_loop_boundary(message: &Message) -> bool {
    message.role == Role::User
        && message
            .content
            .iter()
            .any(|content| !matches!(content, MessageContentBlock::ToolResponse(_)))
}

fn insert_thought_signature(part: &mut Map<String, Value>, signature: &str) {
    part.insert(THOUGHT_SIGNATURE_KEY.to_string(), json!(signature));
}

fn maybe_insert_signature_from_metadata(
    part: &mut Map<String, Value>,
    metadata: &Option<ProviderMetadata>,
) {
    if let Some(signature) = get_thought_signature(metadata) {
        insert_thought_signature(part, signature);
    }
}

fn build_function_response_part(
    id: &str,
    name: &str,
    text: String,
    media: Vec<Value>,
) -> Map<String, Value> {
    let mut part = Map::new();
    let mut function_response = Map::new();
    function_response.insert("id".to_string(), json!(id));
    function_response.insert("name".to_string(), json!(name));
    function_response.insert("response".to_string(), json!({"content": {"text": text}}));
    if !media.is_empty() {
        function_response.insert("parts".to_string(), json!(media));
    }
    part.insert("functionResponse".to_string(), json!(function_response));
    part
}

/// Convert internal Message format to Google's API message specification
pub fn format_messages(messages: &[Message], nested_function_response_media: bool) -> Vec<Value> {
    let filtered: Vec<_> = messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .filter(|message| {
            message.content.iter().any(|content| {
                !matches!(
                    content,
                    MessageContentBlock::ToolConfirmationRequest(_)
                        | MessageContentBlock::ActionRequired(_)
                )
            })
        })
        .collect();

    // Record names as we walk the conversation so a reused tool-call id
    // resolves to the nearest preceding request, not a later overwrite.
    let mut tool_names: HashMap<&str, String> = HashMap::new();

    let active_loop_start_idx = filtered
        .iter()
        .enumerate()
        .rev()
        .find(|(_, m)| is_user_loop_boundary(m))
        .map(|(i, _)| i);

    filtered
        .iter()
        .enumerate()
        .filter_map(|(idx, message)| {
            let role = if message.role == Role::User {
                "user"
            } else {
                "model"
            };
            let include_signature = active_loop_start_idx.is_none_or(|start_idx| idx >= start_idx);
            // Only the first model tool call in a turn is guaranteed to carry
            // a signature for loop continuity.
            let mut needs_synthetic_for_first_model_tool_call =
                include_signature && message.role != Role::User;
            let mut parts = Vec::new();
            for message_content in message.content.iter() {
                match message_content {
                    MessageContentBlock::Text(text) => {
                        if !text.text.is_empty() {
                            parts.push(json!({"text": text.text}));
                        }
                    }
                    MessageContentBlock::ToolRequest(request) => match &request.tool_call {
                        Ok(tool_call) => {
                            let name = sanitize_function_name(&tool_call.name);
                            tool_names.insert(request.id.as_str(), name.clone());
                            let mut function_call_part = Map::new();
                            function_call_part.insert("id".to_string(), json!(request.id));
                            function_call_part.insert("name".to_string(), json!(name));

                            if let Some(args) = &tool_call.arguments {
                                if !args.is_empty() {
                                    function_call_part
                                        .insert("args".to_string(), args.clone().into());
                                }
                            }

                            let mut part = Map::new();
                            part.insert("functionCall".to_string(), json!(function_call_part));

                            if include_signature {
                                if let Some(signature) = get_thought_signature(&request.metadata) {
                                    insert_thought_signature(&mut part, signature);
                                } else if needs_synthetic_for_first_model_tool_call {
                                    insert_thought_signature(
                                        &mut part,
                                        SYNTHETIC_THOUGHT_SIGNATURE,
                                    );
                                }
                            }
                            needs_synthetic_for_first_model_tool_call = false;

                            parts.push(json!(part));
                        }
                        Err(e) => {
                            parts.push(json!({"text":format!("Error: {}", e)}));
                        }
                    },
                    MessageContentBlock::ToolResponse(response) => match &response.tool_result {
                        Ok(result) => {
                            let mut tool_content = Vec::new();
                            let mut media = Vec::new();
                            for content in result.content.iter().cloned() {
                                let inline = match &content {
                                    ContentBlock::Image(image) => {
                                        Some((image.mime_type.clone(), image.data.clone()))
                                    }
                                    ContentBlock::Resource(embedded) => match &embedded.resource {
                                        ResourceContents::BlobResourceContents {
                                            blob,
                                            mime_type,
                                            ..
                                        } => mime_type
                                            .clone()
                                            .filter(|m| !m.is_empty())
                                            .map(|mime| (mime, blob.clone())),
                                        _ => None,
                                    },
                                    _ => None,
                                };
                                match inline {
                                    Some((mime, data)) if nested_function_response_media => {
                                        media.push(json!({
                                            "inlineData": {"mimeType": mime, "data": data}
                                        }));
                                    }
                                    Some((mime, data)) => {
                                        parts.push(json!({
                                            "inline_data": {"mime_type": mime, "data": data}
                                        }));
                                    }
                                    None => tool_content.push(content),
                                }
                            }
                            let mut text = tool_content
                                .iter()
                                .filter_map(|c| match c {
                                    ContentBlock::Text(t) => Some(t.text.clone()),
                                    ContentBlock::Resource(raw_embedded_resource) => Some(
                                        extract_text_from_resource(&raw_embedded_resource.resource),
                                    ),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");

                            if text.is_empty() {
                                text = "Tool call is done.".to_string();
                            }
                            let name = tool_names
                                .get(response.id.as_str())
                                .map(String::as_str)
                                .unwrap_or(response.id.as_str());
                            let mut part =
                                build_function_response_part(&response.id, name, text, media);
                            if include_signature {
                                maybe_insert_signature_from_metadata(&mut part, &response.metadata);
                            }
                            parts.push(json!(part));
                        }
                        Err(e) => {
                            let name = tool_names
                                .get(response.id.as_str())
                                .map(String::as_str)
                                .unwrap_or(response.id.as_str());
                            let mut part = build_function_response_part(
                                &response.id,
                                name,
                                format!("Error: {}", e),
                                Vec::new(),
                            );
                            if include_signature {
                                maybe_insert_signature_from_metadata(&mut part, &response.metadata);
                            }
                            parts.push(json!(part));
                        }
                    },
                    MessageContentBlock::Thinking(_) => {}
                    MessageContentBlock::Image(image) => {
                        parts.push(json!({
                            "inline_data": {
                                "mime_type": image.mime_type,
                                "data": image.data,
                            }
                        }));
                    }
                    MessageContentBlock::Document(document) => {
                        if document_media_type_is_supported(&document.mime_type) {
                            parts.push(json!({
                                "inline_data": {
                                    "mime_type": document.mime_type,
                                    "data": document.data,
                                }
                            }));
                        } else {
                            parts.push(json!({
                                "text": unsupported_document_text(document, UNSUPPORTED_MEDIA_TYPE_REASON)
                            }));
                        }
                    }

                    _ => {}
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(json!({"role": role, "parts": parts}))
            }
        })
        .collect()
}

