//! Model turn SSE stream decoder and observer.

use super::types::{actual_model, count, Wire};
use custos_domain::DomainError;
use custos_provider::turn::{ModelTurnEvent, TurnDelta};
use serde_json::Value;
use uuid::Uuid;

#[derive(Default)]
pub struct ModelTurnStreamDecoder {
    pub completed: bool,
    pub sequence: u64,
}

impl ModelTurnStreamDecoder {
    fn make_event(
        &mut self,
        attempt_id: Uuid,
        actual_provider: &str,
        actual_model: &str,
        delta: TurnDelta,
    ) -> ModelTurnEvent {
        self.sequence += 1;
        ModelTurnEvent {
            attempt_id,
            sequence: self.sequence,
            actual_provider: Some(actual_provider.to_string()),
            actual_model: Some(actual_model.to_string()),
            actual_transport: Some("turn_sse".to_string()),
            delta,
            usage_certainty: Some("estimated".to_string()),
            provider_response_id: None,
        }
    }

    pub fn decode(
        &mut self,
        attempt_id: Uuid,
        actual_provider: &str,
        fallback_model: &str,
        wire: Wire,
        data: &str,
    ) -> Result<Vec<ModelTurnEvent>, DomainError> {
        if data.trim() == "[DONE]" {
            self.completed = true;
            return Ok(vec![]);
        }
        let value: Value = serde_json::from_str(data)
            .map_err(|_| DomainError::Validation("Malformed provider stream frame".into()))?;
        if value.get("error").is_some()
            || matches!(
                value.get("type").and_then(Value::as_str),
                Some("error" | "response.failed" | "response.incomplete")
            )
        {
            return Err(DomainError::Validation(
                "Provider reported stream failure".into(),
            ));
        }

        let mut events = Vec::new();
        let model = actual_model(wire, &value).unwrap_or_else(|| fallback_model.to_string());

        match wire {
            Wire::Claude => {
                let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
                if kind == "content_block_delta" {
                    let delta_type = value.pointer("/delta/type").and_then(Value::as_str);
                    if delta_type == Some("thinking_delta") {
                        if let Some(thinking) =
                            value.pointer("/delta/thinking").and_then(Value::as_str)
                        {
                            if !thinking.is_empty() {
                                events.push(self.make_event(
                                    attempt_id,
                                    actual_provider,
                                    &model,
                                    TurnDelta::Reasoning {
                                        text: thinking.to_string(),
                                    },
                                ));
                            }
                        }
                    } else if delta_type == Some("text_delta") {
                        if let Some(text) = value.pointer("/delta/text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                events.push(self.make_event(
                                    attempt_id,
                                    actual_provider,
                                    &model,
                                    TurnDelta::Text {
                                        text: text.to_string(),
                                    },
                                ));
                            }
                        }
                    } else if delta_type == Some("input_json_delta") {
                        if let Some(partial) =
                            value.pointer("/delta/partial_json").and_then(Value::as_str)
                        {
                            let idx = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                            events.push(self.make_event(
                                attempt_id,
                                actual_provider,
                                &model,
                                TurnDelta::ToolCall {
                                    id: format!("block-{idx}"),
                                    name: None,
                                    arguments_chunk: partial.to_string(),
                                },
                            ));
                        }
                    }
                } else if kind == "content_block_start" {
                    if value.pointer("/content_block/type").and_then(Value::as_str)
                        == Some("tool_use")
                    {
                        let block = &value["content_block"];
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::ToolCall {
                                id: block["id"].as_str().unwrap_or("tool").to_string(),
                                name: block["name"].as_str().map(String::from),
                                arguments_chunk: String::new(),
                            },
                        ));
                    }
                } else if kind == "content_block_stop" {
                    let idx = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::ToolCallComplete {
                            id: format!("block-{idx}"),
                        },
                    ));
                } else if kind == "message_delta" || kind == "message_stop" {
                    let input = count(&value, "/message/usage/input_tokens")
                        .or_else(|| count(&value, "/usage/input_tokens"));
                    let output = count(&value, "/usage/output_tokens")
                        .or_else(|| count(&value, "/message/usage/output_tokens"));
                    let cache_read = count(&value, "/message/usage/cache_read_input_tokens")
                        .or_else(|| count(&value, "/usage/cache_read_input_tokens"));
                    let cache_write = count(&value, "/message/usage/cache_creation_input_tokens")
                        .or_else(|| count(&value, "/usage/cache_creation_input_tokens"));
                    if input.is_some()
                        || output.is_some()
                        || cache_read.is_some()
                        || cache_write.is_some()
                    {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Usage {
                                input_tokens: input.unwrap_or(0) as u64,
                                output_tokens: output.unwrap_or(0) as u64,
                                cache_read_tokens: cache_read.map(|c| c as u64),
                                cache_write_tokens: cache_write.map(|c| c as u64),
                            },
                        ));
                    }
                    if kind == "message_stop" {
                        self.completed = true;
                        let stop_reason = value
                            .pointer("/delta/stop_reason")
                            .and_then(Value::as_str)
                            .unwrap_or("stop");
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Completed {
                                finish_reason: Some(stop_reason.to_string()),
                            },
                        ));
                    }
                }
            }
            Wire::OpenAi => {
                if let Some(reasoning) = value
                    .pointer("/choices/0/delta/reasoning_content")
                    .or_else(|| value.pointer("/choices/0/delta/reasoning"))
                    .and_then(Value::as_str)
                {
                    if !reasoning.is_empty() {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Reasoning {
                                text: reasoning.to_string(),
                            },
                        ));
                    }
                }
                if let Some(text) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                {
                    if !text.is_empty() {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Text {
                                text: text.to_string(),
                            },
                        ));
                    }
                }
                if let Some(calls) = value
                    .pointer("/choices/0/delta/tool_calls")
                    .and_then(Value::as_array)
                {
                    for call in calls {
                        let id = call
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let name = call
                            .pointer("/function/name")
                            .and_then(Value::as_str)
                            .map(String::from);
                        let chunk = call
                            .pointer("/function/arguments")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::ToolCall {
                                id,
                                name,
                                arguments_chunk: chunk,
                            },
                        ));
                    }
                }
                if let (Some(input), Some(output)) = (
                    count(&value, "/usage/prompt_tokens"),
                    count(&value, "/usage/completion_tokens"),
                ) {
                    let cache_read = count(&value, "/usage/prompt_tokens_details/cached_tokens");
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Usage {
                            input_tokens: input as u64,
                            output_tokens: output as u64,
                            cache_read_tokens: cache_read.map(|c| c as u64),
                            cache_write_tokens: None,
                        },
                    ));
                }
                if let Some(reason) = value
                    .pointer("/choices/0/finish_reason")
                    .and_then(Value::as_str)
                {
                    self.completed = true;
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Completed {
                            finish_reason: Some(reason.to_string()),
                        },
                    ));
                }
            }
            Wire::Google => {
                let v = value.get("response").unwrap_or(&value);
                if let Some(parts) = v
                    .pointer("/candidates/0/content/parts")
                    .and_then(Value::as_array)
                {
                    for part in parts {
                        if part.get("thought").and_then(Value::as_bool) == Some(true) {
                            if let Some(thought) = part.get("text").and_then(Value::as_str) {
                                if !thought.is_empty() {
                                    events.push(self.make_event(
                                        attempt_id,
                                        actual_provider,
                                        &model,
                                        TurnDelta::Reasoning {
                                            text: thought.to_string(),
                                        },
                                    ));
                                }
                            }
                        } else if let Some(text) = part.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                events.push(self.make_event(
                                    attempt_id,
                                    actual_provider,
                                    &model,
                                    TurnDelta::Text {
                                        text: text.to_string(),
                                    },
                                ));
                            }
                        } else if let Some(func) = part.get("functionCall") {
                            events.push(self.make_event(
                                attempt_id,
                                actual_provider,
                                &model,
                                TurnDelta::ToolCall {
                                    id: "gemini_call".to_string(),
                                    name:
                                        func.get("name").and_then(Value::as_str).map(String::from),
                                    arguments_chunk:
                                        func.get("args").map(|a| a.to_string()).unwrap_or_default(),
                                },
                            ));
                        }
                    }
                }
                if let (Some(input), Some(output)) = (
                    count(v, "/usageMetadata/promptTokenCount"),
                    count(v, "/usageMetadata/candidatesTokenCount"),
                ) {
                    let cache_read = count(v, "/usageMetadata/cachedContentTokenCount");
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Usage {
                            input_tokens: input as u64,
                            output_tokens: output as u64,
                            cache_read_tokens: cache_read.map(|c| c as u64),
                            cache_write_tokens: None,
                        },
                    ));
                }
                if let Some(reason) = v
                    .pointer("/candidates/0/finishReason")
                    .and_then(Value::as_str)
                {
                    self.completed = true;
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Completed {
                            finish_reason: Some(reason.to_string()),
                        },
                    ));
                }
            }
            Wire::Ollama => {
                if let Some(thinking) = value.pointer("/message/thinking").and_then(Value::as_str) {
                    if !thinking.is_empty() {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Reasoning {
                                text: thinking.to_string(),
                            },
                        ));
                    }
                }
                if let Some(text) = value.pointer("/message/content").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Text {
                                text: text.to_string(),
                            },
                        ));
                    }
                }
                if value.get("done").and_then(Value::as_bool) == Some(true) {
                    self.completed = true;
                    let input = count(&value, "/prompt_eval_count").unwrap_or(0) as u64;
                    let output = count(&value, "/eval_count").unwrap_or(0) as u64;
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Usage {
                            input_tokens: input,
                            output_tokens: output,
                            cache_read_tokens: None,
                            cache_write_tokens: None,
                        },
                    ));
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Completed {
                            finish_reason: Some("stop".to_string()),
                        },
                    ));
                }
            }
            Wire::Codex => {
                if let Some(delta) = value.pointer("/delta").and_then(Value::as_str) {
                    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
                    if kind.contains("reasoning") {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Reasoning {
                                text: delta.to_string(),
                            },
                        ));
                    } else {
                        events.push(self.make_event(
                            attempt_id,
                            actual_provider,
                            &model,
                            TurnDelta::Text {
                                text: delta.to_string(),
                            },
                        ));
                    }
                }
                if value.get("type").and_then(Value::as_str) == Some("response.completed") {
                    self.completed = true;
                    let input = count(&value, "/response/usage/input_tokens").unwrap_or(0) as u64;
                    let output = count(&value, "/response/usage/output_tokens").unwrap_or(0) as u64;
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Usage {
                            input_tokens: input,
                            output_tokens: output,
                            cache_read_tokens: None,
                            cache_write_tokens: None,
                        },
                    ));
                    events.push(self.make_event(
                        attempt_id,
                        actual_provider,
                        &model,
                        TurnDelta::Completed {
                            finish_reason: Some("stop".to_string()),
                        },
                    ));
                }
            }
        }

        Ok(events)
    }
}
