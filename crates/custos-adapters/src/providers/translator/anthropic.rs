//! Anthropic Protocol Translator
//!
//! Converts between Custos `ModelTurnRequest` / `ModelTurnEvent` and
//! Anthropic Messages REST and SSE Streaming format.
//! Synthesized from AgentGateway (crates/llm/conversion/messages.rs) and 9Router.

use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest, TurnDelta};
use custos_provider::types::conversation::message::MessageContentBlock;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Value>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}

/// Convert Custos `ModelTurnRequest` into Anthropic Messages request payload
pub fn build_anthropic_request(req: &ModelTurnRequest) -> AnthropicMessagesRequest {
    let mut messages = Vec::new();

    for msg in &req.structured_messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut blocks = Vec::new();
        for block in &msg.content {
            match block {
                MessageContentBlock::Text(t) => {
                    blocks.push(AnthropicContentBlock::Text {
                        text: t.text.clone(),
                    });
                }
                _ => {}
            }
        }

        messages.push(AnthropicMessage {
            role: role.to_string(),
            content: blocks,
        });
    }

    let tools = if req.tool_schemas.is_empty() {
        None
    } else {
        Some(
            req.tool_schemas
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters
                    })
                })
                .collect(),
        )
    };

    // If model is Claude 3.7 and thinking is requested
    let thinking = if req.requested_model.contains("claude-3-7-sonnet") {
        Some(json!({
            "type": "enabled",
            "budget_tokens": 1024
        }))
    } else {
        None
    };

    AnthropicMessagesRequest {
        model: req.requested_model.clone(),
        messages,
        max_tokens: 4096,
        system: None,
        tools,
        thinking,
        stream: true,
    }
}

/// Parses an Anthropic SSE event chunk into Custos `ModelTurnEvent`.
pub fn parse_anthropic_sse_event(
    attempt_id: Uuid,
    sequence: u64,
    provider_name: &str,
    event_type: &str,
    data_json: &Value,
) -> Option<ModelTurnEvent> {
    match event_type {
        "content_block_start" => {
            if let Some(content_block) = data_json.get("content_block") {
                if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    let id = content_block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = content_block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    return Some(ModelTurnEvent {
                        attempt_id,
                        sequence,
                        actual_provider: Some(provider_name.to_string()),
                        actual_model: None,
                        actual_transport: Some("anthropic_sse".to_string()),
                        delta: TurnDelta::ToolCall {
                            id,
                            name,
                            arguments_chunk: String::new(),
                        },
                        usage_certainty: None,
                        provider_response_id: None,
                    });
                }
            }
        }
        "content_block_delta" => {
            if let Some(delta) = data_json.get("delta") {
                let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match delta_type {
                    "text_delta" => {
                        let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        return Some(ModelTurnEvent {
                            attempt_id,
                            sequence,
                            actual_provider: Some(provider_name.to_string()),
                            actual_model: None,
                            actual_transport: Some("anthropic_sse".to_string()),
                            delta: TurnDelta::Text {
                                text: text.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        });
                    }
                    "thinking_delta" => {
                        let thinking = delta
                            .get("thinking")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        return Some(ModelTurnEvent {
                            attempt_id,
                            sequence,
                            actual_provider: Some(provider_name.to_string()),
                            actual_model: None,
                            actual_transport: Some("anthropic_sse".to_string()),
                            delta: TurnDelta::Reasoning {
                                text: thinking.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        });
                    }
                    "input_json_delta" => {
                        let partial_json = delta
                            .get("partial_json")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        return Some(ModelTurnEvent {
                            attempt_id,
                            sequence,
                            actual_provider: Some(provider_name.to_string()),
                            actual_model: None,
                            actual_transport: Some("anthropic_sse".to_string()),
                            delta: TurnDelta::ToolCall {
                                id: String::new(),
                                name: None,
                                arguments_chunk: partial_json.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        });
                    }
                    _ => {}
                }
            }
        }
        "message_delta" => {
            if let Some(usage) = data_json.get("usage") {
                let output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                return Some(ModelTurnEvent {
                    attempt_id,
                    sequence,
                    actual_provider: Some(provider_name.to_string()),
                    actual_model: None,
                    actual_transport: Some("anthropic_sse".to_string()),
                    delta: TurnDelta::Usage {
                        input_tokens: 0,
                        output_tokens,
                        cache_read_tokens: None,
                        cache_write_tokens: None,
                    },
                    usage_certainty: Some("known".to_string()),
                    provider_response_id: None,
                });
            }
        }
        "message_stop" => {
            return Some(ModelTurnEvent {
                attempt_id,
                sequence,
                actual_provider: Some(provider_name.to_string()),
                actual_model: None,
                actual_transport: Some("anthropic_sse".to_string()),
                delta: TurnDelta::Completed {
                    finish_reason: Some("stop".to_string()),
                },
                usage_certainty: None,
                provider_response_id: None,
            });
        }
        _ => {}
    }

    None
}
