use crate::errors::ProviderError;
use serde_json::Value;
use super::types::StreamingChunk;

pub(crate) fn strip_data_prefix(line: &str) -> Option<&str> {
    // SSE spec allows both "data: value" and "data:value" (space after colon is optional)
    line.strip_prefix("data: ")
        .or_else(|| line.strip_prefix("data:"))
        .map(|s| s.trim())
}

/// Longest error text pulled out of a stream frame, so a pathological payload cannot be
/// pasted wholesale into a user-facing message.
pub(crate) const MAX_STREAM_ERROR_LEN: usize = 500;

/// Best-effort human-readable text for an error payload that may not be a plain string.
///
/// FastAPI reports `HTTPException` as `{"detail": "..."}` but `RequestValidationError` as
/// `{"detail": [{"loc": [...], "msg": "field required", ...}]}`, so a string-only read would
/// drop the commoner validation shape entirely.
pub(crate) fn stream_error_text(value: &Value) -> Option<String> {
    fn one(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => Some(s.clone()),
            Value::Object(_) => value
                .get("msg")
                .or_else(|| value.get("message"))
                .and_then(|m| m.as_str().map(String::from))
                .or_else(|| Some(value.to_string())),
            Value::Null => None,
            other => Some(other.to_string()),
        }
    }

    let text = match value {
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(one).collect();
            if parts.is_empty() {
                return None;
            }
            parts.join("; ")
        }
        other => one(other)?,
    };
    if text.is_empty() {
        return None;
    }
    if text.chars().count() > MAX_STREAM_ERROR_LEN {
        let truncated: String = text.chars().take(MAX_STREAM_ERROR_LEN).collect();
        return Some(format!("{truncated}…"));
    }
    Some(text)
}

/// Decide whether a choice-less SSE frame reports an in-stream failure.
///
/// Returns `Some(err)` when it does, `None` when it is gateway metadata that can be skipped.
///
/// Requires an actual error *signal* — a `status`/`statusCode`/`code` of 400 or above, a
/// `type` of `"error"`, or a `detail` field, which has no benign meaning in this position.
/// Mere prose is not enough: gateways also emit informational frames, and treating
/// `{"message": "processing"}` as a failure would kill a healthy stream, which is the very
/// bug this skip exists to avoid. The converse matters just as much — a gateway that
/// rate-limits with a bare `{"statusCode": 429, "message": …}` on an HTTP 200 must not be
/// silently skipped, or a failed turn is reported as an empty successful one.
pub(crate) fn classify_choiceless_frame(value: &Value) -> Option<ProviderError> {
    let status = ["status", "statusCode", "code"].iter().find_map(|key| {
        let raw = value.get(*key)?;
        raw.as_i64()
            .or_else(|| raw.as_str().and_then(|s| s.parse::<i64>().ok()))
    });

    let has_error_signal = status.is_some_and(|s| s >= 400)
        || value.get("type").and_then(|t| t.as_str()) == Some("error")
        || value.get("detail").is_some_and(|d| !d.is_null());
    if !has_error_signal {
        return None;
    }

    let details = value
        .get("message")
        .and_then(stream_error_text)
        .or_else(|| value.get("detail").and_then(stream_error_text))
        .or_else(|| value.get("error").and_then(stream_error_text))
        // A status with no recoverable text must still be loud rather than vanish.
        .unwrap_or_else(|| match status {
            Some(s) => format!("Gateway returned status {s} mid-stream"),
            None => "Unknown server error".to_string(),
        });
    Some(ProviderError::ServerError(details))
}

/// Parse one SSE `data:` payload.
///
/// Returns `Ok(None)` for a metadata-only frame — a JSON object with no `choices` key at
/// all. Gateways interleave these with the real chunks: Portkey/Azure APIM and friends
/// emit trace/guardrail objects such as `{"hook_results": {...}}` before the first token.
/// They carry nothing this parser consumes, so they are skipped rather than failed on;
/// treating them as decode errors kills the whole turn on an otherwise healthy stream.
///
/// A frame with `"choices": []` is NOT metadata — that is the standard usage-only chunk,
/// so it still deserializes and flows through the empty-choices paths below.
///
/// A choice-less frame that reports an in-stream failure is NOT metadata either — see
/// `classify_choiceless_frame`.
pub(crate) fn parse_streaming_chunk(line: &str) -> Result<Option<StreamingChunk>, ProviderError> {
    let value: Value = serde_json::from_str(line).map_err(|e| {
        ProviderError::stream_decode_error(format!(
            "Failed to parse streaming chunk: {e}: {line:?}"
        ))
    })?;

    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown server error");
        return Err(ProviderError::ServerError(message.to_string()));
    }

    if value.get("object").and_then(|o| o.as_str()) == Some("error") {
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown server error");
        return Err(ProviderError::ServerError(message.to_string()));
    }

    if value
        .as_object()
        .is_some_and(|o| !o.contains_key("choices"))
    {
        if let Some(err) = classify_choiceless_frame(&value) {
            return Err(err);
        }
        return Ok(None);
    }

    serde_json::from_value(value).map(Some).map_err(|e| {
        ProviderError::stream_decode_error(format!(
            "Failed to parse streaming chunk: {e}: {line:?}"
        ))
    })
}

