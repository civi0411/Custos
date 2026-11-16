//! Wire format request and response types for external model providers

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiChatRequestMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiChatRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChatChoiceMessage {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChatChoice {
    pub message: OpenAiChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChatUsage {
    #[serde(default)]
    pub total_tokens: Option<usize>,
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChatResponse {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<OpenAiChatChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiChatUsage>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicMessageItem {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicMessagesRequest {
    pub model: String,
    pub max_tokens: usize,
    pub messages: Vec<AnthropicMessageItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicContentBlock {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicUsage {
    #[serde(default)]
    pub input_tokens: Option<usize>,
    #[serde(default)]
    pub output_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicMessagesResponse {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}
