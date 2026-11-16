use crate::base::ThinkingPreservationFormat;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::documents::{
    convert_document, document_media_type_is_supported, unsupported_document_text, DocumentFormat,
    ASSISTANT_ROLE_REASON, UNSUPPORTED_MEDIA_TYPE_REASON,
};
use crate::images::{convert_image, detect_image_path, load_image_file, ImageFormat};
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::{ContentBlock, Role};
use serde_json::{json, Value};
use super::options::{merge_reasoning_text, sanitize_function_name, OpenAiFormatOptions};

pub fn format_messages(messages: &[Message], image_format: &ImageFormat) -> Vec<Value> {
    format_messages_with_options(
        messages,
        image_format,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    )
}

pub fn format_messages_with_options(
    messages: &[Message],
    image_format: &ImageFormat,
    options: OpenAiFormatOptions,
) -> Vec<Value> {
    let mut messages_spec = Vec::new();
    let mut pending_assistant_reasoning = String::new();
    // Reasoning to propagate across consecutive tool-call messages in the same turn.
    // DeepSeek/Kimi require reasoning_content on every assistant tool-call message.
    let mut tool_call_turn_reasoning = String::new();
    let mut saw_tool_response = false;

    for message in messages {
        if options.preserve_thinking_context && message.role != Role::Assistant {
            pending_assistant_reasoning.clear();
        }

        if options.preserve_thinking_context && message.role == Role::User {
            if message
                .content
                .iter()
                .any(|c| matches!(c, MessageContentBlock::ToolResponse(_)))
            {
                saw_tool_response = true;
            } else {
                tool_call_turn_reasoning.clear();
                saw_tool_response = false;
            }
        }

        // A new assistant message after tool results creates a new turn.
        // Prevents reasoning from the previous turn leaking into the new one.
        if options.preserve_thinking_context && message.role == Role::Assistant && saw_tool_response
        {
            tool_call_turn_reasoning.clear();
            saw_tool_response = false;
        }

        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();
        let mut content_array = Vec::new();
        let mut has_non_text_content = false;
        let mut reasoning_text = String::new();

        for content in &message.content {
            match content {
                MessageContentBlock::Text(text) => {
                    if !text.text.is_empty() {
                        if message.role == Role::User {
                            if options.supports_vision {
                                if let Some(image_path) = detect_image_path(&text.text) {
                                    if let Ok(image) = load_image_file(image_path.as_ref()) {
                                        has_non_text_content = true;
                                        content_array
                                            .push(json!({"type": "text", "text": text.text}));
                                        content_array.push(convert_image(&image, image_format));
                                    } else {
                                        content_array
                                            .push(json!({"type": "text", "text": text.text}));
                                    }
                                } else {
                                    content_array.push(json!({"type": "text", "text": text.text}));
                                }
                            } else {
                                content_array.push(json!({"type": "text", "text": text.text}));
                            }
                        } else {
                            content_array.push(json!({"type": "text", "text": text.text}));
                        }
                    }
                }
                MessageContentBlock::Thinking(t) => {
                    reasoning_text.push_str(&t.thinking);
                }
                MessageContentBlock::RedactedThinking(_) => {
                    continue;
                }
                MessageContentBlock::SystemNotification(_) | MessageContentBlock::Error(_) => {
                    continue;
                }
                MessageContentBlock::ToolRequest(request) => match &request.tool_call {
                    Ok(tool_call) => {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let arguments_str = match &tool_call.arguments {
                            Some(args) => {
                                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                            }
                            None => "{}".to_string(),
                        };

                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));

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

                        tool_calls.as_array_mut().unwrap().push(tool_call_json);
                    }
                    Err(_e) => {
                        // An unparseable tool call still needs a valid assistant
                        // `tool_calls` entry. Emitting the error as a bare `role:"tool"`
                        // message (the old behavior) leaves the paired tool response —
                        // which carries the parse error — as an orphan `role:"tool"` with
                        // no preceding assistant `tool_calls`, which strict
                        // OpenAI-compatible APIs reject. Emit a placeholder call with the
                        // same id so the history stays well-formed; the error rides on the
                        // following tool response.
                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));
                        tool_calls.as_array_mut().unwrap().push(json!({
                            "id": request.id,
                            "type": "function",
                            "function": {
                                "name": "unparseable_tool_call",
                                "arguments": "{}",
                            }
                        }));
                    }
                },
                MessageContentBlock::ToolResponse(response) => {
                    match &response.tool_result {
                        Ok(result) => {
                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in result.content.iter() {
                                match content {
                                    ContentBlock::Image(image) => {
                                        if options.supports_vision {
                                            // Add placeholder text in the tool response
                                            tool_content.push(ContentBlock::text("This tool result included an image that is uploaded in the next message."));

                                            // Create a separate image message
                                            image_messages.push(json!({
                                                "role": "user",
                                                "content": [convert_image(&image.clone(), image_format)]
                                            }));
                                        } else {
                                            // Add placeholder text in the tool response
                                            tool_content.push(ContentBlock::text("This tool result included an image that was omitted as the model does not support vision."));
                                        }
                                    }
                                    ContentBlock::Resource(resource) => {
                                        let text = extract_text_from_resource(&resource.resource);
                                        tool_content.push(ContentBlock::text(text));
                                    }
                                    _ => {
                                        tool_content.push(content.clone());
                                    }
                                }
                            }
                            let tool_response_content: Value = json!(tool_content
                                .iter()
                                .map(|content| match content {
                                    ContentBlock::Text(text) => text.text.clone(),
                                    _ => String::new(),
                                })
                                .collect::<Vec<String>>()
                                .join(" "));

                            // First add the tool response with all content
                            output.push(json!({
                                "role": "tool",
                                "content": tool_response_content,
                                "tool_call_id": response.id
                            }));
                            // Then add any image messages that need to follow
                            output.extend(image_messages);
                        }
                        Err(e) => {
                            // A tool result error is shown as output so the model can interpret the error message
                            output.push(json!({
                                "role": "tool",
                                "content": format!("The tool call returned the following error:\n{}", e),
                                "tool_call_id": response.id
                            }));
                        }
                    }
                }
                MessageContentBlock::ToolConfirmationRequest(_) => {}
                MessageContentBlock::ActionRequired(_) => {}
                MessageContentBlock::Image(image) => {
                    if message.role == Role::User {
                        if options.supports_vision {
                            has_non_text_content = true;
                            content_array.push(convert_image(image, image_format));
                        } else {
                            content_array.push(json!({
                                "type": "text",
                                "text": "[image omitted: model does not support vision]"
                            }));
                        }
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": "[Image content removed - not supported in assistant messages]"
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
                        has_non_text_content = true;
                        content_array.push(convert_document(document, &DocumentFormat::OpenAi));
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": unsupported_document_text(document, UNSUPPORTED_MEDIA_TYPE_REASON)
                        }));
                    }
                }
            }
        }

        if !content_array.is_empty() {
            if has_non_text_content {
                converted["content"] = json!(content_array);
            } else {
                let texts: Vec<String> = content_array
                    .iter()
                    .filter_map(|v| v["text"].as_str().map(|s| s.to_string()))
                    .collect();
                converted["content"] = json!(texts.join("\n"));
            }
        }

        // Some strict OpenAI-compatible providers require "content" to be present
        // (even as null) when tool_calls are provided. See #6717.
        if message.role == Role::Assistant
            && converted.get("tool_calls").is_some()
            && converted.get("content").is_none()
        {
            converted["content"] = json!(null);
        }

        let has_message_payload =
            converted.get("content").is_some() || converted.get("tool_calls").is_some();

        if options.preserve_thinking_context && message.role == Role::Assistant {
            if !has_message_payload && output.is_empty() && !reasoning_text.is_empty() {
                pending_assistant_reasoning.push_str(&reasoning_text);
                continue;
            }

            if !pending_assistant_reasoning.is_empty() {
                reasoning_text =
                    merge_reasoning_text(&pending_assistant_reasoning, &reasoning_text);
                pending_assistant_reasoning.clear();
            }

            let has_tool_calls = converted
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
                .is_some_and(|a| !a.is_empty());

            if has_tool_calls {
                if reasoning_text.is_empty() {
                    reasoning_text = tool_call_turn_reasoning.clone();
                } else {
                    tool_call_turn_reasoning = reasoning_text.clone();
                }
            } else {
                // Carry reasoning forward even through non-tool assistant messages
                // (e.g., a visible text chunk that's is sent before a tool-call chunk
                // in the same streaming turn). An empty reasoning_text is equivalent
                // to clear.
                tool_call_turn_reasoning = reasoning_text.clone();
            }
        }

        // Include reasoning_content only when non-empty. Kimi rejects empty
        // reasoning_content (""), so we must omit it entirely.
        if options.preserve_thinking_context && !reasoning_text.is_empty() {
            converted["reasoning_content"] = json!(reasoning_text);
        }

        if has_message_payload {
            output.insert(0, converted);
        }

        messages_spec.extend(output);
    }

    merge_split_tool_call_messages(&mut messages_spec);

    if let Some(format) = options.thinking_preservation_format {
        inline_reasoning_content(&mut messages_spec, format);
    }

    messages_spec
}

/// Rewrites `reasoning_content` into the message `content` for models that reject a
/// separate reasoning field on replay.
///
/// Must run after `merge_split_tool_call_messages`, which relies on `reasoning_content`
/// to identify messages split from the same assistant turn.
fn inline_reasoning_content(messages: &mut [Value], format: ThinkingPreservationFormat) {
    let wrap: fn(&str) -> String = match format {
        ThinkingPreservationFormat::ReasoningContent => return,
        ThinkingPreservationFormat::ContentPrepend => |text| format!("{text}\n\n"),
        ThinkingPreservationFormat::ContentXml => |text| format!("<think>\n{text}\n</think>\n\n"),
    };

    for message in messages {
        let Some(object) = message.as_object_mut() else {
            continue;
        };
        let Some(Value::String(reasoning)) = object.remove("reasoning_content") else {
            continue;
        };
        let prefix = wrap(&reasoning);

        match object.entry("content").or_insert(Value::Null) {
            Value::String(content) => content.insert_str(0, &prefix),
            Value::Array(blocks) => blocks.insert(0, json!({"type": "text", "text": prefix})),
            content => *content = json!(prefix.trim_end()),
        }
    }
}

/// The agent splits a single assistant response with N tool_calls into N
/// interleaved `asst(TC)/tool` pairs, cloning `reasoning_content` onto each.
/// This function merges them back into one assistant message with all tool_calls,
/// followed by the tool results — the standard OpenAI format.
///
/// Only merges when `reasoning_content` is present and matches, since that is
/// the only signal that messages were split from the same turn.
pub(crate) fn merge_split_tool_call_messages(messages: &mut Vec<Value>) {
    let mut i = 0;
    while i < messages.len() {
        let is_assistant_tool_call = messages[i].get("role") == Some(&json!("assistant"))
            && messages[i]
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
                .is_some_and(|a| !a.is_empty());
        let base_reasoning = messages[i].get("reasoning_content");

        if !is_assistant_tool_call || base_reasoning.is_none() {
            i += 1;
            continue;
        }
        let base_reasoning = base_reasoning.unwrap().clone();

        let mut extra_tool_calls: Vec<Value> = Vec::new();
        let mut collected: Vec<Value> = Vec::new();
        let mut scan = i + 1;

        loop {
            if scan >= messages.len() || messages[scan].get("role") != Some(&json!("tool")) {
                break;
            }

            // Skip past tool result and any image-only user messages that
            // format_messages inserts after tool results containing images.
            let mut peek = scan + 1;
            while peek < messages.len() && is_image_only_user_message(&messages[peek]) {
                peek += 1;
            }

            if peek >= messages.len() {
                break;
            }
            let next = &messages[peek];
            let has_no_content = next.get("content").is_none_or(|c| {
                c.is_null()
                    || c.as_str().is_some_and(|s| s.is_empty())
                    || c.as_array().is_some_and(|a| a.is_empty())
            });
            let is_split = next.get("role") == Some(&json!("assistant"))
                && next
                    .get("tool_calls")
                    .and_then(|tc| tc.as_array())
                    .is_some_and(|a| !a.is_empty())
                && has_no_content
                && next.get("reasoning_content") == Some(&base_reasoning);

            if !is_split {
                break;
            }

            collected.extend(messages[scan..peek].iter().cloned());
            if let Some(tc) = messages[peek]
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
            {
                extra_tool_calls.extend(tc.iter().cloned());
            }
            scan = peek + 1;
        }

        if extra_tool_calls.is_empty() {
            i += 1;
            continue;
        }

        if let Some(base_tc) = messages[i]
            .get_mut("tool_calls")
            .and_then(|tc| tc.as_array_mut())
        {
            base_tc.extend(extra_tool_calls);
        }

        let insert_at = i + 1;
        messages.drain(insert_at..scan);
        let num_collected = collected.len();
        for (j, msg) in collected.into_iter().enumerate() {
            messages.insert(insert_at + j, msg);
        }

        i = insert_at + num_collected;
    }
}

/// True if `msg` is a synthetic image-only user message (content is exclusively image_url items).
fn is_image_only_user_message(msg: &Value) -> bool {
    msg.get("role") == Some(&json!("user"))
        && msg
            .get("content")
            .and_then(|c| c.as_array())
            .is_some_and(|arr| {
                !arr.is_empty()
                    && arr
                        .iter()
                        .all(|item| item.get("type") == Some(&json!("image_url")))
            })
}

