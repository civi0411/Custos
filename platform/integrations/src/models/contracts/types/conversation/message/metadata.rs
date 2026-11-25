use crate::conversation::token_usage::{CostSource, ProviderUsage};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InferenceMetadata {
    pub provider: String,
    pub requested_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_session_id: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_source: Option<CostSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_token_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_compaction: bool,
}

impl MessageUsage {
    pub fn from_provider_usage(usage: &ProviderUsage, is_compaction: bool) -> Self {
        let stats = usage.stats.as_ref();
        MessageUsage {
            input_tokens: usage.usage.input_tokens,
            output_tokens: usage.usage.output_tokens,
            total_tokens: usage.usage.total_tokens,
            cache_read_tokens: usage.usage.cache_read_input_tokens,
            cache_write_tokens: usage.usage.cache_write_input_tokens,
            cost: usage.cost,
            cost_source: usage.cost_source,
            elapsed_ms: stats.and_then(|s| s.elapsed_ms),
            time_to_first_token_ms: stats.and_then(|s| s.time_to_first_token_ms),
            is_compaction,
        }
    }
}

/// Notes an operation left on a message, keyed by operation name. Nested rather
/// than free-form so metadata that is not shaped like notes fails to load
/// instead of being silently ignored.
pub type OperationNotes =
    std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
/// Metadata for message visibility and model inference details
#[serde(rename_all = "camelCase")]
pub struct MessageMetadata {
    /// Whether the message should be visible to the user in the UI
    pub user_visible: bool,
    /// Whether the message should be included in the agent's context window
    pub agent_visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceMetadata>,
    /// Whether the provider stopped generating because it reached its output token limit
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub output_token_limit_reached: bool,
    /// Whether this message is a steer injected into an active run. UI-only:
    /// surfaced as `_meta.goose.steer` so clients can mark the steer boundary
    /// without matching user-visible text. Never sent to providers.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub steer: bool,
    /// Whether this message is a per-turn context event appended by the agent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turn_context: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<MessageUsage>>,
    /// What an operation did to this message, keyed by operation name. Read back
    /// from the persisted conversation so a rebuilt pipeline knows what already
    /// happened. Never sent to providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operations: Option<Box<OperationNotes>>,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        MessageMetadata {
            user_visible: true,
            agent_visible: true,
            inference: None,
            output_token_limit_reached: false,
            steer: false,
            turn_context: false,
            usage: None,
            operations: None,
        }
    }
}

impl MessageMetadata {
    pub fn operation_note(&self, operation: &str, key: &str) -> Option<&serde_json::Value> {
        self.operations.as_ref()?.get(operation)?.get(key)
    }

    pub fn set_operation_note(&mut self, operation: &str, key: &str, value: serde_json::Value) {
        self.operations
            .get_or_insert_with(Box::default)
            .entry(operation.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    /// Create metadata for messages visible only to the agent
    pub fn agent_only() -> Self {
        MessageMetadata {
            user_visible: false,
            agent_visible: true,
            ..Default::default()
        }
    }

    /// Create metadata for messages visible only to the user
    pub fn user_only() -> Self {
        MessageMetadata {
            user_visible: true,
            agent_visible: false,
            ..Default::default()
        }
    }

    /// Create metadata for messages visible to neither user nor agent (archived)
    pub fn invisible() -> Self {
        MessageMetadata {
            user_visible: false,
            agent_visible: false,
            ..Default::default()
        }
    }

    /// Return a copy with agent_visible set to false
    pub fn with_agent_invisible(self) -> Self {
        Self {
            agent_visible: false,
            ..self
        }
    }

    /// Return a copy with user_visible set to false
    pub fn with_user_invisible(self) -> Self {
        Self {
            user_visible: false,
            ..self
        }
    }

    /// Return a copy with agent_visible set to true
    pub fn with_agent_visible(self) -> Self {
        Self {
            agent_visible: true,
            ..self
        }
    }

    /// Return a copy with user_visible set to true
    pub fn with_user_visible(self) -> Self {
        Self {
            user_visible: true,
            ..self
        }
    }

    pub fn with_inference(mut self, inference: InferenceMetadata) -> Self {
        self.inference = Some(inference);
        self
    }

    pub fn with_steer(mut self) -> Self {
        self.steer = true;
        self
    }

    pub fn with_turn_context(mut self) -> Self {
        self.turn_context = true;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenState {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    #[serde(default)]
    pub cache_read_tokens: i32,
    #[serde(default)]
    pub cache_write_tokens: i32,
    pub accumulated_input_tokens: i32,
    pub accumulated_output_tokens: i32,
    pub accumulated_total_tokens: i32,
    #[serde(default)]
    pub accumulated_cache_read_tokens: i32,
    #[serde(default)]
    pub accumulated_cache_write_tokens: i32,
    pub accumulated_cost: Option<f64>,
}
