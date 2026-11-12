use std::collections::HashMap;

use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

/// Request param keys that describe model-family-agnostic reasoning behavior and
/// are therefore safe to carry across a model switch or subagent delegation.
/// Provider-specific keys (e.g. `anthropic_beta`) are deliberately excluded so
/// they can't bleed into a request targeting a different model family.
pub(crate) const INHERITED_SESSION_PARAM_KEYS: &[&str] = &[
    "thinking_effort",
    "thinking_budget",
    "budget_tokens",
    "enable_thinking",
    "preserve_thinking_context",
    "preserve_unsigned_thinking",
];

/// Request params Custos/Goose consumes internally: formats that forward unknown params into
/// the payload must skip these, or the provider gets an unrecognized wire parameter.
pub fn is_custos_internal_request_param(key: &str) -> bool {
    matches!(
        key,
        "thinking_effort"
            | "disable_prompt_cache"
            | "cache_ttl"
            | "emit_clear_thinking"
            | "preserve_thinking_context"
            | "preserve_unsigned_thinking"
    )
}

/// Backwards-compatible alias for Goose formats
pub use is_custos_internal_request_param as is_goose_internal_request_param;

#[derive(Debug, Clone, Serialize)]
pub struct ModelConfig {
    pub model_name: String,
    #[serde(skip)]
    pub context_limit: Option<usize>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub toolshim: bool,
    pub toolshim_model: Option<String>,
    /// Provider-specific request parameters (e.g., anthropic_beta headers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_params: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_vision: Option<bool>,
    /// Per-request HTTP headers attached to outgoing provider calls.
    /// Never serialized into request bodies.
    #[serde(skip)]
    pub request_headers: Option<HashMap<String, String>>,
}

impl<'de> Deserialize<'de> for ModelConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawModelConfig {
            model_name: String,
            #[serde(rename = "context_limit")]
            _context_limit: Option<usize>,
            temperature: Option<f32>,
            max_tokens: Option<i32>,
            toolshim: bool,
            toolshim_model: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            request_params: Option<HashMap<String, Value>>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            reasoning: Option<bool>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            supports_vision: Option<bool>,
        }

        let raw = RawModelConfig::deserialize(deserializer)?;
        let mut config = Self {
            model_name: raw.model_name,
            context_limit: None,
            temperature: raw.temperature,
            max_tokens: raw.max_tokens,
            toolshim: raw.toolshim,
            toolshim_model: raw.toolshim_model,
            request_params: raw.request_params,
            reasoning: raw.reasoning,
            supports_vision: raw.supports_vision,
            request_headers: None,
        };
        config.normalize_effort_suffix();
        Ok(config)
    }
}

