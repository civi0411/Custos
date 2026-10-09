//! AWS Bedrock Converse API Protocol Translator
//!
//! Converts between Custos `ModelTurnRequest` / `ModelTurnEvent` and
//! AWS Bedrock `converse` / `converse-stream` API format.
//! Synthesized from AgentGateway (crates/llm/src/conversion/bedrock.rs).

use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest, TurnDelta};
use custos_provider::types::conversation::message::MessageContentBlock;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockConverseRequest {
    pub messages: Vec<BedrockMessage>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<BedrockToolConfig>,
    #[serde(rename = "inferenceConfig", skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<BedrockInferenceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockMessage {
    pub role: String,
    pub content: Vec<BedrockContentBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BedrockContentBlock {
    Text { text: String },
    ToolUse {
        #[serde(rename = "toolUse")]
        tool_use: BedrockToolUse,
    },
    ToolResult {
        #[serde(rename = "toolResult")]
        tool_result: BedrockToolResult,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockToolUse {
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockToolResult {
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub content: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockToolConfig {
    pub tools: Vec<BedrockToolDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockToolDefinition {
    #[serde(rename = "toolSpec")]
    pub tool_spec: BedrockToolSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockToolSpec {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockInferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Convert Custos `ModelTurnRequest` into AWS Bedrock Converse request payload
pub fn build_bedrock_request(req: &ModelTurnRequest) -> BedrockConverseRequest {
    let mut messages = Vec::new();

    for msg in &req.structured_messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut content = Vec::new();
        for block in &msg.content {
            match block {
                MessageContentBlock::Text(t) => {
                    content.push(BedrockContentBlock::Text {
                        text: t.text.clone(),
                    });
                }
                _ => {}
            }
        }

        if !content.is_empty() {
            messages.push(BedrockMessage {
                role: role.to_string(),
                content,
            });
        }
    }

    let tool_config = if req.tool_schemas.is_empty() {
        None
    } else {
        let tools: Vec<BedrockToolDefinition> = req
            .tool_schemas
            .iter()
            .map(|t| BedrockToolDefinition {
                tool_spec: BedrockToolSpec {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: json!({ "json": t.parameters }),
                },
            })
            .collect();

        Some(BedrockToolConfig { tools })
    };

    BedrockConverseRequest {
        messages,
        tool_config,
        inference_config: Some(BedrockInferenceConfig {
            temperature: Some(0.7),
            max_tokens: None,
        }),
    }
}

/// Parses a Bedrock SSE streaming event into Custos `ModelTurnEvent`s.
pub fn parse_bedrock_sse_event(
    attempt_id: Uuid,
    sequence: u64,
    provider_name: &str,
    model_name: &str,
    raw_json: &str,
) -> Vec<ModelTurnEvent> {
    let mut events = Vec::new();

    let Ok(val) = serde_json::from_str::<Value>(raw_json) else {
        return events;
    };

    // 1. Text or reasoning delta
    if let Some(delta) = val.get("contentBlockDelta").and_then(|d| d.get("delta")) {
        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
            events.push(ModelTurnEvent {
                attempt_id,
                sequence,
                actual_provider: Some(provider_name.to_string()),
                actual_model: Some(model_name.to_string()),
                actual_transport: Some("bedrock_stream".to_string()),
                delta: TurnDelta::Text {
                    text: text.to_string(),
                },
                usage_certainty: Some("estimated".to_string()),
                provider_response_id: None,
            });
        }

        // Tool use delta
        if let Some(tool_use) = delta.get("toolUse") {
            let input_chunk = tool_use
                .get("input")
                .and_then(|i| i.as_str())
                .unwrap_or("{}");
            let call_id = Uuid::new_v4().to_string();

            events.push(ModelTurnEvent {
                attempt_id,
                sequence,
                actual_provider: Some(provider_name.to_string()),
                actual_model: Some(model_name.to_string()),
                actual_transport: Some("bedrock_stream".to_string()),
                delta: TurnDelta::ToolCall {
                    id: call_id.clone(),
                    name: None,
                    arguments_chunk: input_chunk.to_string(),
                },
                usage_certainty: Some("estimated".to_string()),
                provider_response_id: None,
            });
        }
    }

    // 2. Terminal message stop event
    if let Some(stop) = val.get("messageStop") {
        let reason = stop
            .get("stopReason")
            .and_then(|r| r.as_str())
            .unwrap_or("end_turn");

        events.push(ModelTurnEvent {
            attempt_id,
            sequence: sequence + 1,
            actual_provider: Some(provider_name.to_string()),
            actual_model: Some(model_name.to_string()),
            actual_transport: Some("bedrock_stream".to_string()),
            delta: TurnDelta::Completed {
                finish_reason: Some(reason.to_string()),
            },
            usage_certainty: Some("known".to_string()),
            provider_response_id: None,
        });
    }

    // 3. Usage metadata event
    if let Some(metadata) = val.get("metadata").and_then(|m| m.get("usage")) {
        let input_tokens = metadata
            .get("inputTokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let output_tokens = metadata
            .get("outputTokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        events.push(ModelTurnEvent {
            attempt_id,
            sequence: sequence + 2,
            actual_provider: Some(provider_name.to_string()),
            actual_model: Some(model_name.to_string()),
            actual_transport: Some("bedrock_stream".to_string()),
            delta: TurnDelta::Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens: None,
                cache_write_tokens: None,
            },
            usage_certainty: Some("known".to_string()),
            provider_response_id: None,
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_provider::tool::ToolDefinition;
    use custos_provider::types::conversation::message::Message;

    #[test]
    fn test_build_bedrock_request() {
        let req = ModelTurnRequest {
            task_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            connection_id: "aws-bedrock".to_string(),
            requested_model: "anthropic.claude-3-7-sonnet".to_string(),
            structured_messages: vec![Message::user().with_text("Hello Bedrock")],
            tool_schemas: vec![ToolDefinition {
                name: "query_db".to_string(),
                description: "Run SQL query".to_string(),
                parameters: json!({"type": "object"}),
            }],
            required_capabilities: vec![],
            source_scope_digest: None,
            privacy_class: "standard".to_string(),
            budget_reservation: None,
            deadline: None,
            cancellation_token: None,
            decoding_options: None,
        };

        let bedrock_req = build_bedrock_request(&req);
        assert_eq!(bedrock_req.messages.len(), 1);
        assert_eq!(bedrock_req.messages[0].role, "user");
        assert!(bedrock_req.tool_config.is_some());
    }

    #[test]
    fn test_parse_bedrock_sse_event() {
        let raw = r#"{"contentBlockDelta":{"delta":{"text":"Hello from Bedrock!"}}}"#;
        let events = parse_bedrock_sse_event(
            Uuid::new_v4(),
            1,
            "aws_bedrock",
            "anthropic.claude-3-7-sonnet",
            raw,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].delta, TurnDelta::Text { .. }));
    }
}
