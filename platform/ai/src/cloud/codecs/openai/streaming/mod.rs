pub(crate) mod parser;
pub(crate) mod types;

pub(crate) use parser::*;
pub(crate) use types::*;

use crate::conversation::message::{Message, MessageContentBlock, ProviderMetadata};
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::json::{parse_tool_arguments, truncation_error_message};
use crate::thinking::{ThinkFilter, GEMINI_THOUGHT_SIGNATURE_KEY};
use anyhow::anyhow;
use async_stream::try_stream;
use futures::Stream;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;

use super::response::extract_usage_with_output_tokens;

pub(crate) fn output_token_limit_tool_error(function_name: &str, id: &str) -> ErrorData {
    ErrorData {
        code: ErrorCode::INVALID_PARAMS,
        message: Cow::from(format!(
            "Tool arguments for {function_name} (id {id}) were truncated because the model reached its output token limit"
        )),
        data: None,
    }
}

fn output_token_limit_marker(id: Option<String>) -> Message {
    let mut message = Message::assistant();
    if let Some(id) = id {
        message = message.with_id(id);
    }
    message.metadata.output_token_limit_reached = true;
    message
}

pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    try_stream! {
        use futures::StreamExt;

        let mut accumulated_reasoning: Vec<Value> = Vec::new();
        let mut accumulated_reasoning_content = String::new();
        let mut think_filter = ThinkFilter::new();
        let mut saw_structured_reasoning = false;
        let mut yielded_reasoning_content_len = 0usize;
        let mut last_signature: Option<String> = None;
        // Buffer inline <think>...</think> content until we know whether structured
        // reasoning will arrive. Emitting it immediately and then receiving
        // reasoning_content in a later chunk would produce duplicated reasoning.
        let mut pending_inline_thinking = String::new();
        let mut last_seen_model: Option<String> = None;
        let mut last_response_id: Option<String> = None;
        let mut last_finish_reason: Option<String> = None;
        let mut output_token_limit_reached = false;
        let mut output_token_limit_metadata_emitted = false;
        let mut usage_emitted = false;

        'outer: while let Some(response) = stream.next().await {
            let response_str = response?;
            let line = strip_data_prefix(&response_str);

            if line.is_some_and(|l| l == "[DONE]") {
                break 'outer;
            }

            if line.is_none() || line.is_some_and(|l| l.is_empty()) {
                continue
            }

            let Some(chunk) = parse_streaming_chunk(
                line.ok_or_else(|| anyhow!("unexpected stream format"))?
            )? else {
                continue  // metadata-only frame
            };
            if let Some(model) = &chunk.model {
                last_seen_model = Some(model.clone());
            }
            if let Some(id) = &chunk.id {
                last_response_id = Some(id.clone());
            }

            if !chunk.choices.is_empty() {
                if let Some(details) = &chunk.choices[0].delta.reasoning_details {
                    accumulated_reasoning.extend(details.iter().cloned());
                }
                if let Some(rc) = chunk.choices[0].delta.reasoning_text() {
                    accumulated_reasoning_content.push_str(rc);
                    if !rc.is_empty() {
                        saw_structured_reasoning = true;
                        pending_inline_thinking.clear();
                    }
                }
            }

            if let Some(reason) = chunk.choices.first().and_then(|c| c.finish_reason.clone()) {
                last_finish_reason = Some(reason);
            }
            let mut usage = extract_usage_with_output_tokens(&chunk, last_seen_model.as_deref());
            if let Some(u) = usage.as_mut() {
                if let Some(reason) = &last_finish_reason {
                    u.finish_reasons = Some(vec![reason.clone()]);
                }
                if let Some(id) = &last_response_id {
                    u.response_id = Some(id.clone());
                }
            }
            output_token_limit_reached |= last_finish_reason.as_deref() == Some("length");

            if chunk.choices.is_empty() {
                usage_emitted |= usage.is_some();
                yield (None, usage)
            } else if chunk.choices[0].delta.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty()) {
                let mut tool_call_data: ToolCallData = HashMap::new();

                if let Some(tool_calls) = &chunk.choices[0].delta.tool_calls {
                    for (position, tool_call) in tool_calls.iter().enumerate() {
                        if let (Some(id), Some(name)) = (&tool_call.id, &tool_call.function.name) {
                            let index = tool_call.index.unwrap_or(position as i32);
                            tool_call_data.insert(index, (id.clone(), name.clone(), tool_call.function.arguments.clone(), tool_call.extra.clone()));
                        }
                    }
                }

                let is_complete = matches!(
                    chunk.choices[0].finish_reason.as_deref(),
                    Some("tool_calls" | "length")
                );

                if !is_complete {
                    let mut done = false;
                    while !done {
                        if let Some(response_chunk) = stream.next().await {
                            let response_str = response_chunk?;
                            if let Some(line) = strip_data_prefix(&response_str) {
                                if line == "[DONE]" {
                                    break 'outer;
                                }

                                // A metadata frame here must NOT fall through to the
                                // empty-choices branch below, which ends accumulation and
                                // would truncate this tool call's arguments.
                                let Some(tool_chunk) = parse_streaming_chunk(line)? else {
                                    continue
                                };
                                if let Some(model) = &tool_chunk.model {
                                    last_seen_model = Some(model.clone());
                                }
                                if let Some(id) = &tool_chunk.id {
                                    last_response_id = Some(id.clone());
                                }
                                if let Some(reason) = tool_chunk.choices.first().and_then(|c| c.finish_reason.clone()) {
                                    last_finish_reason = Some(reason);
                                }

                                if let Some(mut chunk_usage) = extract_usage_with_output_tokens(&tool_chunk, last_seen_model.as_deref()) {
                                    if let Some(reason) = &last_finish_reason {
                                        chunk_usage.finish_reasons = Some(vec![reason.clone()]);
                                    }
                                    if let Some(id) = &last_response_id {
                                        chunk_usage.response_id = Some(id.clone());
                                    }
                                    usage = Some(chunk_usage);
                                }

                                if !tool_chunk.choices.is_empty() {
                                    output_token_limit_reached |=
                                        tool_chunk.choices[0].finish_reason.as_deref()
                                            == Some("length");

                                    if let Some(details) = &tool_chunk.choices[0].delta.reasoning_details {
                                        accumulated_reasoning.extend(details.iter().cloned());
                                    }
                                    if let Some(rc) = tool_chunk.choices[0].delta.reasoning_text() {
                                        accumulated_reasoning_content.push_str(rc);
                                        if !rc.is_empty() {
                                            saw_structured_reasoning = true;
                                            pending_inline_thinking.clear();
                                        }
                                    }
                                    if let Some(delta_tool_calls) = &tool_chunk.choices[0].delta.tool_calls {
                                        for delta_call in delta_tool_calls {
                                            if let Some(index) = delta_call.index {
                                                if let Some((_, ref mut stored_name, ref mut args, ref mut extra)) = tool_call_data.get_mut(&index) {
                                                    if let Some(new_name) = &delta_call.function.name {
                                                        if !new_name.is_empty() {
                                                            *stored_name = new_name.clone();
                                                        }
                                                    }
                                                    args.push_str(&delta_call.function.arguments);
                                                    if extra.is_none() && delta_call.extra.is_some() {
                                                        *extra = delta_call.extra.clone();
                                                    } else if let (Some(existing), Some(new_extra)) = (extra.as_mut(), &delta_call.extra) {
                                                        for (key, value) in new_extra {
                                                            existing.entry(key.clone()).or_insert(value.clone());
                                                        }
                                                    }
                                                } else if let (Some(id), Some(name)) = (&delta_call.id, &delta_call.function.name) {
                                                    tool_call_data.insert(index, (id.clone(), name.clone(), delta_call.function.arguments.clone(), delta_call.extra.clone()));
                                                }
                                            }
                                        }
                                    }
                                    if tool_chunk.choices[0].finish_reason.is_some() {
                                        done = true;
                                    }
                                } else {
                                    done = true;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }

                let _metadata: Option<ProviderMetadata> = if !accumulated_reasoning.is_empty() {
                    let mut map = ProviderMetadata::new();
                    map.insert("reasoning_details".to_string(), json!(accumulated_reasoning));
                    Some(map)
                } else {
                    None
                };

                let filtered = think_filter.push("");
                let mut flush_thinking = String::new();
                if !saw_structured_reasoning {
                    flush_thinking.push_str(&pending_inline_thinking);
                    flush_thinking.push_str(&filtered.thinking);
                }
                pending_inline_thinking.clear();
                if !filtered.content.is_empty() || !flush_thinking.is_empty() {
                    let mut filtered_contents = Vec::new();
                    if !filtered.content.is_empty() {
                        filtered_contents.push(MessageContentBlock::text(filtered.content));
                    }
                    if !flush_thinking.is_empty() {
                        filtered_contents.push(MessageContentBlock::thinking(flush_thinking, ""));
                    }

                    if !filtered_contents.is_empty() {
                        let mut msg = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            filtered_contents,
                        );

                        if let Some(id) = chunk.id.clone() {
                            msg = msg.with_id(id);
                        }

                        yield (Some(msg), None);
                    }
                }

                let mut contents = Vec::new();
                if yielded_reasoning_content_len < accumulated_reasoning_content.len() {
                    if let Some(unyielded_reasoning) =
                        accumulated_reasoning_content.get(yielded_reasoning_content_len..)
                    {
                        if !unyielded_reasoning.is_empty() {
                            contents.push(MessageContentBlock::thinking(unyielded_reasoning, ""));
                        }
                    }
                }
                accumulated_reasoning_content.clear();
                yielded_reasoning_content_len = 0;
                let mut sorted_indices: Vec<_> = tool_call_data.keys().cloned().collect();
                sorted_indices.sort();

                for index in sorted_indices {
                    if let Some((id, function_name, arguments, extra_fields)) = tool_call_data.get(&index) {
                        let metadata = if let Some(sig) = &last_signature {
                            let mut combined = extra_fields.clone().unwrap_or_default();
                            combined.insert(
                                GEMINI_THOUGHT_SIGNATURE_KEY.to_string(),
                                json!(sig)
                            );
                            Some(combined)
                        } else {
                            extra_fields.as_ref().filter(|m| !m.is_empty()).cloned()
                        };

                        let content = if output_token_limit_reached {
                            MessageContentBlock::tool_request_with_provider_index(
                                id.clone(),
                                Err(output_token_limit_tool_error(function_name, id)),
                                metadata.as_ref(),
                                index,
                            )
                        } else if arguments.is_empty() {
                            MessageContentBlock::tool_request_with_provider_index(
                                id.clone(),
                                Ok(CallToolRequestParams::new(function_name.clone()).with_arguments(object(json!({})))),
                                metadata.as_ref(),
                                index,
                            )
                        } else {
                            match parse_tool_arguments(arguments) {
                                Some(params) if params.is_object() => MessageContentBlock::tool_request_with_provider_index(
                                    id.clone(),
                                    Ok(CallToolRequestParams::new(function_name.clone()).with_arguments(object(params))),
                                    metadata.as_ref(),
                                    index,
                                ),
                                // Valid JSON but NOT an object (a bare array/string/number).
                                // Surface a tool error so the model retries instead of
                                // crashing the run (rmcp's `object()` debug-asserts on
                                // non-objects). Mirrors the non-streaming decoder.
                                Some(other) => {
                                    let error = ErrorData {
                                        code: ErrorCode::INVALID_PARAMS,
                                        message: Cow::from(format!(
                                            "Tool arguments for {} (id {}) must be a JSON object, got {}. Raw arguments: '{}'",
                                            function_name, id, describe_json_value(&other), arguments
                                        )),
                                        data: None,
                                    };
                                    MessageContentBlock::tool_request_with_provider_index(id.clone(), Err(error), metadata.as_ref(), index)
                                }
                                None => {
                                    let message_text = truncation_error_message(arguments)
                                        .unwrap_or_else(|| {
                                            format!("Could not interpret tool use parameters for id {id}")
                                        });
                                    let error = ErrorData {
                                        code: ErrorCode::INVALID_PARAMS,
                                        message: Cow::from(message_text),
                                        data: None,
                                    };
                                    MessageContentBlock::tool_request_with_provider_index(id.clone(), Err(error), metadata.as_ref(), index)
                                }
                            }
                        };

                        contents.push(content);
                    }
                }

                let mut msg = Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    contents,
                );

                // Add ID if present
                if let Some(id) = chunk.id {
                    msg = msg.with_id(id);
                }
                msg.metadata.output_token_limit_reached = output_token_limit_reached;
                output_token_limit_metadata_emitted |= output_token_limit_reached;

                usage_emitted |= usage.is_some();
                yield (
                    Some(msg),
                    usage,
                )
            } else if chunk.choices[0].delta.content.is_some() || chunk.choices[0].delta.reasoning_text().is_some() {
                let mut content = Vec::new();

                if let Some(reasoning) = chunk.choices[0].delta.reasoning_text() {
                    let signature = last_signature.as_deref().unwrap_or("");
                    content.push(MessageContentBlock::thinking(reasoning, signature));
                    yielded_reasoning_content_len = accumulated_reasoning_content.len();
                }

                let (text_content, thought_signature) = extract_content_and_signature(chunk.choices[0].delta.content.as_ref());

                if let Some(sig) = thought_signature {
                    last_signature = Some(sig);
                }

                if let Some(text) = text_content {
                    let filtered = think_filter.push(&text);

                    if !saw_structured_reasoning && !filtered.thinking.is_empty() {
                        pending_inline_thinking.push_str(&filtered.thinking);
                    }

                    if !filtered.content.is_empty() {
                        content.push(MessageContentBlock::text(filtered.content));
                    }
                }

                if !content.is_empty() {
                    let mut msg = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        content,
                    );

                    if let Some(id) = chunk.id {
                        msg = msg.with_id(id);
                    }

                    let final_usage = if chunk.choices[0].finish_reason.is_some() {
                        usage
                    } else {
                        None
                    };
                    usage_emitted |= final_usage.is_some();
                    yield (Some(msg), final_usage)
                } else if usage.is_some() {
                    usage_emitted = true;
                    yield (None, usage)
                }
            } else if usage.is_some() {
                usage_emitted = true;
                yield (None, usage)
            }
        }

        let filtered = think_filter.finish();
        let mut trailing_thinking = String::new();
        if !saw_structured_reasoning {
            trailing_thinking.push_str(&pending_inline_thinking);
            trailing_thinking.push_str(&filtered.thinking);
        }
        pending_inline_thinking.clear();

        if !filtered.content.is_empty() || !trailing_thinking.is_empty() {
            let mut content = Vec::new();

            if !filtered.content.is_empty() {
                content.push(MessageContentBlock::text(filtered.content));
            }

            if !trailing_thinking.is_empty() {
                content.push(MessageContentBlock::thinking(trailing_thinking, ""));
            }

            let mut message = Message::new(
                Role::Assistant,
                chrono::Utc::now().timestamp(),
                content,
            );
            if let Some(id) = last_response_id.clone() {
                message = message.with_id(id);
            }
            message.metadata.output_token_limit_reached =
                output_token_limit_reached && !output_token_limit_metadata_emitted;
            output_token_limit_metadata_emitted |= message.metadata.output_token_limit_reached;

            yield (Some(message), None)
        }

        if output_token_limit_reached && !output_token_limit_metadata_emitted {
            yield (Some(output_token_limit_marker(last_response_id.clone())), None)
        }

        if !usage_emitted && (last_response_id.is_some() || last_finish_reason.is_some()) {
            let mut usage = ProviderUsage::new(
                last_seen_model.unwrap_or_else(|| "unknown".to_string()),
                Usage::default(),
            );
            usage.response_id = last_response_id;
            usage.finish_reasons = last_finish_reason.map(|reason| vec![reason]);
            yield (None, Some(usage))
        }
    }
}

