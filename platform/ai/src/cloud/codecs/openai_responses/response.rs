use super::types::*;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::Usage;
use crate::utils::{sanitize_unicode_tags, strip_unicode_tags};
use anyhow::anyhow;
use rmcp::model::{object, CallToolRequestParams, Role};
use serde_json::{json, Value};
use std::collections::HashSet;

pub(crate) fn reasoning_from_summary(summary: &[SummaryText]) -> Option<MessageContentBlock> {
    let text: String = summary
        .iter()
        .map(|s| sanitize_unicode_tags(&s.text))
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        None
    } else {
        Some(MessageContentBlock::thinking(text, ""))
    }
}

pub(crate) fn sanitize_tool_arguments(value: Value) -> anyhow::Result<Value> {
    match value {
        Value::String(text) => Ok(Value::String(strip_unicode_tags(&text))),
        Value::Array(values) => Ok(Value::Array(
            values
                .into_iter()
                .map(sanitize_tool_arguments)
                .collect::<anyhow::Result<_>>()?,
        )),
        Value::Object(values) => {
            let mut sanitized = serde_json::Map::new();
            for (key, value) in values {
                let key = strip_unicode_tags(&key);
                if sanitized.contains_key(&key) {
                    return Err(anyhow!(
                        "Responses tool arguments contain duplicate key after Unicode tag sanitization"
                    ));
                }
                sanitized.insert(key, sanitize_tool_arguments(value)?);
            }
            Ok(Value::Object(sanitized))
        }
        value => Ok(value),
    }
}

pub(crate) fn parse_tool_arguments(arguments: &str) -> anyhow::Result<Value> {
    if arguments.is_empty() {
        Ok(json!({}))
    } else {
        match serde_json::from_str(arguments) {
            Ok(value) => sanitize_tool_arguments(value),
            Err(_) => Ok(json!({})),
        }
    }
}

pub(crate) fn sanitize_tool_request_id(id: &str, seen_ids: &mut HashSet<String>) -> anyhow::Result<String> {
    let id = strip_unicode_tags(id);
    if !seen_ids.insert(id.clone()) {
        return Err(anyhow!(
            "Responses tool calls contain duplicate ID after Unicode tag sanitization"
        ));
    }
    Ok(id)
}

pub fn responses_api_to_message(response: &ResponsesApiResponse) -> anyhow::Result<Message> {
    let mut content = Vec::new();
    let mut tool_request_ids = HashSet::new();

    for item in &response.output {
        match item {
            ResponseOutputItem::Reasoning { summary, .. } => {
                content.extend(reasoning_from_summary(summary));
            }
            ResponseOutputItem::Message {
                content: msg_content,
                ..
            } => {
                for block in msg_content {
                    match block {
                        ResponseContentBlock::OutputText { text, .. } => {
                            let text = sanitize_unicode_tags(text);
                            if !text.is_empty() {
                                content.push(MessageContentBlock::text(text));
                            }
                        }
                        ResponseContentBlock::Refusal { refusal } => {
                            let refusal = sanitize_unicode_tags(refusal);
                            if !refusal.is_empty() {
                                content.push(MessageContentBlock::text(refusal));
                            }
                        }
                        ResponseContentBlock::ReasoningText { text } => {
                            let text = sanitize_unicode_tags(text);
                            if !text.is_empty() {
                                content.push(MessageContentBlock::thinking(text, ""));
                            }
                        }
                        ResponseContentBlock::ToolCall { id, name, input } => {
                            let id = sanitize_tool_request_id(id, &mut tool_request_ids)?;
                            content.push(MessageContentBlock::tool_request(
                                id,
                                Ok(CallToolRequestParams::new(strip_unicode_tags(name))
                                    .with_arguments(object(sanitize_tool_arguments(
                                        input.clone(),
                                    )?))),
                            ));
                        }
                    }
                }
            }
            ResponseOutputItem::FunctionCall {
                id,
                call_id,
                name,
                arguments,
                ..
            } => {
                let request_id = call_id.clone().or_else(|| id.clone()).ok_or_else(|| {
                    anyhow!("Responses function_call output missing call_id and id")
                })?;
                let request_id = sanitize_tool_request_id(&request_id, &mut tool_request_ids)?;
                let parsed_args = parse_tool_arguments(arguments)?;

                content.push(MessageContentBlock::tool_request(
                    request_id,
                    Ok(CallToolRequestParams::new(strip_unicode_tags(name))
                        .with_arguments(object(parsed_args))),
                ));
            }
        }
    }

    let mut message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);

    message = message.with_id(response.id.clone());

    Ok(message)
}

pub fn get_responses_usage(response: &ResponsesApiResponse) -> Usage {
    response
        .usage
        .as_ref()
        .map_or_else(Usage::default, ResponseUsage::to_usage)
}
