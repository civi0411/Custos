//! Provider Events (Streaming & Asynchronous Interaction)
//!
//! Complies with schemas/protocol/provider-event.v1.schema.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderEventType {
    Chunk,
    ToolCall,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEvent {
    pub request_id: String,
    pub event_type: ProviderEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ProviderEvent {
    pub fn chunk(request_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            event_type: ProviderEventType::Chunk,
            text: Some(text.into()),
            tool_calls: None,
            usage: None,
            error: None,
        }
    }

    pub fn tool_call(request_id: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            request_id: request_id.into(),
            event_type: ProviderEventType::ToolCall,
            text: None,
            tool_calls: Some(tool_calls),
            usage: None,
            error: None,
        }
    }

    pub fn completed(
        request_id: impl Into<String>,
        text: Option<String>,
        usage: Option<TokenUsage>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            event_type: ProviderEventType::Completed,
            text,
            tool_calls: None,
            usage,
            error: None,
        }
    }

    pub fn failed(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            event_type: ProviderEventType::Failed,
            text: None,
            tool_calls: None,
            usage: None,
            error: Some(error.into()),
        }
    }
}
