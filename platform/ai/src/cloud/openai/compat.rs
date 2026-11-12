use crate::cloud::codecs::openai::is_reserved_request_param_key;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use serde_json::json;
use std::collections::HashMap;

/// Providers known to reject `max_completion_tokens` and require
/// the legacy `max_tokens` field instead.
pub const PROVIDERS_NEEDING_MAX_TOKENS_REMAP: &[&str] = &[
    "cerebras",
    "custom_deepseek",
    "groq",
    "inception",
    "kimi",
    "lmstudio",
    "mistral",
    "moonshot",
    "nearai",
    "ovhcloud",
];

pub const PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS: &[&str] = &["nearai", "pleumrouter"];

/// Providers whose endpoints require the non-standard `tool_stream`
/// field to stream tool-call arguments incrementally.
pub const PROVIDERS_NEEDING_TOOL_STREAM: &[&str] = &["zai_coding_plan"];

/// Providers whose reasoning models accept an OpenAI-style
/// `reasoning_effort` field on chat-completions requests but aren't
/// matched by [`is_openai_responses_model`] (which only recognises
/// OpenAI's own `o*`/`gpt-5*` model names). These need the unified
/// [`ThinkingEffort`] mapped onto the request explicitly.
pub const PROVIDERS_NEEDING_REASONING_EFFORT_MAPPING: &[&str] = &["meta"];

/// Maps the unified thinking effort onto Meta's Muse Spark
/// `reasoning_effort` levels: `low`, `medium`, `high`, `xhigh`.
///
/// Muse Spark always reasons and has no supported "disable reasoning"
/// level, so `Off` is clamped to `low` (the lightest level Meta
/// supports) rather than sent as-is or omitted.
pub fn meta_reasoning_effort(effort: ThinkingEffort) -> &'static str {
    match effort {
        ThinkingEffort::Off | ThinkingEffort::Low => "low",
        ThinkingEffort::Medium => "medium",
        ThinkingEffort::High => "high",
        ThinkingEffort::Max => "xhigh",
    }
}

pub fn sanitize_request_for_compat(
    provider_name: &str,
    mut payload: serde_json::Value,
    model_config: &ModelConfig,
    is_responses_model_fn: impl Fn(&str) -> bool,
) -> serde_json::Value {
    if let Some(obj) = payload.as_object_mut() {
        if PROVIDERS_NEEDING_TOOL_STREAM.contains(&provider_name)
            && obj.get("stream") == Some(&json!(true))
        {
            obj.entry("tool_stream").or_insert(json!(true));
        }

        if PROVIDERS_NEEDING_MAX_TOKENS_REMAP.contains(&provider_name) {
            if let Some(value) = obj.remove("max_completion_tokens") {
                obj.entry("max_tokens").or_insert(value);
            }
        }

        if PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS.contains(&provider_name) {
            let model_name = obj.get("model").and_then(|model| model.as_str());
            if !model_name.is_some_and(is_responses_model_fn) {
                obj.remove("reasoning_effort");
            }

            if let Some(messages) = obj.get_mut("messages").and_then(|m| m.as_array_mut()) {
                for message in messages {
                    if message
                        .get("role")
                        .and_then(|role| role.as_str())
                        .is_some_and(|role| role == "developer")
                    {
                        message["role"] = serde_json::Value::String("system".to_string());
                    }
                }
            }
        }

        if PROVIDERS_NEEDING_REASONING_EFFORT_MAPPING.contains(&provider_name) {
            match model_config.thinking_effort() {
                Some(effort) => {
                    obj.insert(
                        "reasoning_effort".to_string(),
                        json!(meta_reasoning_effort(effort)),
                    );
                }
                None => {
                    obj.remove("reasoning_effort");
                }
            }
        }
    }

    payload
}

/// Merges a model's declared `request_params` into an already-built payload.
///
/// Reserved keys are skipped so a declaration cannot clobber the streaming setup.
pub fn apply_declared_request_params(
    payload: &mut serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };

    for (key, value) in params {
        if !is_reserved_request_param_key(key) {
            object.insert(key.clone(), value.clone());
        }
    }
}
