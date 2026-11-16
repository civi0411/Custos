use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::ProviderUsage;
use crate::errors::ProviderError;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde_json::{json, Map, Value};
use super::options::*;
use super::response::*;

pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl futures::Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: futures::Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    use async_stream::try_stream;
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct StreamingEvent {
        #[serde(rename = "type")]
        event_type: String,
        #[serde(flatten)]
        data: Value,
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "snake_case")]
    #[allow(clippy::enum_variant_names)]
    enum ContentBlockDelta {
        TextDelta { text: String },
        InputJsonDelta { partial_json: String },
        ThinkingDelta { thinking: String },
        SignatureDelta { signature: String },
    }

    struct ThinkingState {
        text: String,
        signature: String,
    }

    fn block_index(event_data: &Value) -> Option<i32> {
        event_data
            .get("index")
            .and_then(|v| v.as_i64())
            .map(|index| index as i32)
    }

    try_stream! {
        struct StreamingToolCall {
            id: String,
            name: String,
            arguments: String,
        }

        let mut accumulated_tool_calls: std::collections::HashMap<i32, StreamingToolCall> = std::collections::HashMap::new();
        let mut final_usage: Option<ProviderUsage> = None;
        let mut message_id: Option<String> = None;
        let mut thinking: Option<ThinkingState> = None;
        let mut stop_reason: Option<String> = None;
        let mut additional_data: Option<Map<String, Value>> = None;

        while let Some(line_result) = stream.next().await {
            let line = line_result?;

            // Skip empty lines and non-data lines
            // Note: SSE spec allows both "data: value" and "data:value" (space is optional)
            if line.trim().is_empty() || !line.starts_with("data:") {
                continue;
            }

            let data_part = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")).unwrap_or(&line);

            // Handle end of stream
            if data_part.trim() == "[DONE]" {
                break;
            }

            // Parse the JSON event
            let event: StreamingEvent = match serde_json::from_str(data_part) {
                Ok(event) => event,
                Err(e) => {
                    tracing::debug!("Failed to parse streaming event: {} - Line: {}", e, data_part);
                    continue;
                }
            };

            match event.event_type.as_str() {
                EVENT_MESSAGE_START => {
                    if let Some(message_data) = event.data.get("message") {
                        additional_data = get_additional_data(message_data);
                        if let Some(transformations) = input_transformations(message_data) {
                            additional_data
                                .get_or_insert_with(Map::new)
                                .insert(INPUT_TRANSFORMATIONS_FIELD.to_string(), transformations);
                        }
                        if let Some(id) = message_data.get("id").and_then(|v| v.as_str()) {
                            message_id = Some(id.to_string());
                        }

                        if let Some(usage_data) = message_data.get("usage") {
                            let usage = get_usage(usage_data).unwrap_or_default();
                            let model = message_data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(provider_usage_with_cost(model, usage, usage_data, None));
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_START => {
                    if let Some(content_block) = event.data.get("content_block") {
                        match content_block.get(TYPE_FIELD).and_then(|v| v.as_str()) {
                            Some(TOOL_USE_TYPE) => {
                                if let (Some(index), Some(id), Some(name)) = (
                                    block_index(&event.data),
                                    content_block.get("id").and_then(|v| v.as_str()),
                                    content_block.get("name").and_then(|v| v.as_str()),
                                ) {
                                    accumulated_tool_calls.insert(index, StreamingToolCall {
                                        id: id.to_string(),
                                        name: name.to_string(),
                                        arguments: String::new(),
                                    });
                                }
                            }
                            Some(THINKING_TYPE) => {
                                thinking = Some(ThinkingState {
                                    text: content_block
                                        .get(THINKING_TYPE)
                                        .and_then(|t| t.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    signature: content_block
                                        .get(SIGNATURE_FIELD)
                                        .and_then(|s| s.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                });
                            }
                            Some(REDACTED_THINKING_TYPE) => {
                                if let Some(data) = content_block.get(DATA_FIELD).and_then(|d| d.as_str()) {
                                    let mut message = Message::assistant()
                                        .with_redacted_thinking(data);
                                    message.id = message_id.clone();
                                    yield (Some(message), None);
                                } else {
                                    tracing::warn!("redacted_thinking block missing '{}' field", DATA_FIELD);
                                }
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_DELTA => {
                    if let Some(delta) = event.data.get("delta") {
                        match serde_json::from_value::<ContentBlockDelta>(delta.clone()) {
                            Ok(ContentBlockDelta::TextDelta { text }) => {
                                let mut message = Message::assistant().with_text(&text);
                                message.id = message_id.clone();
                                yield (Some(message), None);
                            }
                            Ok(ContentBlockDelta::InputJsonDelta { partial_json }) => {
                                if let Some(call) = block_index(&event.data)
                                    .and_then(|index| accumulated_tool_calls.get_mut(&index))
                                {
                                    call.arguments.push_str(&partial_json);
                                }
                            }
                            Ok(ContentBlockDelta::ThinkingDelta { thinking: t }) => {
                                if let Some(ref mut state) = thinking {
                                    state.text.push_str(&t);
                                }
                            }
                            Ok(ContentBlockDelta::SignatureDelta { signature: s }) => {
                                if let Some(ref mut state) = thinking {
                                    state.signature.push_str(&s);
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Unknown content_block_delta type: {}", e);
                            }
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_STOP => {
                    if let Some(state) = thinking.take() {
                        // Omitted thinking arrives as an empty string with a signature and must still be replayed.
                        if !state.text.is_empty() || !state.signature.is_empty() {
                            let mut message = Message::assistant()
                                .with_thinking(state.text, state.signature);
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    if let Some(index) = block_index(&event.data) {
                        if let Some(call) = accumulated_tool_calls.remove(&index) {
                            let StreamingToolCall { id, name, arguments } = call;
                            let parsed_args = if arguments.is_empty() {
                                json!({})
                            } else {
                                match crate::json::parse_tool_arguments(&arguments) {
                                    Some(parsed) => parsed,
                                    None => {
                                        let message_text = crate::json::truncation_error_message(&arguments)
                                            .unwrap_or_else(|| {
                                                format!("Could not parse tool arguments: {arguments}")
                                            });
                                        let error = ErrorData::new(
                                            ErrorCode::INVALID_PARAMS,
                                            message_text,
                                            None,
                                        );
                                        let mut message = Message::new(
                                            Role::Assistant,
                                            chrono::Utc::now().timestamp(),
                                            vec![MessageContentBlock::tool_request_with_provider_index(id, Err(error), None, index)],
                                        );
                                        message.id = message_id.clone();
                                        yield (Some(message), None);
                                        continue;
                                    }
                                }
                            };

                            let tool_call = CallToolRequestParams::new(name).with_arguments(object(parsed_args));

                            let mut message = Message::new(
                                rmcp::model::Role::Assistant,
                                chrono::Utc::now().timestamp(),
                                vec![MessageContentBlock::tool_request_with_provider_index(id, Ok(tool_call), None, index)],
                            );
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    continue;
                }
                EVENT_MESSAGE_DELTA => {
                    if let Some(usage_data) = event.data.get("usage") {
                        let delta_usage = get_usage(usage_data).unwrap_or_default();

                        if let Some(existing_usage) = &final_usage {
                            let merged_usage = merge_delta_usage(&existing_usage.usage, &delta_usage, usage_data);
                            final_usage = Some(provider_usage_with_cost(
                                existing_usage.model.clone(),
                                merged_usage,
                                usage_data,
                                existing_usage.cost,
                            ));
                        } else {
                            let model = event.data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(provider_usage_with_cost(model, delta_usage, usage_data, None));
                        }
                    }
                    if let Some(delta) = event.data.get("delta") {
                        let stop_details = delta.get("stop_details").filter(|d| !d.is_null());
                        if stop_reason.is_none() {
                            if let Some(sr) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                                stop_reason = Some(sr.to_string());
                            }
                        }
                        if delta.get("stop_reason").and_then(|v| v.as_str()) == Some(STOP_REASON_REFUSAL) {
                            let str_field = |key: &str| stop_details
                                .and_then(|d| d.get(key))
                                .and_then(|v| v.as_str())
                                .map(str::to_string);
                            let details = str_field("explanation")
                                .or_else(|| stop_details.map(|d| d.to_string()))
                                .unwrap_or_else(|| REFUSAL_FALLBACK_DETAILS.to_string());
                            let category = str_field("category");
                            // The refusal delta carries the request's usage;
                            // flush it so refused turns are still accounted.
                            if let Some(mut usage) = final_usage.take() {
                                usage.finish_reasons = Some(vec![STOP_REASON_REFUSAL.to_string()]);
                                usage.response_id = message_id.clone();
                                usage.additional_data = additional_data.clone();
                                yield (None, Some(usage));
                            }
                            Err(ProviderError::Refusal { details, category })?;
                        } else if let Some(details) = stop_details {
                            // No specific handling for these stop details yet —
                            // forward them rather than silently dropping the turn.
                            let mut message = Message::assistant().with_text(format!(
                                "The provider ended the response with: {details}"
                            ));
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    continue;
                }
                EVENT_MESSAGE_STOP => {
                    if let Some(usage_data) = event.data.get("usage") {
                        let usage = get_usage(usage_data).unwrap_or_default();
                        let model = event.data.get("model")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let fallback_cost = final_usage.as_ref().and_then(|u| u.cost);
                        final_usage = Some(provider_usage_with_cost(model, usage, usage_data, fallback_cost));
                    }
                    break;
                }
                _ => {
                    // Unknown event type, log and continue
                    tracing::debug!("Unknown streaming event type: {}", event.event_type);
                    continue;
                }
            }
        }

        // A tool_use block left open at stream end never received its
        // content_block_stop, so its args are truncated rather than complete.
        if !accumulated_tool_calls.is_empty() {
            let truncated_by_limit = stop_reason.as_deref() == Some("max_tokens");
            let mut indices: Vec<i32> = accumulated_tool_calls.keys().copied().collect();
            indices.sort();
            for index in indices {
                if let Some(StreamingToolCall { id, arguments: args, .. }) = accumulated_tool_calls.remove(&index) {
                    let guidance = if truncated_by_limit {
                        "The model's response was truncated — it hit the output token limit while generating this tool call. \
                         Try increasing max_tokens for this provider or breaking the task into smaller steps."
                    } else {
                        "A tool call was not completed before the stream ended. \
                         Try resending your message or breaking the task into smaller steps."
                    };
                    let snippet_len = args.chars().count();
                    let tail: String = args.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect();
                    let message_text = format!(
                        "{guidance}\nReceived {snippet_len} characters of arguments; cut off at: …{tail}"
                    );
                    let error = ErrorData::new(ErrorCode::INVALID_PARAMS, message_text, None);
                    let mut message = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        vec![MessageContentBlock::tool_request_with_provider_index(id, Err(error), None, index)],
                    );
                    message.id = message_id.clone();
                    yield (Some(message), None);
                }
            }
        }

        if stop_reason.as_deref() == Some("max_tokens") {
            let mut message = Message::assistant();
            message.id = message_id.clone();
            message.metadata.output_token_limit_reached = true;
            yield (Some(message), None);
        }

        if let Some(mut usage) = final_usage {
            if let Some(reason) = stop_reason {
                usage.finish_reasons = Some(vec![reason]);
            }
            if let Some(id) = message_id {
                usage.response_id = Some(id);
            }
            usage.additional_data = additional_data;
            yield (None, Some(usage));
        }
    }
}
