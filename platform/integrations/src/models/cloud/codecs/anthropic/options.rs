use crate::catalog::canonical::{maybe_get_canonical_model, ThinkingMode};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use serde_json::{json, Value};
use std::fmt;
use std::str::FromStr;

pub const ANTHROPIC_PROVIDER_NAME: &str = "anthropic";

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $str:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)+
                    other => Err(format!("unknown {}: '{other}'", stringify!($name))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self { $(Self::$variant => write!(f, $str),)+ }
            }
        }
    }
}

string_enum!(ThinkingType { Adaptive => "adaptive", Enabled => "enabled", Disabled => "disabled" });
string_enum!(CacheTtl { FiveMinutes => "5m", OneHour => "1h" });

string_enum!(PrefixMismatchBehavior { DropBlock => "drop_block", Error => "error" });

pub const THINKING_BINDING_CONTROLS_BETA: &str = "thinking-binding-controls-2026-08-01";
pub const INPUT_TRANSFORMATIONS_FIELD: &str = "input_transformations";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnthropicFormatOptions {
    pub preserve_unsigned_thinking: bool,
    pub preserve_thinking_context: bool,
    pub thinking_disabled: bool,
    pub emit_clear_thinking: bool,
    pub current_model: Option<String>,
    pub prompt_cache_disabled: bool,
    pub cache_ttl: Option<CacheTtl>,
    pub prefix_mismatch_behavior: Option<PrefixMismatchBehavior>,
    pub strip_thinking_history: bool,
}

impl AnthropicFormatOptions {
    /// Anthropic-compatible providers keep `Default`, which does not request block binding.
    pub fn native() -> Self {
        Self {
            prefix_mismatch_behavior: Some(PrefixMismatchBehavior::DropBlock),
            ..Self::default()
        }
    }

    pub(crate) fn for_model(self, model_config: &ModelConfig) -> Self {
        let preserve_thinking_context = model_config
            .request_param::<bool>("preserve_thinking_context")
            .unwrap_or(self.preserve_thinking_context);
        let preserve_unsigned_thinking = model_config
            .request_param::<bool>("preserve_unsigned_thinking")
            .unwrap_or(self.preserve_unsigned_thinking)
            || preserve_thinking_context;
        let thinking_disabled = model_config.reasoning == Some(false)
            || model_config.thinking_effort() == Some(ThinkingEffort::Off);
        let emit_clear_thinking = model_config
            .request_param::<bool>("emit_clear_thinking")
            .unwrap_or(self.emit_clear_thinking);
        let cache_ttl = model_config
            .cache_ttl()
            .and_then(|ttl| ttl.parse::<CacheTtl>().ok())
            .or(self.cache_ttl);
        let prefix_mismatch_behavior = match model_config
            .request_param::<String>("prefix_mismatch_behavior")
            .as_deref()
        {
            None => self.prefix_mismatch_behavior,
            Some("off") => None,
            Some(value) => value.parse().ok().or(self.prefix_mismatch_behavior),
        };

        Self {
            preserve_unsigned_thinking,
            preserve_thinking_context,
            thinking_disabled,
            emit_clear_thinking,
            current_model: self
                .current_model
                .or_else(|| Some(model_config.model_name.clone())),
            prompt_cache_disabled: model_config.prompt_cache_disabled(),
            cache_ttl,
            prefix_mismatch_behavior,
            strip_thinking_history: self.strip_thinking_history,
        }
    }

    /// `{"type":"ephemeral"}` selects Anthropic's default 5m TTL; the `ttl`
    /// field is only sent for an explicit 1h opt-in, since a 1h write is
    /// billed at 2x input instead of 1.25x.
    pub(crate) fn cache_control(&self) -> Value {
        match self.cache_ttl {
            Some(CacheTtl::OneHour) => {
                json!({ TYPE_FIELD: "ephemeral", "ttl": "1h" })
            }
            _ => json!({ TYPE_FIELD: "ephemeral" }),
        }
    }
}

pub fn thinking_block_is_stale(message: &Message, current_model: Option<&str>) -> bool {
    let Some(current_model) = current_model else {
        return false;
    };
    let Some(inference) = message.metadata.inference.as_ref() else {
        return false;
    };
    let requested = inference.requested_model.as_str();
    let resolved = inference.resolved_model.as_deref().unwrap_or("");
    if requested.is_empty() && resolved.is_empty() {
        return false;
    }
    current_model != requested && current_model != resolved
}

fn canonical_thinking_mode(provider_name: &str, model_name: &str) -> Option<ThinkingMode> {
    maybe_get_canonical_model(provider_name, model_name).and_then(|model| model.thinking_mode)
}

/// Adaptive models run adaptive thinking when `thinking` is omitted, so turning
/// it off takes an explicit disable. Always-on models reject that disable.
pub fn requires_explicit_thinking_disable(provider_name: &str, model_name: &str) -> bool {
    canonical_thinking_mode(provider_name, model_name) == Some(ThinkingMode::Adaptive)
}

fn canonical_reasoning(provider_name: &str, model_config: &ModelConfig) -> Option<bool> {
    maybe_get_canonical_model(provider_name, &model_config.model_name)
        .and_then(|model| model.reasoning)
}

pub fn model_supports_temperature(provider_name: &str, model_config: &ModelConfig) -> bool {
    maybe_get_canonical_model(provider_name, &model_config.model_name)
        .and_then(|model| model.temperature)
        .unwrap_or(true)
}

pub fn thinking_type(model_config: &ModelConfig) -> ThinkingType {
    thinking_type_for_provider(ANTHROPIC_PROVIDER_NAME, model_config)
}

pub fn thinking_type_for_provider(provider_name: &str, model_config: &ModelConfig) -> ThinkingType {
    let mode = canonical_thinking_mode(provider_name, &model_config.model_name);
    let reasoning = model_config
        .reasoning
        .or_else(|| canonical_reasoning(provider_name, model_config));

    if reasoning != Some(true) {
        return ThinkingType::Disabled;
    }

    if mode == Some(ThinkingMode::AlwaysOnAdaptive) {
        return ThinkingType::Adaptive;
    }

    let effort = model_config.thinking_effort();

    if effort.is_none() && model_config.request_param::<i32>("budget_tokens").is_some() {
        return match mode {
            Some(ThinkingMode::Adaptive) => ThinkingType::Adaptive,
            _ => ThinkingType::Enabled,
        };
    }

    match effort.unwrap_or(ThinkingEffort::Off) {
        ThinkingEffort::Off => ThinkingType::Disabled,
        _ if mode == Some(ThinkingMode::Adaptive) => ThinkingType::Adaptive,
        _ => ThinkingType::Enabled,
    }
}

// Constants for frequently used strings in Anthropic API format
pub(crate) const TYPE_FIELD: &str = "type";
pub(crate) const CONTENT_FIELD: &str = "content";
pub(crate) const TEXT_TYPE: &str = "text";
pub(crate) const ROLE_FIELD: &str = "role";
pub(crate) const USER_ROLE: &str = "user";
pub(crate) const ASSISTANT_ROLE: &str = "assistant";
pub(crate) const TOOL_USE_TYPE: &str = "tool_use";
pub(crate) const TOOL_RESULT_TYPE: &str = "tool_result";
pub(crate) const THINKING_TYPE: &str = "thinking";
pub(crate) const REDACTED_THINKING_TYPE: &str = "redacted_thinking";
pub(crate) const CACHE_CONTROL_FIELD: &str = "cache_control";
pub(crate) const ID_FIELD: &str = "id";
pub(crate) const NAME_FIELD: &str = "name";
pub(crate) const INPUT_FIELD: &str = "input";
pub(crate) const TOOL_USE_ID_FIELD: &str = "tool_use_id";
pub(crate) const IS_ERROR_FIELD: &str = "is_error";
pub(crate) const SIGNATURE_FIELD: &str = "signature";
pub(crate) const DATA_FIELD: &str = "data";
pub(crate) const IMAGE_TYPE: &str = "image";
pub(crate) const DOCUMENT_TYPE: &str = "document";
pub(crate) const SOURCE_FIELD: &str = "source";
pub(crate) const BASE64_TYPE: &str = "base64";
pub(crate) const MEDIA_TYPE_FIELD: &str = "media_type";
// Claude vision only accepts these image media types; other image/* blobs fall
// through to the text/binary-marker path so an unsupported type (e.g.
// image/svg+xml) doesn't turn the next request into a provider rejection.
pub(crate) const ANTHROPIC_IMAGE_MEDIA_TYPES: [&str; 4] =
    ["image/jpeg", "image/png", "image/gif", "image/webp"];
pub(crate) const EVENT_MESSAGE_START: &str = "message_start";
pub(crate) const EVENT_MESSAGE_DELTA: &str = "message_delta";
pub(crate) const EVENT_MESSAGE_STOP: &str = "message_stop";
pub(crate) const EVENT_CONTENT_BLOCK_START: &str = "content_block_start";
pub(crate) const EVENT_CONTENT_BLOCK_DELTA: &str = "content_block_delta";
pub(crate) const EVENT_CONTENT_BLOCK_STOP: &str = "content_block_stop";
pub(crate) const STOP_REASON_REFUSAL: &str = "refusal";
pub(crate) const REFUSAL_FALLBACK_DETAILS: &str = "No additional details were provided.";

/// Coerce a tool call's optional arguments into the JSON value Anthropic
/// expects for the `input` field of a `tool_use` content block.
///
/// Anthropic's Messages API requires `input` to be an object. When the
/// internal `CallToolRequestParams::arguments` is `None` (which happens for
/// parameterless tools, tool calls round-tripped from disk, or calls created
/// via `CallToolRequestParams::new` without `.with_arguments(...)`) the
/// `json!` macro would otherwise serialize it as JSON `null` and the API
/// rejects the next replay of the tool_use block with a 400 error:
/// `messages.<N>.content.<M>.tool_use.input: Input should be an object.`
/// See issue #9287.
pub fn thinking_effort(model_config: &ModelConfig) -> ThinkingEffort {
    model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High)
}

pub fn adaptive_output_effort(model_config: &ModelConfig) -> ThinkingEffort {
    match thinking_effort(model_config) {
        ThinkingEffort::Off => ThinkingEffort::High,
        effort => effort,
    }
}

pub fn thinking_budget_tokens(model_config: &ModelConfig) -> i32 {
    if let Some(request_param) = model_config
        .request_params
        .as_ref()
        .and_then(|params| params.get("budget_tokens"))
        .and_then(|v| serde_json::from_value::<i32>(v.clone()).ok())
    {
        return request_param.max(1024);
    }

    let effort = model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High);
    match effort {
        ThinkingEffort::Off => 1024,
        ThinkingEffort::Low => 4000,
        ThinkingEffort::Medium => 10000,
        ThinkingEffort::High => 16000,
        ThinkingEffort::Max => 32000,
    }
}

// Anthropic counts thinking tokens against max_tokens, so the budget must leave
// room for a response. Clamp it to preserve at least this many answer tokens, and
// drop thinking only when even a minimal budget wouldn't fit under the cap.
// Shared with the Bedrock formatter, which applies the same clamp.
pub const MIN_ANSWER_TOKENS: i32 = 1024;

pub(crate) fn apply_thinking_config(
    payload: &mut Value,
    provider_name: &str,
    model_config: &ModelConfig,
    max_tokens: i32,
    options: AnthropicFormatOptions,
) {
    let obj = payload.as_object_mut().unwrap();
    match thinking_type_for_provider(provider_name, model_config) {
        ThinkingType::Adaptive => {
            obj.insert("thinking".to_string(), json!({"type": "adaptive"}));
            let effort = adaptive_output_effort(model_config).to_string();
            obj.insert("output_config".to_string(), json!({"effort": effort}));
        }
        ThinkingType::Enabled => {
            let budget_tokens = thinking_budget_tokens(model_config)
                .min(max_tokens.saturating_sub(MIN_ANSWER_TOKENS));
            if budget_tokens >= MIN_ANSWER_TOKENS {
                obj.insert(
                    "thinking".to_string(),
                    json!({
                        "type": "enabled",
                        "budget_tokens": budget_tokens
                    }),
                );
            }
        }
        ThinkingType::Disabled => {}
    }

    if options.preserve_thinking_context && !options.thinking_disabled {
        if !obj.contains_key("thinking") {
            let budget_tokens = thinking_budget_tokens(model_config)
                .min(max_tokens.saturating_sub(MIN_ANSWER_TOKENS));
            if budget_tokens >= MIN_ANSWER_TOKENS {
                obj.insert(
                    "thinking".to_string(),
                    json!({
                        "type": "enabled",
                        "budget_tokens": budget_tokens
                    }),
                );
            }
        }

        // Z.AI requires this to preserve reasoning; Anthropic rejects it.
        if options.emit_clear_thinking {
            if let Some(thinking) = obj.get_mut("thinking").and_then(|t| t.as_object_mut()) {
                thinking.insert("clear_thinking".to_string(), json!(false));
            }
        }
    }

    if !obj.contains_key("thinking")
        && requires_explicit_thinking_disable(provider_name, &model_config.model_name)
    {
        obj.insert("thinking".to_string(), json!({"type": "disabled"}));
    }

    // `block_binding` is only accepted alongside adaptive or enabled thinking.
    if let Some(behavior) = options.prefix_mismatch_behavior {
        if let Some(thinking) = obj.get_mut("thinking").and_then(|t| t.as_object_mut()) {
            if thinking.get("type").and_then(|t| t.as_str()) != Some("disabled") {
                thinking.insert(
                    "block_binding".to_string(),
                    json!({"prefix_mismatch_behavior": behavior.to_string()}),
                );
            }
        }
    }
}

pub fn block_binding_behavior(payload: &Value) -> Option<PrefixMismatchBehavior> {
    payload
        .pointer("/thinking/block_binding/prefix_mismatch_behavior")
        .and_then(Value::as_str)
        .and_then(|behavior| behavior.parse().ok())
}

pub use crate::contracts::types::retry::is_thinking_signature_error;

