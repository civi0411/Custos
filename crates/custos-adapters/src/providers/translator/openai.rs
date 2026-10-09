//! OpenAI Protocol Translator
//!
//! Converts between Custos `ModelTurnRequest` / `ModelTurnEvent` and
//! OpenAI Chat Completions REST and SSE Streaming format.
//! Synthesized from AgentGateway (crates/llm/conversion/openai_compat.rs).

use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest, TurnDelta};
use custos_provider::types::conversation::message::MessageContentBlock;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Convert Custos `ModelTurnRequest` into OpenAI request payload
pub fn build_openai_request(req: &ModelTurnRequest) -> OpenAiChatRequest {
    let mut messages = Vec::new();

    for msg in &req.structured_messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut text_parts = Vec::new();
        for block in &msg.content {
            match block {
                MessageContentBlock::Text(t) => {
                    text_parts.push(t.text.clone());
                }
                _ => {}
            }
        }

        messages.push(OpenAiMessage {
            role: role.to_string(),
            content: Value::String(text_parts.join("\n")),
            name: None,
            tool_call_id: None,
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
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect(),
        )
    };

    OpenAiChatRequest {
        model: req.requested_model.clone(),
        messages,
        tools,
        temperature: Some(0.7),
        stream: true,
        stream_options: Some(json!({ "include_usage": true })),
    }
}

/// Parses an OpenAI SSE data chunk (`data: {...}`) into Custos `ModelTurnEvent`s.
pub fn parse_openai_sse_chunk(
    attempt_id: Uuid,
    sequence: u64,
    provider_name: &str,
    raw_line: &str,
) -> Option<ModelTurnEvent> {
    let trimmed = raw_line.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }
    let data_str = trimmed.strip_prefix("data:")?.trim();
    if data_str == "[DONE]" {
        return Some(ModelTurnEvent {
            attempt_id,
            sequence,
            actual_provider: Some(provider_name.to_string()),
            actual_model: None,
            actual_transport: Some("openai_sse".to_string()),
            delta: TurnDelta::Completed {
                finish_reason: Some("stop".to_string()),
            },
            usage_certainty: None,
            provider_response_id: None,
        });
    }

    let parsed: Value = serde_json::from_str(data_str).ok()?;
    let model = parsed.get("model").and_then(|v| v.as_str()).map(String::from);
    let resp_id = parsed.get("id").and_then(|v| v.as_str()).map(String::from);

    // Check usage block
    if let Some(usage) = parsed.get("usage") {
        let prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let completion_tokens = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return Some(ModelTurnEvent {
            attempt_id,
            sequence,
            actual_provider: Some(provider_name.to_string()),
            actual_model: model,
            actual_transport: Some("openai_sse".to_string()),
            delta: TurnDelta::Usage {
                input_tokens: prompt_tokens,
                output_tokens: completion_tokens,
                cache_read_tokens: None,
                cache_write_tokens: None,
            },
            usage_certainty: Some("known".to_string()),
            provider_response_id: resp_id,
        });
    }

    // Check choices delta
    if let Some(choice) = parsed.get("choices").and_then(|c| c.get(0)) {
        if let Some(delta) = choice.get("delta") {
            // Reasoning content (o-series, DeepSeek R1)
            if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                if !reasoning.is_empty() {
                    return Some(ModelTurnEvent {
                        attempt_id,
                        sequence,
                        actual_provider: Some(provider_name.to_string()),
                        actual_model: model,
                        actual_transport: Some("openai_sse".to_string()),
                        delta: TurnDelta::Reasoning {
                            text: reasoning.to_string(),
                        },
                        usage_certainty: None,
                        provider_response_id: resp_id,
                    });
                }
            }

            // Standard text delta
            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                if !content.is_empty() {
                    return Some(ModelTurnEvent {
                        attempt_id,
                        sequence,
                        actual_provider: Some(provider_name.to_string()),
                        actual_model: model,
                        actual_transport: Some("openai_sse".to_string()),
                        delta: TurnDelta::Text {
                            text: content.to_string(),
                        },
                        usage_certainty: None,
                        provider_response_id: resp_id,
                    });
                }
            }

            // Tool call delta
            if let Some(tc_array) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                if let Some(tc) = tc_array.get(0) {
                    let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let name = tc
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .map(String::from);
                    let args = tc
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                        .unwrap_or("")
                        .to_string();

                    return Some(ModelTurnEvent {
                        attempt_id,
                        sequence,
                        actual_provider: Some(provider_name.to_string()),
                        actual_model: model,
                        actual_transport: Some("openai_sse".to_string()),
                        delta: TurnDelta::ToolCall {
                            id,
                            name,
                            arguments_chunk: args,
                        },
                        usage_certainty: None,
                        provider_response_id: resp_id,
                    });
                }
            }
        }
    }

    None
}
