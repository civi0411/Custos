//! Google Gemini / Vertex AI Protocol Translator
//!
//! Converts between Custos `ModelTurnRequest` / `ModelTurnEvent` and
//! Google Gemini `generateContent` / `streamGenerateContent` API formats.
//! Synthesized from AgentGateway (crates/llm/conversion/vertex_gemini.rs).

use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest, TurnDelta};
use custos_provider::types::conversation::message::MessageContentBlock;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContentRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

/// Convert Custos `ModelTurnRequest` into Gemini content request payload
pub fn build_gemini_request(req: &ModelTurnRequest) -> GeminiContentRequest {
    let mut contents = Vec::new();

    for msg in &req.structured_messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "model",
        };

        let mut parts = Vec::new();
        for block in &msg.content {
            match block {
                MessageContentBlock::Text(t) => {
                    parts.push(GeminiPart::Text {
                        text: t.text.clone(),
                    });
                }
                _ => {}
            }
        }

        if !parts.is_empty() {
            contents.push(GeminiContent {
                role: role.to_string(),
                parts,
            });
        }
    }

    let tools = if req.tool_schemas.is_empty() {
        None
    } else {
        let declarations: Vec<Value> = req
            .tool_schemas
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                })
            })
            .collect();

        Some(vec![json!({
            "functionDeclarations": declarations
        })])
    };

    GeminiContentRequest {
        contents,
        tools,
        generation_config: Some(GeminiGenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: None,
        }),
    }
}

/// Parses a Gemini SSE data chunk (`data: {...}`) into Custos `ModelTurnEvent`s.
pub fn parse_gemini_sse_chunk(
    attempt_id: Uuid,
    sequence: u64,
    provider_name: &str,
    model_name: &str,
    raw_line: &str,
) -> Vec<ModelTurnEvent> {
    let mut events = Vec::new();
    let trimmed = raw_line.trim();

    if !trimmed.starts_with("data:") {
        return events;
    }

    let json_str = trimmed["data:".len()..].trim();
    if json_str.is_empty() || json_str == "[DONE]" {
        return events;
    }

    let Ok(val) = serde_json::from_str::<Value>(json_str) else {
        return events;
    };

    // Candidates content parsing
    if let Some(candidates) = val.get("candidates").and_then(|c| c.as_array()) {
        for candidate in candidates {
            if let Some(parts) = candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
            {
                for part in parts {
                    // Text delta
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        events.push(ModelTurnEvent {
                            attempt_id,
                            sequence,
                            actual_provider: Some(provider_name.to_string()),
                            actual_model: Some(model_name.to_string()),
                            actual_transport: Some("gemini_sse".to_string()),
                            delta: TurnDelta::Text {
                                text: text.to_string(),
                            },
                            usage_certainty: Some("estimated".to_string()),
                            provider_response_id: None,
                        });
                    }

                    // Function call delta
                    if let Some(fn_call) = part.get("functionCall") {
                        if let Some(name) = fn_call.get("name").and_then(|n| n.as_str()) {
                            let args = fn_call
                                .get("args")
                                .map(|a| a.to_string())
                                .unwrap_or_else(|| "{}".to_string());
                            let call_id = Uuid::new_v4().to_string();

                            events.push(ModelTurnEvent {
                                attempt_id,
                                sequence,
                                actual_provider: Some(provider_name.to_string()),
                                actual_model: Some(model_name.to_string()),
                                actual_transport: Some("gemini_sse".to_string()),
                                delta: TurnDelta::ToolCall {
                                    id: call_id.clone(),
                                    name: Some(name.to_string()),
                                    arguments_chunk: args,
                                },
                                usage_certainty: Some("estimated".to_string()),
                                provider_response_id: None,
                            });

                            events.push(ModelTurnEvent {
                                attempt_id,
                                sequence: sequence + 1,
                                actual_provider: Some(provider_name.to_string()),
                                actual_model: Some(model_name.to_string()),
                                actual_transport: Some("gemini_sse".to_string()),
                                delta: TurnDelta::ToolCallComplete { id: call_id },
                                usage_certainty: Some("estimated".to_string()),
                                provider_response_id: None,
                            });
                        }
                    }
                }
            }

            // Finish reason
            if let Some(finish_reason) = candidate.get("finishReason").and_then(|f| f.as_str()) {
                events.push(ModelTurnEvent {
                    attempt_id,
                    sequence: sequence + 2,
                    actual_provider: Some(provider_name.to_string()),
                    actual_model: Some(model_name.to_string()),
                    actual_transport: Some("gemini_sse".to_string()),
                    delta: TurnDelta::Completed {
                        finish_reason: Some(finish_reason.to_string()),
                    },
                    usage_certainty: Some("known".to_string()),
                    provider_response_id: None,
                });
            }
        }
    }

    // Usage metadata
    if let Some(usage) = val.get("usageMetadata") {
        let prompt_tokens = usage
            .get("promptTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let output_tokens = usage
            .get("candidatesTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let cached_tokens = usage
            .get("cachedContentTokenCount")
            .and_then(|t| t.as_u64());

        events.push(ModelTurnEvent {
            attempt_id,
            sequence: sequence + 3,
            actual_provider: Some(provider_name.to_string()),
            actual_model: Some(model_name.to_string()),
            actual_transport: Some("gemini_sse".to_string()),
            delta: TurnDelta::Usage {
                input_tokens: prompt_tokens,
                output_tokens,
                cache_read_tokens: cached_tokens,
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
    fn test_build_gemini_request() {
        let req = ModelTurnRequest {
            task_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            connection_id: "google-conn".to_string(),
            requested_model: "gemini-2.0-flash".to_string(),
            structured_messages: vec![Message::user().with_text("Hello Gemini")],
            tool_schemas: vec![ToolDefinition {
                name: "search".to_string(),
                description: "Search web".to_string(),
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

        let gemini_req = build_gemini_request(&req);
        assert_eq!(gemini_req.contents.len(), 1);
        assert_eq!(gemini_req.contents[0].role, "user");
        assert!(gemini_req.tools.is_some());
    }

    #[test]
    fn test_parse_gemini_sse_chunk() {
        let raw = r#"data: {"candidates":[{"content":{"parts":[{"text":"Hi there!"}],"role":"model"},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":3}}"#;
        let events = parse_gemini_sse_chunk(
            Uuid::new_v4(),
            1,
            "google",
            "gemini-2.0-flash",
            raw,
        );

        assert_eq!(events.len(), 3);
        assert!(matches!(events[0].delta, TurnDelta::Text { .. }));
        assert!(matches!(events[1].delta, TurnDelta::Completed { .. }));
        assert!(matches!(events[2].delta, TurnDelta::Usage { .. }));
    }
}
