//! Secret/token scrubbing and byte-size capping prior to supervision persistence.

use serde_json::{Map, Value};

const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "token",
    "access_token",
    "refresh_token",
    "secret",
    "password",
    "passwd",
    "authorization",
    "cookie",
    "private_key",
    "credential",
];

/// Scrubs sensitive keys and inline bearer/API tokens from JSON metadata and strings.
pub struct PayloadRedactor {
    pub enabled: bool,
    pub max_payload_bytes: usize,
}

impl Default for PayloadRedactor {
    fn default() -> Self {
        Self {
            enabled: true,
            max_payload_bytes: 16 * 1024,
        }
    }
}

impl PayloadRedactor {
    pub fn redact_text(&self, input: &str) -> String {
        if !self.enabled {
            return truncate_utf8(input, self.max_payload_bytes);
        }
        let scrubbed = scrub_inline_secrets(input);
        truncate_utf8(&scrubbed, self.max_payload_bytes)
    }

    pub fn redact_json(&self, value: Value) -> Value {
        let scrubbed = if self.enabled {
            scrub_json_value(value)
        } else {
            value
        };
        if self.max_payload_bytes == 0 {
            return Value::Object(Map::new());
        }
        if let Ok(serialized) = serde_json::to_string(&scrubbed) {
            if serialized.len() > self.max_payload_bytes {
                let preview = truncate_utf8(&serialized, self.max_payload_bytes.saturating_sub(64));
                return serde_json::json!({
                    "_truncated": true,
                    "original_bytes": serialized.len(),
                    "preview": preview,
                });
            }
        }
        scrubbed
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|needle| lower.contains(needle))
}

fn scrub_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut clean = Map::with_capacity(map.len());
            for (k, v) in map {
                if is_sensitive_key(&k) {
                    clean.insert(k, Value::String("[REDACTED]".into()));
                } else {
                    clean.insert(k, scrub_json_value(v));
                }
            }
            Value::Object(clean)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(scrub_json_value).collect()),
        Value::String(s) => Value::String(scrub_inline_secrets(&s)),
        other => other,
    }
}

fn scrub_inline_secrets(text: &str) -> String {
    let mut result = text.to_string();
    for prefix in ["Bearer ", "bearer ", "sk-", "ghp_", "xoxb-"] {
        if let Some(pos) = result.find(prefix) {
            let start = pos + prefix.len();
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                .map(|rel| start + rel)
                .unwrap_or(result.len());
            if end > start {
                result.replace_range(start..end, "[REDACTED]");
            }
        }
    }
    result
}

fn truncate_utf8(input: &str, max_bytes: usize) -> String {
    if max_bytes == 0 {
        return String::new();
    }
    if input.len() <= max_bytes {
        return input.to_string();
    }
    let mut end = max_bytes.min(input.len());
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} [truncated]", &input[..end])
}

