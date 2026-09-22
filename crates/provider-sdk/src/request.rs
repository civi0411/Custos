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
            request_id: custos_core_domain::new_id("req"),
            task_id: "default_task".to_string(),
            span_num: 1,
            prompt: prompt.into(),
            model: "default".to_string(),
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
    pub model_id: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

impl ModelResponse {
    pub fn text(
        content: impl Into<String>,
        model_id: impl Into<String>,
        tokens_used: usize,
    ) -> Self {
        Self {
            content: content.into(),
            tokens_used,
            model_id: model_id.into(),
            tool_calls: Vec::new(),
        }
    }
}
