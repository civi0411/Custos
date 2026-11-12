use crate::conversation::message::Message;
use crate::conversation::token_usage::{CostSource, ProviderUsage, Usage};
use crate::errors::ProviderError;
use anyhow::{anyhow, Result};
use rmcp::model::{object, CallToolRequestParams};
use serde_json::{Map, Value};
use super::options::*;

pub fn response_to_message(response: &Value) -> Result<Message> {
    let content_blocks = response
        .get(CONTENT_FIELD)
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("Invalid response format: missing content array"))?;

    let mut message = Message::assistant();

    for block in content_blocks {
        match block.get(TYPE_FIELD).and_then(|t| t.as_str()) {
            Some(TEXT_TYPE) => {
                if let Some(text) = block.get(TEXT_TYPE).and_then(|t| t.as_str()) {
                    message = message.with_text(text.to_string());
                }
            }
            Some(TOOL_USE_TYPE) => {
                let id = block
                    .get(ID_FIELD)
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use id"))?;
                let name = block
                    .get(NAME_FIELD)
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use name"))?
                    .to_string();
                let input = block
                    .get(INPUT_FIELD)
                    .ok_or_else(|| anyhow!("Missing tool_use input"))?;

                let tool_call =
                    CallToolRequestParams::new(name).with_arguments(object(input.clone()));
                message = message.with_tool_request(id, Ok(tool_call));
            }
            Some(THINKING_TYPE) => {
                let thinking = block
                    .get(THINKING_TYPE)
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow!("Missing thinking content"))?
                    .to_string();
                let signature = block
                    .get(SIGNATURE_FIELD)
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                message = message.with_thinking(thinking, signature);
            }
            Some(REDACTED_THINKING_TYPE) => {
                let data = block
                    .get(DATA_FIELD)
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| anyhow!("Missing redacted_thinking data"))?;
                message = message.with_redacted_thinking(data);
            }
            _ => continue,
        }
    }

    Ok(message)
}

pub(crate) fn usage_from_anthropic_fields(usage: &Value) -> Usage {
    let field = |key: &str| {
        usage
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v.min(i32::MAX as u64) as i32)
    };

    Usage::from_cache_exclusive_input(
        Some(field("input_tokens").unwrap_or(0)),
        Some(field("output_tokens").unwrap_or(0)),
        None,
        field("cache_read_input_tokens"),
        field("cache_creation_input_tokens"),
    )
}

/// Merge a `message_delta` usage into the usage captured at `message_start`.
/// Delta usage is cumulative (input grows during server tool use), so fields
/// present in the raw delta payload win over the start values.
pub(crate) fn merge_delta_usage(existing: &Usage, delta: &Usage, delta_data: &Value) -> Usage {
    let reports = |key: &str| delta_data.get(key).is_some();

    let output = if reports("output_tokens") {
        delta.output_tokens
    } else {
        existing.output_tokens
    };

    if !reports("input_tokens") {
        Usage::new(existing.input_tokens, output, None).with_cache_tokens(
            existing.cache_read_input_tokens,
            existing.cache_write_input_tokens,
        )
    } else if reports("cache_read_input_tokens") || reports("cache_creation_input_tokens") {
        Usage::new(delta.input_tokens, output, None).with_cache_tokens(
            delta.cache_read_input_tokens,
            delta.cache_write_input_tokens,
        )
    } else {
        Usage::from_cache_exclusive_input(
            delta.input_tokens,
            output,
            None,
            existing.cache_read_input_tokens,
            existing.cache_write_input_tokens,
        )
    }
}

pub fn get_usage(data: &Value) -> Result<Usage> {
    if let Some(usage) = data.get("usage") {
        Ok(usage_from_anthropic_fields(usage))
    } else if data.as_object().is_some() {
        // Check if the data itself is the usage object (for message_delta events that might have usage at top level)
        let usage = usage_from_anthropic_fields(data);
        if usage.total_tokens.unwrap_or(0) > 0 {
            Ok(usage)
        } else {
            tracing::debug!("🔍 Anthropic no token data found in object");
            Ok(Usage::new(None, None, None))
        }
    } else {
        tracing::debug!(
            "Failed to get usage data: {}",
            ProviderError::UsageError("No usage data found in response".to_string())
        );
        // If no usage data, return None for all values
        Ok(Usage::new(None, None, None))
    }
}

/// Anthropic response fields that have no canonical `ProviderUsage` equivalent.
const ADDITIONAL_USAGE_FIELDS: [&str; 1] = ["service_tier"];

pub fn input_transformations(message_data: &Value) -> Option<Value> {
    let transformations = message_data.get(INPUT_TRANSFORMATIONS_FIELD)?.as_array()?;
    let dropped: Vec<(&str, &str)> = transformations
        .iter()
        .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("thinking_dropped"))
        .map(|t| {
            (
                t.get("path").and_then(Value::as_str).unwrap_or(""),
                t.get("reason").and_then(Value::as_str).unwrap_or(""),
            )
        })
        .collect();
    if !dropped.is_empty() {
        tracing::warn!(?dropped, "API dropped thinking blocks from the request");
    }
    Some(Value::Array(transformations.clone()))
}

pub fn get_additional_data(data: &Value) -> Option<Map<String, Value>> {
    let usage = data.get("usage")?.as_object()?;
    let additional: Map<String, Value> = ADDITIONAL_USAGE_FIELDS
        .iter()
        .filter_map(|field| Some(((*field).to_string(), usage.get(*field)?.clone())))
        .collect();
    (!additional.is_empty()).then_some(additional)
}

pub(crate) fn provider_usage_with_cost(
    model: String,
    usage: Usage,
    data: &Value,
    fallback_cost: Option<f64>,
) -> ProviderUsage {
    let provider_usage = ProviderUsage::new(model, usage);
    match crate::cloud::codecs::openai::get_cost(data).or(fallback_cost) {
        Some(cost) => provider_usage.with_cost(cost, CostSource::ProviderReported),
        None => provider_usage,
    }
}

