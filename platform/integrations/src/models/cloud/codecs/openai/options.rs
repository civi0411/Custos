use crate::base::ThinkingPreservationFormat;
use crate::thinking::ThinkingEffort;
use regex::Regex;
use std::sync::OnceLock;

pub fn is_reserved_request_param_key(key: &str) -> bool {
    matches!(key, "messages" | "model" | "stream" | "stream_options")
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAiFormatOptions {
    pub preserve_thinking_context: bool,
    pub supports_vision: bool,
    pub thinking_preservation_format: Option<ThinkingPreservationFormat>,
}

pub(crate) fn merge_reasoning_text(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        return suffix.to_string();
    }
    if suffix.is_empty() {
        return prefix.to_string();
    }
    if suffix.starts_with(prefix) {
        return suffix.to_string();
    }
    if prefix.ends_with(suffix) {
        return prefix.to_string();
    }

    format!("{prefix}{suffix}")
}

pub use crate::contracts::types::model::{
    extract_reasoning_effort, is_openai_responses_model, is_xai_reasoning_model,
    supports_xai_reasoning_effort,
};

/// Maps Goose's effort levels to values accepted by xAI Chat Completions.
pub fn xai_reasoning_effort_for_thinking(
    model_name: &str,
    effort: ThinkingEffort,
) -> Option<String> {
    let model_name = model_name.to_ascii_lowercase();
    let supports_none = model_name.starts_with("grok-4.3");
    let supports_medium = !model_name.starts_with("grok-3-mini");

    match effort {
        ThinkingEffort::Off if supports_none => Some("none".to_string()),
        ThinkingEffort::Off => Some("low".to_string()),
        ThinkingEffort::Low => Some("low".to_string()),
        ThinkingEffort::Medium if supports_medium => Some("medium".to_string()),
        ThinkingEffort::Medium | ThinkingEffort::High | ThinkingEffort::Max => {
            Some("high".to_string())
        }
    }
}

pub fn openai_reasoning_effort_for_thinking(
    model_name: &str,
    effort: ThinkingEffort,
) -> Option<String> {
    let supported = openai_reasoning_efforts_for_model(model_name);

    let preferred: &[&str] = match effort {
        ThinkingEffort::Off => &["none", "low"],
        ThinkingEffort::Low => &["low", "medium", "high", "xhigh"],
        ThinkingEffort::Medium => &["medium", "high", "low", "xhigh"],
        ThinkingEffort::High => &["high", "medium", "xhigh", "low"],
        ThinkingEffort::Max => &["max", "xhigh", "high", "medium", "low"],
    };

    preferred
        .iter()
        .find(|level| supported.contains(level))
        .map(|level| (*level).to_string())
}

pub(crate) fn openai_reasoning_efforts_for_model(model_name: &str) -> &'static [&'static str] {
    let normalized = model_name.to_ascii_lowercase();

    if normalized.contains("gpt-5") || normalized.contains("gpt-6") {
        if normalized.contains("-pro") || normalized.contains("/pro") {
            &["high"]
        } else if normalized.contains("gpt-6") {
            // GPT-6 Astra does not accept `none`; Sol and Luna do.
            if normalized.contains("astra") {
                &["low", "medium", "high", "xhigh", "max"]
            } else {
                &["none", "low", "medium", "high", "xhigh", "max"]
            }
        } else if normalized.contains("gpt-5.4")
            || normalized.contains("gpt-5-4")
            || normalized.contains("gpt-5.5")
            || normalized.contains("gpt-5-5")
            || normalized.contains("gpt-5.6")
            || normalized.contains("gpt-5-6")
        {
            &["none", "low", "medium", "high", "xhigh"]
        } else {
            &["low", "medium", "high"]
        }
    } else {
        &["low", "medium", "high"]
    }
}

pub(crate) const MAX_FUNCTION_NAME_LENGTH: usize = 128;

pub fn sanitize_function_name(name: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[^a-zA-Z0-9_-]").unwrap());
    re.replace_all(name, "_")
        .chars()
        .take(MAX_FUNCTION_NAME_LENGTH)
        .collect()
}

pub fn is_valid_function_name(name: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());
    re.is_match(name)
}
