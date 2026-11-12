use custos_provider::conversation::message::{Message, MessageContent};
use custos_provider::conversation::token_usage::{ProviderUsage, Usage};
use custos_provider::images::ImageFormat;
use custos_provider::request_log::{LoggerHandleExt, RequestLogHandle};
use serde_json::{json, Value};

#[allow(dead_code)]
pub(crate) fn build_openai_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    use custos_provider::cloud::codecs::openai::format_messages;

    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(format_messages(messages, &ImageFormat::OpenAi));
    strip_image_parts_from_messages(&mut arr);
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[allow(dead_code)]
pub(crate) fn build_openai_text_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(messages.iter().filter_map(|m| {
        let content = extract_text_content(m);
        if content.trim().is_empty() {
            return None;
        }
        let role = match m.role {
            rmcp::model::Role::User => "user",
            rmcp::model::Role::Assistant => "assistant",
        };
        Some(json!({"role": role, "content": content}))
    }));
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[allow(dead_code)]
pub(crate) fn convert_text_media_markers(messages: &mut [Value], marker: &str) {
    if marker.is_empty() {
        return;
    }

    for msg in messages {
        let Some(content) = msg.get_mut("content") else {
            continue;
        };

        if let Some(text) = content.as_str() {
            if let Some(parts) = split_media_marker_text(text, marker) {
                *content = json!(parts);
            }
            continue;
        }

        let Some(content_parts) = content.as_array_mut() else {
            continue;
        };
        let mut updated = Vec::new();
        let mut changed = false;
        for part in content_parts.iter() {
            if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    if let Some(parts) = split_media_marker_text(text, marker) {
                        updated.extend(parts);
                        changed = true;
                        continue;
                    }
                }
            }
            updated.push(part.clone());
        }
        if changed {
            *content_parts = updated;
        }
    }
}

#[allow(dead_code)]
pub(crate) fn split_media_marker_text(text: &str, marker: &str) -> Option<Vec<Value>> {
    let mut parts = Vec::new();
    let mut rest = text;
    let mut found_marker = false;
    while let Some((before, after)) = rest.split_once(marker) {
        found_marker = true;
        let before = before.strip_suffix('\n').unwrap_or(before);
        if !before.is_empty() {
            parts.push(json!({"type": "text", "text": before}));
        }
        parts.push(json!({"type": "media_marker", "text": marker}));
        rest = after;
        rest = rest.strip_prefix('\n').unwrap_or(rest);
    }
    if !found_marker {
        return None;
    }
    if !rest.is_empty() {
        parts.push(json!({"type": "text", "text": rest}));
    }
    Some(parts)
}

/// Remove `image_url` content parts from OpenAI-format messages JSON, replacing
/// each with a text note. This prevents an FFI crash in llama.cpp which does not
/// accept `image_url` content-part types.
#[allow(dead_code)]
pub(crate) fn strip_image_parts_from_messages(messages: &mut [Value]) {
    let mut stripped = false;
    for msg in messages.iter_mut() {
        if let Some(content) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            for part in content.iter_mut() {
                if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                    *part = json!({
                        "type": "text",
                        "text": "[Image attached — image input is not supported with the currently selected model]"
                    });
                    stripped = true;
                }
            }
        }
    }
    if stripped {
        tracing::warn!("Stripped image content parts from messages — vision encoder not available for this model");
    }
}

/// Convert a message into plain text for the emulator path's chat history.
pub(crate) fn extract_text_content(msg: &Message) -> String {
    let mut parts = Vec::new();

    for content in &msg.content {
        match content {
            MessageContent::Text(text) => {
                let text = text.text.to_string();
                if !text.trim().is_empty() {
                    parts.push(text);
                }
            }
            MessageContent::ToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    if let Some(cmd) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("command"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("$ {}", cmd));
                    } else if let Some(code) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("code"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("```execute_typescript\n{}\n```", code));
                    }
                }
            }
            MessageContent::ToolResponse(response) => match &response.tool_result {
                Ok(result) => {
                    let mut output_parts = Vec::new();
                    for content_item in &result.content {
                        if let Some(text_content) = content_item.as_text() {
                            output_parts.push(text_content.text.to_string());
                        }
                    }
                    if !output_parts.is_empty() {
                        parts.push(format!("Command output:\n{}", output_parts.join("\n")));
                    }
                }
                Err(e) => {
                    parts.push(format!("Command error: {}", e));
                }
            },
            MessageContent::Image(_) => {
                parts.push(
                    "[Image attached — image input is not supported with the currently selected model]"
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    parts.join("\n")
}

/// Build a `ProviderUsage` and write the request log entry.
#[allow(dead_code)]
pub(crate) fn finalize_usage(
    log: &mut Option<Box<dyn RequestLogHandle>>,
    model_name: String,
    path_label: &str,
    prompt_token_count: usize,
    output_token_count: i32,
    extra_log_fields: Option<(&str, &str)>,
) -> ProviderUsage {
    let input_tokens = prompt_token_count as i32;
    let total_tokens = input_tokens + output_token_count;
    let usage = Usage::new(
        Some(input_tokens),
        Some(output_token_count),
        Some(total_tokens),
    );
    let mut log_json = serde_json::json!({
        "path": path_label,
        "prompt_tokens": input_tokens,
        "output_tokens": output_token_count,
    });
    if let Some((key, value)) = extra_log_fields {
        log_json[key] = serde_json::json!(value);
    }
    let _ = log.write(&log_json, Some(&usage));
    ProviderUsage::new(model_name, usage)
}
