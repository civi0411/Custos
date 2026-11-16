use super::messages::{metadata_with_signature, THOUGHT_SIGNATURE_KEY};
use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::Usage;
use crate::errors::ProviderError;
use crate::cloud::codecs::openai::is_valid_function_name;
use anyhow::Result;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde_json::Value;
use std::borrow::Cow;
use uuid::Uuid;

pub(crate) fn process_response_part_impl(
    part: &Value,
    last_signature: &mut Option<String>,
) -> Option<MessageContentBlock> {
    let signature = part.get(THOUGHT_SIGNATURE_KEY).and_then(|v| v.as_str());
    let is_thought = part
        .get("thought")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if let Some(sig) = signature {
        *last_signature = Some(sig.to_string());
    }

    let text_value = part.get("text");
    if let Some(text) = text_value.and_then(|v| v.as_str()) {
        if text.is_empty() {
            return None;
        }
        if is_thought {
            match signature {
                Some(sig) => Some(MessageContentBlock::thinking(
                    text.to_string(),
                    sig.to_string(),
                )),
                None => Some(MessageContentBlock::thinking(text.to_string(), "")),
            }
        } else {
            Some(MessageContentBlock::text(text.to_string()))
        }
    } else if text_value.is_some() {
        tracing::warn!(
            "Google response part has 'text' field but it's not a string: {:?}",
            text_value
        );
        None
    } else if let Some(function_call) = part.get("functionCall") {
        let id = function_call
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let name = function_call["name"].as_str().unwrap_or_default();

        if !is_valid_function_name(name) {
            let error = ErrorData {
                code: ErrorCode::INVALID_REQUEST,
                message: Cow::from(format!(
                    "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                    name
                )),
                data: None,
            };
            Some(MessageContentBlock::tool_request(id, Err(error)))
        } else {
            let arguments = function_call
                .get("args")
                .map(|params| object(params.clone()));
            let effective_signature = signature.or(last_signature.as_deref());
            let metadata = effective_signature.map(metadata_with_signature);

            Some(MessageContentBlock::tool_request_with_metadata(
                id,
                Ok({
                    let mut params = CallToolRequestParams::new(name.to_string());
                    if let Some(args) = arguments {
                        params = params.with_arguments(args);
                    }
                    params
                }),
                metadata.as_ref(),
            ))
        }
    } else {
        None
    }
}

pub fn response_to_message(response: Value) -> Result<Message> {
    let role = Role::Assistant;
    let created = chrono::Utc::now().timestamp();

    let parts = response
        .get("candidates")
        .and_then(|v| v.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());

    let Some(parts) = parts else {
        return Ok(Message::new(role, created, Vec::new()));
    };

    let mut content = Vec::new();
    let mut last_signature: Option<String> = None;

    for part in parts {
        if let Some(msg_content) = process_response_part_impl(part, &mut last_signature) {
            content.push(msg_content);
        }
    }
    Ok(Message::new(role, created, content))
}

/// Extract usage information from Google's API response
pub fn get_usage(data: &Value) -> Result<Usage> {
    if let Some(usage_meta_data) = data.get("usageMetadata") {
        let input_tokens = usage_meta_data
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        // `candidatesTokenCount` is the visible output; thinking models
        // (Gemini 2.5/3) report reasoning tokens separately in
        // `thoughtsTokenCount`, and per the API spec `totalTokenCount` =
        // prompt + thoughts + candidates. Fold thoughts into `output_tokens` so
        // the record reconciles (input + output == total) and cost, which
        // Google bills at the output rate, is correct -- matching the OpenAI
        // (completion_tokens includes reasoning) and Anthropic (output_tokens
        // includes thinking) adapters.
        let candidates_tokens = usage_meta_data
            .get("candidatesTokenCount")
            .and_then(|v| v.as_u64());
        let thoughts_tokens = usage_meta_data
            .get("thoughtsTokenCount")
            .and_then(|v| v.as_u64());
        let output_tokens = match (candidates_tokens, thoughts_tokens) {
            (None, None) => None,
            (candidates, thoughts) => {
                Some((candidates.unwrap_or(0) + thoughts.unwrap_or(0)) as i32)
            }
        };
        let total_tokens = usage_meta_data
            .get("totalTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        // promptTokenCount already includes cachedContentTokenCount
        let cached_tokens = usage_meta_data
            .get("cachedContentTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        Ok(Usage::new(input_tokens, output_tokens, total_tokens)
            .with_cache_tokens(cached_tokens, None))
    } else {
        tracing::debug!(
            "Failed to get usage data: {}",
            ProviderError::UsageError("No usage data found in response".to_string())
        );
        // If no usage data, return None for all values
        Ok(Usage::new(None, None, None))
    }
}
