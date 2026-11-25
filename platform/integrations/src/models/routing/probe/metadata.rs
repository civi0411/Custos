use serde_json::Value;

pub const CODEX_CLIENT_VERSION: &str = "0.162.0";

pub fn codex_user_agent() -> String {
    format!("codex-cli/{CODEX_CLIENT_VERSION}")
}

/// User-requested conversation curation also applies to aliases in verified caches.
pub fn conversation_model_is_visible(id: &str, label: Option<&str>) -> bool {
    fn hidden(value: &str) -> bool {
        let lower = value.trim().to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        if name == "tab"
            || name.starts_with("tab_")
            || name.starts_with("tab-")
            || name.starts_with("tab ")
        {
            return true;
        }
        if let Some(suffix) = name
            .strip_prefix("gemini-")
            .or_else(|| name.strip_prefix("gemini "))
        {
            let version: String = suffix
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            return matches!(version.as_str(), "2.5" | "3" | "3.1" | "3.5" | "3.6");
        }
        false
    }
    !hidden(id) && !label.is_some_and(hidden)
}

/// Conservative capability filtering; absent metadata does not invent an entitlement.
pub fn model_is_selectable(model: &Value) -> bool {
    let id = model
        .get("id")
        .or_else(|| model.get("slug"))
        .or_else(|| model.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if [
        "text-embedding",
        "embedding",
        "tts-",
        "whisper-",
        "dall-e",
        "omni-moderation",
        "text-moderation",
    ]
    .iter()
    .any(|prefix| id.starts_with(prefix))
        || id.contains("realtime")
        || id.contains("gpt-audio")
        || id.contains("gpt-image")
    {
        return false;
    }
    if model.get("isInternal").and_then(Value::as_bool) == Some(true)
        || model.get("disabled").and_then(Value::as_bool) == Some(true)
        || model.get("available").and_then(Value::as_bool) == Some(false)
        || model.get("enabled").and_then(Value::as_bool) == Some(false)
        || model.get("model_picker_enabled").and_then(Value::as_bool) == Some(false)
        || model
            .get("visibility")
            .and_then(Value::as_str)
            .is_some_and(|v| matches!(v, "hide" | "hidden" | "none"))
    {
        return false;
    }
    for pointer in ["/output_modalities", "/capabilities/output_modalities"] {
        if let Some(modalities) = model.pointer(pointer).and_then(Value::as_array) {
            if !modalities.iter().any(|v| v.as_str() == Some("text")) {
                return false;
            }
        }
    }
    model
        .pointer("/capabilities/type")
        .and_then(Value::as_str)
        .is_none_or(|kind| matches!(kind, "chat" | "completion" | "text"))
}

pub fn advertised_efforts(model: &Value) -> Vec<String> {
    let levels = model
        .get("supported_reasoning_levels")
        .or_else(|| model.pointer("/capabilities/supports/reasoning_effort"))
        .or_else(|| model.get("efforts"));
    levels
        .and_then(Value::as_array)
        .map(|levels| {
            levels
                .iter()
                .filter_map(|level| {
                    level
                        .as_str()
                        .or_else(|| level.get("effort").and_then(Value::as_str))
                        .filter(|value| {
                            matches!(
                                *value,
                                "none"
                                    | "minimal"
                                    | "low"
                                    | "medium"
                                    | "high"
                                    | "xhigh"
                                    | "max"
                                    | "ultra"
                            )
                        })
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn advertised_default_effort(model: &Value) -> Option<String> {
    let value = model
        .get("default_reasoning_level")
        .or_else(|| model.get("default_effort"))
        .and_then(Value::as_str)?;
    advertised_efforts(model)
        .iter()
        .any(|effort| effort == value)
        .then(|| value.to_string())
}

/// Scope is shared by discovery, dispatch admission and cached capability reads.
pub fn verified_catalog_key(
    account: &str,
    endpoint: &str,
    oauth: bool,
    generation: i64,
    service: &str,
) -> String {
    let base = format!(
        "verified-probe:{}:{}:{}:{}",
        account,
        endpoint.trim().trim_end_matches('/'),
        if oauth { "oauth" } else { "api_key" },
        generation
    );
    if oauth && matches!(service, "openai" | "codex") {
        format!("{base}:codex-{CODEX_CLIENT_VERSION}")
    } else {
        base
    }
}
