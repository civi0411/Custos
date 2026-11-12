use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::{CostSource, ProviderUsage, Usage};
use crate::json::{parse_tool_arguments, truncation_error_message};
use crate::thinking::split_think_blocks;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde_json::Value;
use std::borrow::Cow;
use super::streaming::types::{describe_json_value, StreamingChunk};
use super::streaming::output_token_limit_tool_error;

pub fn record_response_metadata(usage: &mut ProviderUsage, response: &Value) {
    usage.response_id = response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);

    let finish_reasons = response
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|choice| choice.get("finish_reason").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !finish_reasons.is_empty() {
        usage.finish_reasons = Some(finish_reasons);
    }
}

/// Convert OpenAI's API response to internal Message format
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let output_token_limit_reached = response
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
        == Some("length");

    let Some(original) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
    else {
        if let Some(error) = response.get("error") {
            let error_message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("API error: {}", error_message));
        }
        return Err(anyhow::anyhow!(
            "No message in API response. This may indicate a quota limit or other restriction."
        ));
    };

    let mut content = Vec::new();

    // Capture reasoning content if present (DeepSeek uses "reasoning_content", vLLM uses "reasoning")
    let reasoning_value = original
        .get("reasoning_content")
        .or_else(|| original.get("reasoning"));
    let mut has_structured_thinking = false;
    if let Some(reasoning_content) = reasoning_value {
        if let Some(reasoning_str) = reasoning_content.as_str() {
            if !reasoning_str.is_empty() {
                has_structured_thinking = true;
                content.push(MessageContentBlock::thinking(reasoning_str, ""));
            }
        }
    }

    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            let (cleaned, inline_thinking) = split_think_blocks(text_str);

            if !has_structured_thinking && !inline_thinking.is_empty() {
                content.push(MessageContentBlock::thinking(inline_thinking, ""));
            }

            if !cleaned.is_empty() {
                content.push(MessageContentBlock::text(cleaned));
            }
        }
    }

    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // Get the raw arguments string from the LLM.
                let arguments_str = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // If arguments_str is empty, default to an empty JSON object string.
                let arguments_str = if arguments_str.is_empty() {
                    "{}".to_string()
                } else {
                    arguments_str
                };

                let standard_fields = ["id", "function", "type", "index"];
                let metadata: Option<serde_json::Map<String, Value>> = tool_call
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter(|(k, _)| !standard_fields.contains(&k.as_str()))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .filter(|m: &serde_json::Map<String, Value>| !m.is_empty());

                if output_token_limit_reached {
                    let error = output_token_limit_tool_error(&function_name, &id);
                    content.push(MessageContentBlock::tool_request_with_metadata(
                        id,
                        Err(error),
                        metadata.as_ref(),
                    ));
                    continue;
                }

                if function_name.is_empty() {
                    let error = ErrorData {
                        code: ErrorCode::INVALID_REQUEST,
                        message: Cow::from(
                            "The provided function name was empty; a tool call must name a tool"
                                .to_string(),
                        ),
                        data: None,
                    };
                    content.push(MessageContentBlock::tool_request_with_metadata(
                        id,
                        Err(error),
                        metadata.as_ref(),
                    ));
                    continue;
                }
                match parse_tool_arguments(&arguments_str) {
                    Some(params) if params.is_object() => {
                        content.push(MessageContentBlock::tool_request_with_metadata(
                            id,
                            Ok(CallToolRequestParams::new(function_name)
                                .with_arguments(object(params))),
                            metadata.as_ref(),
                        ));
                    }
                    Some(other) => {
                        let error = ErrorData {
                            code: ErrorCode::INVALID_PARAMS,
                            message: Cow::from(format!(
                                "Tool arguments for {} (id {}) must be a JSON object, got {}. Raw arguments: '{}'",
                                function_name,
                                id,
                                describe_json_value(&other),
                                arguments_str
                            )),
                            data: None,
                        };
                        content.push(MessageContentBlock::tool_request_with_metadata(
                            id,
                            Err(error),
                            metadata.as_ref(),
                        ));
                    }
                    None => {
                        let message_text =
                            truncation_error_message(&arguments_str).unwrap_or_else(|| {
                                format!("Could not interpret tool use parameters for id {id}")
                            });
                        let error = ErrorData {
                            code: ErrorCode::INVALID_PARAMS,
                            message: Cow::from(message_text),
                            data: None,
                        };
                        content.push(MessageContentBlock::tool_request_with_metadata(
                            id,
                            Err(error),
                            metadata.as_ref(),
                        ));
                    }
                }
            }
        }
    }

    let mut message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);
    message.metadata.output_token_limit_reached = output_token_limit_reached;
    Ok(message)
}

pub fn get_usage(usage: &Value) -> Usage {
    let usage = usage
        .get("usage")
        .filter(|nested| nested.is_object())
        .unwrap_or(usage);

    // Try standard OpenAI fields first, then fall back to Ollama-native fields
    // (prompt_eval_count / eval_count) for compatibility with older Ollama builds
    // that don't translate to OpenAI field names.
    // Parse the value before falling back so that present-but-null keys
    // (e.g. "completion_tokens": null) don't block the fallback.
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("prompt_eval_count").and_then(|v| v.as_i64()))
        .map(|v| v as i32);

    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("eval_count").and_then(|v| v.as_i64()))
        .map(|v| v as i32);

    let cache_read_input_tokens = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            usage
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(|v| v.as_i64())
        })
        .map(|v| v as i32);

    let cache_write_input_tokens = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            usage
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cache_write_tokens"))
                .and_then(|v| v.as_i64())
        })
        .map(|v| v as i32);

    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .or_else(|| match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input.saturating_add(output)),
            _ => None,
        });

    Usage::new(input_tokens, output_tokens, total_tokens)
        .with_cache_tokens(cache_read_input_tokens, cache_write_input_tokens)
}

pub fn get_cost(usage: &Value) -> Option<f64> {
    usage
        .get("cost")
        .and_then(|v| v.as_f64())
        .filter(|c| c.is_finite() && *c >= 0.0)
}


pub(crate) fn extract_usage_with_output_tokens(
    chunk: &StreamingChunk,
    fallback_model: Option<&str>,
) -> Option<ProviderUsage> {
    chunk
        .usage
        .as_ref()
        .and_then(|u| {
            chunk.model.as_deref().or(fallback_model).map(|model| {
                let usage = ProviderUsage::new(model.to_string(), get_usage(u));
                match get_cost(u) {
                    Some(cost) => usage.with_cost(cost, CostSource::ProviderReported),
                    None => usage,
                }
            })
        })
        .filter(|u| u.usage.output_tokens.is_some())
}
