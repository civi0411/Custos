use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use crate::thinking::ThinkingEffort;

use super::types::ModelConfig;

/// True when the model should use the OpenAI Responses API.
///
/// The Responses API is backwards-compatible with all OpenAI reasoning
/// models, so every `o`-series (`o1`, `o3`, `o4`, …), `gpt-5`, and `gpt-6` variant
/// routes here. The matcher intentionally scans the full model identifier so
/// hosted aliases like `databricks-gpt-5.4`, `goose-o3-mini`, or
/// `headless-goose-o3-mini` work without provider-specific normalization.
pub fn is_openai_responses_model(model_name: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[-/])(?:o\d+(?:$|-)|gpt-(?:5|6)(?:$|[-.]))").unwrap()
    });
    re.is_match(model_name)
}

pub fn extract_reasoning_effort(model_name: &str) -> (String, Option<String>) {
    if !is_openai_responses_model(model_name) {
        return (model_name.to_string(), None);
    }

    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^(?P<base>.+)-(?P<effort>none|low|medium|high|xhigh)$").unwrap()
    });

    if let Some(captures) = re.captures(model_name) {
        let base = captures["base"].to_string();
        let effort = captures["effort"].to_ascii_lowercase();
        return (base, Some(effort));
    }

    (model_name.to_string(), None)
}

/// Returns whether an xAI Chat Completions model accepts `reasoning_effort`.
pub fn supports_xai_reasoning_effort(model_name: &str) -> bool {
    let model_name = model_name.to_ascii_lowercase();

    model_name.starts_with("grok-4.5")
        || model_name.starts_with("grok-4.3")
        || model_name.starts_with("grok-3-mini")
}

/// Returns whether an xAI model performs server-side reasoning.
pub fn is_xai_reasoning_model(model_name: &str) -> bool {
    let model_name = model_name.to_ascii_lowercase();

    if model_name.contains("non-reasoning") || model_name.contains("non_reasoning") {
        return false;
    }

    supports_xai_reasoning_effort(&model_name)
        || model_name.starts_with("grok-4.20")
        || model_name.starts_with("grok-4-0709")
        || model_name.starts_with("grok-4-fast-reasoning")
        || model_name.starts_with("grok-4-1-fast-reasoning")
}

impl ModelConfig {
    pub fn with_thinking_effort(mut self, effort: ThinkingEffort) -> Self {
        let params = self.request_params.get_or_insert_with(HashMap::new);
        params.insert(
            "thinking_effort".to_string(),
            serde_json::json!(effort.to_string()),
        );
        self
    }

    pub fn with_default_thinking_effort(mut self, effort: Option<ThinkingEffort>) -> Self {
        if self.request_param::<String>("thinking_effort").is_none() {
            if let Some(effort) = effort {
                self = self.with_thinking_effort(effort);
            }
        }
        self
    }

    pub fn is_openai_reasoning_model(&self) -> bool {
        is_openai_responses_model(&self.model_name)
    }

    pub fn is_reasoning_model(&self) -> bool {
        if let Some(reasoning) = self.reasoning {
            return reasoning;
        }

        self.is_openai_reasoning_model()
            || self.model_name.to_lowercase().contains("claude")
            || Self::is_gemini3_reasoning_model_name(&self.model_name)
            || self.is_glm_5_3_reasoning_model()
            || self.is_kimi_k3_reasoning_model()
            || is_xai_reasoning_model(&self.model_name)
    }

    pub fn is_glm_5_3_reasoning_model(&self) -> bool {
        let name = self
            .model_name
            .splitn(3, '.')
            .nth(2)
            .unwrap_or(&self.model_name);
        let lower = name.to_lowercase();
        let segments: Vec<_> = lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|segment| !segment.is_empty())
            .collect();
        segments
            .windows(3)
            .any(|segments| segments == ["glm", "5", "3"])
    }

    pub fn is_kimi_k3_reasoning_model(&self) -> bool {
        let name = self
            .model_name
            .splitn(3, '.')
            .nth(2)
            .unwrap_or(&self.model_name);
        let lower = name.to_lowercase();
        let segments: Vec<_> = lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|segment| !segment.is_empty())
            .collect();
        segments
            .windows(2)
            .any(|segments| segments == ["kimi", "k3"])
    }

    fn is_gemini3_reasoning_model_name(model_name: &str) -> bool {
        let lower = model_name.to_lowercase();
        lower.starts_with("gemini-3") || lower.contains("/gemini-3") || lower.contains("-gemini-3")
    }

    pub fn normalize_effort_suffix(&mut self) {
        if !self.is_openai_reasoning_model() && !supports_xai_reasoning_effort(&self.model_name) {
            return;
        }
        let parts: Vec<&str> = self.model_name.split('-').collect();
        let last = match parts.last() {
            Some(l) => *l,
            None => return,
        };
        let effort = match last {
            "none" => ThinkingEffort::Off,
            "low" => ThinkingEffort::Low,
            "medium" => ThinkingEffort::Medium,
            "high" => ThinkingEffort::High,
            "xhigh" => ThinkingEffort::Max,
            _ => return,
        };
        self.model_name = parts[..parts.len() - 1].join("-");
        let has_explicit_effort = self
            .request_params
            .as_ref()
            .and_then(|p| p.get("thinking_effort"))
            .is_some();
        if !has_explicit_effort {
            let params = self.request_params.get_or_insert_with(HashMap::new);
            params.insert(
                "thinking_effort".to_string(),
                serde_json::json!(effort.to_string()),
            );
        }
    }

    pub fn thinking_effort(&self) -> Option<ThinkingEffort> {
        self.request_param::<String>("thinking_effort")
            .and_then(|s| s.parse::<ThinkingEffort>().ok())
    }

    pub fn with_prompt_cache_disabled(self) -> Self {
        self.with_merged_request_params(HashMap::from([(
            "disable_prompt_cache".to_string(),
            Value::Bool(true),
        )]))
    }

    pub fn prompt_cache_disabled(&self) -> bool {
        self.request_param::<bool>("disable_prompt_cache")
            .unwrap_or(false)
    }

    /// Set the prompt-cache TTL requested from providers that support one
    /// (currently the Anthropic message format). Valid values are "5m" and
    /// "1h"; absent means the provider default (5m).
    pub fn with_cache_ttl(self, ttl: &str) -> Self {
        self.with_merged_request_params(HashMap::from([(
            "cache_ttl".to_string(),
            Value::String(ttl.to_string()),
        )]))
    }

    /// Remove any prompt-cache TTL request parameter. The TTL is
    /// configuration state, not session state: callers that resume a
    /// persisted config drop the stored value and re-derive it from the
    /// current configuration so a clamped run never sticks to the session.
    pub fn without_cache_ttl(mut self) -> Self {
        if let Some(params) = self.request_params.as_mut() {
            params.remove("cache_ttl");
            if params.is_empty() {
                self.request_params = None;
            }
        }
        self
    }

    /// Clamp the prompt-cache TTL back to the provider default (5m).
    /// Burst-only surfaces (headless runs, subagents, scheduled recipes) call
    /// this so a user-level 1h opt-in never pays the 2x cache-write premium on
    /// workloads that finish in one burst and cannot idle.
    pub fn with_cache_ttl_clamped(self) -> Self {
        if self.cache_ttl().is_some_and(|ttl| ttl != "5m") {
            self.with_cache_ttl("5m")
        } else {
            self
        }
    }

    pub fn cache_ttl(&self) -> Option<String> {
        self.request_param::<String>("cache_ttl")
    }
}

