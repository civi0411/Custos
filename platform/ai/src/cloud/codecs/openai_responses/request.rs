use crate::conversation::message::{Message, MessageContentBlock};
use crate::documents::{
    convert_document, document_media_type_is_supported, unsupported_document_text, DocumentFormat,
    ASSISTANT_ROLE_REASON, UNSUPPORTED_MEDIA_TYPE_REASON,
};
use crate::cloud::codecs::openai::{
    extract_reasoning_effort, is_openai_responses_model, openai_reasoning_effort_for_thinking,
    sanitize_function_name,
};
use crate::mcp_utils::extract_text_from_resource;
use crate::model::ModelConfig;
use anyhow::{anyhow, Error};
use rmcp::model::{ContentBlock, Role, Tool};
use serde_json::{json, Value};

pub(crate) fn add_message_items(input_items: &mut Vec<Value>, messages: &[Message], supports_vision: bool) {
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut text_items = Vec::new();

        for content in &message.content {
            match content {
                MessageContentBlock::Text(text) if !text.text.is_empty() => {
                    if message.role == Role::Assistant {
                        // Responses output_text items require annotations even when empty.
                        text_items.push(json!({
                            "type": "output_text",
                            "text": text.text,
                            "annotations": []
                        }));
                    } else {
                        text_items.push(json!({
                            "type": "input_text",
                            "text": text.text
                        }));
                    }
                }
                MessageContentBlock::ToolRequest(request) if message.role == Role::Assistant => {
                    if !text_items.is_empty() {
                        input_items.push(json!({
                            "type": "message",
                            "role": role,
                            "content": text_items
                        }));
                        text_items = Vec::new();
                    }

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

                            tracing::debug!(
                                "Replaying function_call with call_id: {}, name: {}",
                                request.id,
                                tool_call.name
                            );
                            input_items.push(json!({
                                "type": "function_call",
                                "call_id": request.id,
                                "name": sanitized_name,
                                "arguments": arguments_str
                            }));
                        }
                        Err(e) => {
                            input_items.push(json!({
                                "type": "function_call_output",
                                "call_id": request.id,
                                "output": format!("Error: {}", e.message)
                            }));
                        }
                    }
                }
                MessageContentBlock::Image(image) => {
                    if supports_vision {
                        text_items.push(json!({
                            "type": "input_image",
                            "image_url": format!("data:{};base64,{}", image.mime_type, image.data)
                        }));
                    } else {
                        text_items.push(json!({
                            "type": "input_text",
                            "text": "[image omitted: model does not support vision]"
                        }));
                    }
                }
                MessageContentBlock::Document(document) => {
                    if message.role != Role::User {
                        text_items.push(json!({
                            "type": "output_text",
                            "text": unsupported_document_text(document, ASSISTANT_ROLE_REASON),
                            "annotations": []
                        }));
                    } else if document_media_type_is_supported(&document.mime_type) {
                        let mut converted = convert_document(document, &DocumentFormat::OpenAi);
                        let mut file = converted["file"].take();
                        file["type"] = json!("input_file");
                        text_items.push(file);
                    } else {
                        text_items.push(json!({
                            "type": "input_text",
                            "text": unsupported_document_text(document, UNSUPPORTED_MEDIA_TYPE_REASON)
                        }));
                    }
                }
                MessageContentBlock::ToolResponse(response) => {
                    if !text_items.is_empty() {
                        input_items.push(json!({
                            "type": "message",
                            "role": role,
                            "content": text_items
                        }));
                        text_items = Vec::new();
                    }

                    match &response.tool_result {
                        Ok(contents) => {
                            let has_images = supports_vision
                                && contents
                                    .content
                                    .iter()
                                    .any(|c| matches!(c, ContentBlock::Image(_)));

                            let output = if has_images {
                                json!(contents
                                    .content
                                    .iter()
                                    .map(|c| match c {
                                        ContentBlock::Text(t) => json!({
                                            "type": "input_text", "text": t.text
                                        }),
                                        ContentBlock::Resource(r) => json!({
                                            "type": "input_text",
                                            "text": extract_text_from_resource(&r.resource)
                                        }),
                                        ContentBlock::Image(image) => json!({
                                            "type": "input_image",
                                            "image_url": format!(
                                                "data:{};base64,{}",
                                                image.mime_type, image.data
                                            )
                                        }),
                                        ContentBlock::Audio(_) => json!({
                                            "type": "input_text", "text": "[Audio content]"
                                        }),
                                        ContentBlock::ResourceLink(_) => json!({
                                            "type": "input_text", "text": "[Resource link]"
                                        }),
                                        _ => json!({
                                            "type": "input_text", "text": "[Unsupported content]"
                                        }),
                                    })
                                    .collect::<Vec<Value>>())
                            } else {
                                json!(contents
                                    .content
                                    .iter()
                                    .map(|c| match c {
                                        ContentBlock::Text(t) => t.text.clone(),
                                        ContentBlock::Resource(r) => {
                                            extract_text_from_resource(&r.resource)
                                        }
                                        ContentBlock::Audio(_) => "[Audio content]".into(),
                                        ContentBlock::ResourceLink(_) => {
                                            "[Resource link]".into()
                                        }
                                        ContentBlock::Image(_) =>
                                            "[image omitted: model does not support vision]".into(),
                                        _ => "[Unsupported content]".into(),
                                    })
                                    .collect::<Vec<String>>()
                                    .join("\n"))
                            };

                            input_items.push(json!({
                                "type": "function_call_output",
                                "call_id": response.id,
                                "output": output
                            }));
                        }
                        Err(error_data) => {
                            tracing::debug!(
                                "Sending function_call_output error with call_id: {}",
                                response.id
                            );
                            input_items.push(json!({
                                "type": "function_call_output",
                                "call_id": response.id,
                                "output": format!("Error: {}", error_data.message)
                            }));
                        }
                    }
                }
                _ => {}
            }
        }

        if !text_items.is_empty() {
            input_items.push(json!({
                "type": "message",
                "role": role,
                "content": text_items
            }));
        }
    }
}

fn is_gpt_5_6_model(model_name: &str) -> bool {
    let normalized = model_name.to_ascii_lowercase();
    ["gpt-5.6", "gpt-5-6"].iter().any(|needle| {
        normalized.match_indices(needle).any(|(start, matched)| {
            let before = start
                .checked_sub(1)
                .and_then(|index| normalized.as_bytes().get(index));
            let after = normalized.as_bytes().get(start + matched.len());

            before.is_none_or(|byte| matches!(byte, b'-' | b'/'))
                && after.is_none_or(|byte| matches!(byte, b'-' | b'/'))
        })
    })
}

pub fn create_responses_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    let (wire_model_name, _) = extract_reasoning_effort(&model_config.model_name);
    create_responses_request_for_model(
        model_config,
        &wire_model_name,
        &model_config.model_name,
        system,
        messages,
        tools,
    )
}

pub fn create_responses_request_for_model(
    model_config: &ModelConfig,
    wire_model_name: &str,
    capability_model_name: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    let mut input_items = Vec::new();

    if !system.is_empty() {
        input_items.push(json!({
            "type": "message",
            "role": "system",
            "content": [{
                "type": "input_text",
                "text": system
            }]
        }));
    }

    add_message_items(
        &mut input_items,
        messages,
        model_config.supports_vision.unwrap_or_default(),
    );

    let (model_name, legacy_reasoning_effort) = extract_reasoning_effort(capability_model_name);
    // All models routed here are responses-capable; temperature is rejected
    // by the API for reasoning models regardless of whether an explicit
    // effort suffix was provided.
    let is_reasoning_model = is_openai_responses_model(&model_name);
    let reasoning_effort = if is_reasoning_model {
        if let Some(effort) = legacy_reasoning_effort.as_deref() {
            if effort.eq_ignore_ascii_case("none") {
                legacy_reasoning_effort
            } else {
                effort
                    .parse()
                    .ok()
                    .and_then(|effort| openai_reasoning_effort_for_thinking(&model_name, effort))
                    .or(legacy_reasoning_effort)
            }
        } else {
            model_config
                .thinking_effort()
                .and_then(|effort| openai_reasoning_effort_for_thinking(&model_name, effort))
        }
    } else {
        None
    };

    let store = model_config.request_param::<bool>("store").unwrap_or(false);
    let reasoning_mode = model_config
        .request_param::<String>("reasoning_mode")
        .map(|mode| {
            let normalized = mode.to_ascii_lowercase();
            match normalized.as_str() {
                "standard" | "pro" => Ok(normalized),
                _ => Err(anyhow!(
                    "Invalid reasoning_mode '{}'. Supported values are: standard, pro",
                    mode
                )),
            }
        })
        .transpose()?;
    if reasoning_mode.is_some() && !is_gpt_5_6_model(&model_name) {
        return Err(anyhow!(
            "reasoning_mode is only supported for GPT-5.6 models"
        ));
    }
    let mut payload = json!({
        "model": wire_model_name,
        "input": input_items,
        "store": store,
    });

    if reasoning_effort.is_some() || reasoning_mode.is_some() {
        let mut reasoning = serde_json::Map::new();
        if let Some(effort) = reasoning_effort {
            reasoning.insert("effort".to_string(), json!(effort));
            reasoning.insert("summary".to_string(), json!("auto"));
        }
        if let Some(mode) = reasoning_mode {
            reasoning.insert("mode".to_string(), json!(mode));
        }
        payload
            .as_object_mut()
            .unwrap()
            .insert("reasoning".to_string(), Value::Object(reasoning));
    }

    if !tools.is_empty() {
        let tools_spec: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                    "strict": false,
                })
            })
            .collect();

        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if !is_reasoning_model {
        if let Some(temp) = model_config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }
    }

    if let Some(max_tokens) = model_config.max_tokens {
        payload
            .as_object_mut()
            .unwrap()
            .insert("max_output_tokens".to_string(), json!(max_tokens));
    }

    Ok(payload)
}
