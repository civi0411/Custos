use super::response::*;
use super::types::*;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;
use crate::utils::{sanitize_unicode_tags, strip_unicode_tags};
use anyhow::anyhow;
use async_stream::try_stream;
use futures::Stream;
use rmcp::model::{object, CallToolRequestParams, Role};
use std::collections::HashSet;

pub(crate) fn process_streaming_output_items(
    output_items: Vec<ResponseOutputItemInfo>,
    is_text_response: bool,
) -> anyhow::Result<Vec<MessageContentBlock>> {
    let mut content = Vec::new();
    let mut tool_request_ids = HashSet::new();

    for item in output_items {
        match item {
            ResponseOutputItemInfo::Reasoning { summary, .. } => {
                content.extend(reasoning_from_summary(&summary));
            }
            ResponseOutputItemInfo::Message { content: parts, .. } => {
                for part in parts {
                    match part {
                        ContentBlockPart::OutputText { text, .. } => {
                            let text = sanitize_unicode_tags(&text);
                            if !text.is_empty() && !is_text_response {
                                content.push(MessageContentBlock::text(text));
                            }
                        }
                        ContentBlockPart::Refusal { refusal } => {
                            let refusal = sanitize_unicode_tags(&refusal);
                            if !refusal.is_empty() && !is_text_response {
                                content.push(MessageContentBlock::text(refusal));
                            }
                        }
                        ContentBlockPart::ReasoningText { text } => {
                            let text = sanitize_unicode_tags(&text);
                            if !text.is_empty() {
                                content.push(MessageContentBlock::thinking(text, ""));
                            }
                        }
                        ContentBlockPart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => {
                            let id = sanitize_tool_request_id(&id, &mut tool_request_ids)?;
                            let parsed_args = parse_tool_arguments(&arguments)?;

                            content.push(MessageContentBlock::tool_request(
                                id,
                                Ok(CallToolRequestParams::new(strip_unicode_tags(&name))
                                    .with_arguments(object(parsed_args))),
                            ));
                        }
                    }
                }
            }
            ResponseOutputItemInfo::FunctionCall {
                id,
                call_id,
                name,
                arguments,
                ..
            } => {
                let request_id = call_id.or(id).ok_or_else(|| {
                    anyhow!("Responses function_call output missing call_id and id")
                })?;
                let request_id = sanitize_tool_request_id(&request_id, &mut tool_request_ids)?;
                let parsed_args = parse_tool_arguments(&arguments)?;

                content.push(MessageContentBlock::tool_request(
                    request_id,
                    Ok(CallToolRequestParams::new(strip_unicode_tags(&name))
                        .with_arguments(object(parsed_args))),
                ));
            }
        }
    }

    Ok(content)
}

pub(crate) fn output_token_limit_marker(id: Option<String>) -> Message {
    let mut message = Message::assistant();
    if let Some(id) = id {
        message = message.with_id(id);
    }
    message.metadata.output_token_limit_reached = true;
    message
}

/// Parse a line per the SSE grammar and return its field name:
/// - `field: value` / `field:value` -> `Some(field)`
/// - `field` (no colon, empty value) -> `Some(field)`
/// - `: comment` -> `Some("")`
///
/// Returns `None` when the line does not look like an SSE field (e.g. a
/// bare JSON payload such as `{"type": ...}`), so callers can fall back
/// to parsing it as JSON.
pub(crate) fn sse_field_name(line: &str) -> Option<&str> {
    let field = line.split_once(':').map_or(line, |(name, _)| name);
    if field.is_empty() {
        return Some("");
    }
    let field_like = !field.contains(char::is_whitespace)
        && field
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
    field_like.then_some(field)
}

pub fn responses_api_to_streaming_message<S>(
    mut stream: S,
) -> impl Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    try_stream! {
        use futures::StreamExt;

        let mut accumulated_text = String::new();
        let mut response_id: Option<String> = None;
        let mut model_name: Option<String> = None;
        let mut final_usage: Option<ProviderUsage> = None;
        let mut output_items: Vec<ResponseOutputItemInfo> = Vec::new();
        let mut is_text_response = false;
        let mut output_token_limit_reached = false;

        'outer: while let Some(response) = stream.next().await {
            let response_str = response?;

            // Skip empty lines
            if response_str.trim().is_empty() {
                continue;
            }
            if response_str.starts_with(':') {
                continue;
            }

            // Parse SSE format: "event: <type>\ndata: <json>"
            // For now, we only care about the data line
            // SSE spec allows both "data: value" and "data:value" (space after colon is optional)
            let data_line = if response_str.starts_with("data: ") {
                response_str.strip_prefix("data: ").unwrap()
            } else if response_str.starts_with("data:") {
                response_str.strip_prefix("data:").unwrap()
            } else if sse_field_name(&response_str).is_some_and(|f| f != "data") {
                // Skip payload-free SSE fields: event, id, retry, comments,
                // colon-less fields with empty values, and unknown extension
                // fields — the SSE spec requires all of these to be ignored.
                continue;
            } else {
                // Try to parse as-is when there's no prefix (bare JSON frames)
                &response_str
            };

            if data_line == "[DONE]" {
                break 'outer;
            }

            let Some(event) = parse_responses_stream_event(data_line)? else {
                continue;
            };

            match event {
                ResponsesStreamEvent::ResponseCreated { response, .. } |
                ResponsesStreamEvent::ResponseInProgress { response, .. } => {
                    response_id = Some(response.id);
                    model_name = Some(response.model);
                }

                ResponsesStreamEvent::OutputTextDelta { delta, .. } => {
                    is_text_response = true;
                    let delta = strip_unicode_tags(&delta);
                    if !delta.is_empty() {
                        accumulated_text.push_str(&delta);

                        // Yield incremental text updates for true streaming
                        let mut msg = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![MessageContentBlock::text(&delta)],
                        );

                        // Add ID so desktop client knows these deltas are part of the same message
                        if let Some(id) = &response_id {
                            msg = msg.with_id(id.clone());
                        }

                        yield (Some(msg), None);
                    }
                }

                ResponsesStreamEvent::OutputItemDone { item, .. } => {
                    output_items.push(item);
                }

                ResponsesStreamEvent::OutputTextDone { .. } => {
                    // Text is already complete from deltas, this is just a summary event
                }

                ResponsesStreamEvent::ResponseCompleted { response, .. } => {
                    let model = model_name.as_ref().unwrap_or(&response.model);
                    let usage = response.usage.as_ref().map_or_else(
                        Usage::default,
                        ResponseUsage::to_usage,
                    );
                    let mut pu = ProviderUsage::new(model.clone(), usage);
                    pu.finish_reasons = Some(vec![response.status.clone()]);
                    pu.response_id = Some(response.id.clone());
                    final_usage = Some(pu);

                    // For complete output, use the response output items
                    if !response.output.is_empty() {
                        output_items = response.output;
                    }

                    break 'outer;
                }

                ResponsesStreamEvent::ResponseIncomplete { response, .. } => {
                    let model = model_name.as_ref().unwrap_or(&response.model);
                    let usage = response.usage.as_ref().map_or_else(
                        Usage::default,
                        ResponseUsage::to_usage,
                    );
                    let mut pu = ProviderUsage::new(model.clone(), usage);
                    pu.finish_reasons = Some(vec![response
                        .incomplete_details
                        .as_ref()
                        .and_then(|details| details.reason.clone())
                        .unwrap_or_else(|| response.status.clone())]);
                    pu.response_id = Some(response.id.clone());
                    final_usage = Some(pu);
                    response_id = Some(response.id.clone());
                    output_token_limit_reached = response_reached_output_token_limit(
                        &response.status,
                        response.incomplete_details.as_ref(),
                    );

                    if !response.output.is_empty() {
                        output_items = response
                            .output
                            .into_iter()
                            .filter(|item| match item {
                                ResponseOutputItemInfo::FunctionCall { status, .. } => {
                                    status.as_deref() == Some("completed")
                                }
                                _ => true,
                            })
                            .collect();
                    }

                    break 'outer;
                }

                ResponsesStreamEvent::FunctionCallArgumentsDelta { .. } => {
                    // Function call arguments are being streamed, but we'll get the complete
                    // arguments in the OutputItemDone event, so we can ignore deltas for now
                }

                ResponsesStreamEvent::FunctionCallArgumentsDone { .. } => {
                    // Arguments are complete, will be in the OutputItemDone event
                }

                ResponsesStreamEvent::RefusalDelta { delta, .. } => {
                    is_text_response = true;
                    let delta = strip_unicode_tags(&delta);
                    if !delta.is_empty() {
                        accumulated_text.push_str(&delta);

                        let mut msg = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![MessageContentBlock::text(&delta)],
                        );

                        if let Some(id) = &response_id {
                            msg = msg.with_id(id.clone());
                        }

                        yield (Some(msg), None);
                    }
                }

                ResponsesStreamEvent::RefusalDone { .. } => {
                    // Refusal text already streamed via deltas
                }

                ResponsesStreamEvent::ResponseFailed { error, .. } => {
                    Err::<(), ProviderError>(ProviderError::RequestFailed(format!(
                        "Responses API failed: {:?}",
                        error
                    )))?;
                }

                ResponsesStreamEvent::Error { error } => {
                    Err::<(), ProviderError>(ProviderError::RequestFailed(format!(
                        "Responses API error: {:?}",
                        error
                    )))?;
                }

                _ => {
                    // Ignore other event types (OutputItemAdded, ContentBlockPartAdded, ContentBlockPartDone)
                }
            }
        }

        // Process final output items and yield usage data
        let content = process_streaming_output_items(output_items, is_text_response)?;

        if !content.is_empty() {
            let mut message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);
            if let Some(id) = response_id {
                message = message.with_id(id);
            }
            message.metadata.output_token_limit_reached = output_token_limit_reached;
            yield (Some(message), final_usage);
        } else if output_token_limit_reached {
            yield (Some(output_token_limit_marker(response_id)), final_usage);
        } else if let Some(usage) = final_usage {
            yield (None, Some(usage));
        }
    }
}
