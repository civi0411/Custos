//! Provider Request & Response Models
//!
//! Complies with schemas/protocol/provider-request.v1.schema.json

use crate::events::ToolCall;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub request_id: String,
    pub task_id: String,
    pub span_num: u32,
    pub prompt: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

impl ProviderRequest {
    /// Creates a simple request with reasonable defaults.
    pub fn simple(prompt: impl Into<String>) -> Self {
        Self {
            request_id: custos_domain::new_id("req"),
            task_id: "default_task".to_string(),
            span_num: 1,
            prompt: prompt.into(),
            model: "default".to_string(),
            provider_account_id: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            tools: None,
        }
    }

    /// Full constructor for structured pipeline invocation.
    pub fn new(
        request_id: impl Into<String>,
        task_id: impl Into<String>,
        span_num: u32,
        prompt: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            task_id: task_id.into(),
            span_num,
            prompt: prompt.into(),
            model: model.into(),
            provider_account_id: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            tools: None,
        }
    }
}

/// Backward compatibility alias
pub type ModelRequest = ProviderRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub tokens_used: usize,
    #[serde(default)]
    pub input_tokens: Option<usize>,
    #[serde(default)]
    pub output_tokens: Option<usize>,
    #[serde(default)]
    pub total_tokens: Option<usize>,
    pub model_id: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

impl ModelResponse {
    pub fn with_usage(mut self, input: Option<usize>, output: Option<usize>, total: Option<usize>) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self.total_tokens = total.or_else(|| input.zip(output).map(|(i, o)| i.saturating_add(o)));
        self
    }

    pub fn text(
        content: impl Into<String>,
        model_id: impl Into<String>,
        tokens_used: usize,
    ) -> Self {
        Self {
            content: content.into(),
            tokens_used,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            model_id: model_id.into(),
            tool_calls: Vec::new(),
        }
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;
    #[test]
    fn measured_zero_is_distinct_from_unknown_and_old_requests_decode() {
        let unknown = ModelResponse::text("answer", "test", 42);
        assert_eq!(unknown.total_tokens,None);
        let measured = unknown.with_usage(Some(7),Some(3),None);
        assert_eq!(measured.total_tokens,Some(10));
        assert_eq!(ModelResponse::text("", "test",0).with_usage(Some(0),Some(0),None).total_tokens,Some(0));
        let request: ProviderRequest = serde_json::from_value(serde_json::json!({"request_id":"req","task_id":"task","span_num":1,"prompt":"hello","model":"test"})).unwrap();
        assert_eq!(request.provider_account_id,None);
    }
}
