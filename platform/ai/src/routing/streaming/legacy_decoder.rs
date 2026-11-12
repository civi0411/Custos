//! Incremental provider framing decoder for legacy provider events.

use super::types::{actual_model, count, event, Wire};
use custos_domain::DomainError;
use custos_provider::{ProviderEvent, ProviderEventType, TokenUsage, ToolCall};
use serde_json::Value;

#[derive(Default)]
pub(crate) struct Decoder {
    pub(crate) model: Option<String>,
    pub(crate) text: String,
    pub(crate) input: Option<usize>,
    pub(crate) output: Option<usize>,
    pub(crate) completed: bool,
}

impl Decoder {
    pub(crate) fn completion(self, id: &str) -> ProviderEvent {
        let mut event = ProviderEvent::completed(
            id,
            Some(self.text),
            Some(TokenUsage {
                input_tokens: self.input,
                output_tokens: self.output,
            }),
        );
        event.model_id = self.model;
        event.tokens_used = self
            .input
            .zip(self.output)
            .map(|(a, b)| a.saturating_add(b));
        event
    }

    pub(crate) fn decode(
        &mut self,
        id: &str,
        wire: Wire,
        data: &str,
    ) -> Result<Vec<ProviderEvent>, DomainError> {
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
        let mut events = vec![];
        let previous_usage = (self.input, self.output);
        self.model = actual_model(wire, &value).or(self.model.take());

        match wire {
            Wire::OpenAi => {
                if let Some(text) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                {
                    self.text.push_str(text);
                    events.push(ProviderEvent::chunk(id, text));
                }
                if value
                    .pointer("/choices/0/delta/reasoning_content")
                    .is_some()
                {
                    events.push(event(id, ProviderEventType::Thinking, None));
                }
                self.input = count(&value, "/usage/prompt_tokens").or(self.input);
                self.output = count(&value, "/usage/completion_tokens").or(self.output);
                if value
                    .pointer("/choices/0/finish_reason")
                    .is_some_and(|v| !v.is_null())
                {
                    self.completed = true;
                }
                if let Some(calls) = value
                    .pointer("/choices/0/delta/tool_calls")
                    .and_then(Value::as_array)
                {
                    for call in calls {
                        let mut e = event(id, ProviderEventType::ToolCall, None);
                        e.tool_calls = Some(vec![ToolCall {
                            call_id: call
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or("pending")
                                .into(),
                            name: call
                                .pointer("/function/name")
                                .and_then(Value::as_str)
                                .unwrap_or("tool")
                                .into(),
                            arguments: serde_json::json!({"partial_json":call.pointer("/function/arguments")}),
                        }]);
                        events.push(e);
                    }
                }
            }
            Wire::Claude => {
                let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
                if kind == "content_block_delta" {
                    if let Some(text) =
                        value
                            .pointer("/delta/text")
                            .and_then(Value::as_str)
                            .filter(|_| {
                                value.pointer("/delta/type").and_then(Value::as_str)
                                    == Some("text_delta")
                            })
                    {
                        self.text.push_str(text);
                        events.push(ProviderEvent::chunk(id, text));
                    }
                    if value.pointer("/delta/type").and_then(Value::as_str)
                        == Some("thinking_delta")
                    {
                        events.push(event(id, ProviderEventType::Thinking, None));
                    }
                }
                if kind == "content_block_delta"
                    && value.pointer("/delta/type").and_then(Value::as_str)
                        == Some("input_json_delta")
                {
                    let mut e = event(id, ProviderEventType::ToolCall, None);
                    e.tool_calls = Some(vec![ToolCall {
                        call_id: format!("block-{}", value["index"]),
                        name: "tool".into(),
                        arguments: serde_json::json!({"partial_json":value.pointer("/delta/partial_json")}),
                    }]);
                    events.push(e);
                }
                if kind == "content_block_start"
                    && value.pointer("/content_block/type").and_then(Value::as_str)
                        == Some("tool_use")
                {
                    let block = &value["content_block"];
                    let mut e = event(id, ProviderEventType::ToolCall, None);
                    e.tool_calls = Some(vec![ToolCall {
                        call_id: block["id"].as_str().unwrap_or("pending").into(),
                        name: block["name"].as_str().unwrap_or("tool").into(),
                        arguments: block["input"].clone(),
                    }]);
                    events.push(e);
                }
                self.input = count(&value, "/message/usage/input_tokens")
                    .or(count(&value, "/usage/input_tokens"))
                    .or(self.input);
                self.output = count(&value, "/usage/output_tokens")
                    .or(count(&value, "/message/usage/output_tokens"))
                    .or(self.output);
                if kind == "message_stop" {
                    self.completed = true;
                }
            }
            Wire::Codex => {
                let kind = value["type"].as_str().unwrap_or("");
                if matches!(
                    kind,
                    "response.output_text.delta" | "response.refusal.delta"
                ) {
                    if let Some(text) = value["delta"].as_str() {
                        self.text.push_str(text);
                        events.push(ProviderEvent::chunk(id, text));
                    }
                }
                if kind == "response.reasoning_summary_text.delta" {
                    events.push(event(
                        id,
                        ProviderEventType::ReasoningSummary,
                        value["delta"].as_str().map(str::to_string),
                    ));
                }
                if kind == "response.function_call_arguments.delta" {
                    let mut e = event(id, ProviderEventType::ToolCall, None);
                    e.tool_calls = Some(vec![ToolCall {
                        call_id: value["item_id"].as_str().unwrap_or("tool").into(),
                        name: value["name"].as_str().unwrap_or("tool").into(),
                        arguments: serde_json::json!({"partial_json":value["delta"]}),
                    }]);
                    events.push(e);
                }
                if kind == "response.function_call_arguments.done"
                    || (kind == "response.output_item.done"
                        && value.pointer("/item/type").and_then(Value::as_str)
                            == Some("function_call"))
                {
                    let item = value.get("item").unwrap_or(&value);
                    let arguments = item["arguments"]
                        .as_str()
                        .and_then(|text| serde_json::from_str::<Value>(text).ok())
                        .unwrap_or_else(|| serde_json::json!({"invalid_json":true}));
                    let mut e = event(id, ProviderEventType::ToolCall, None);
                    e.tool_calls = Some(vec![ToolCall {
                        call_id: item["call_id"]
                            .as_str()
                            .or_else(|| item["item_id"].as_str())
                            .unwrap_or("tool")
                            .into(),
                        name: item["name"].as_str().unwrap_or("tool").into(),
                        arguments,
                    }]);
                    events.push(e);
                }
                if kind == "response.completed" {
                    self.completed = true;
                    self.input = count(&value, "/response/usage/input_tokens");
                    self.output = count(&value, "/response/usage/output_tokens");
                    let canonical = value
                        .pointer("/response/output")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|i| i.get("content").and_then(Value::as_array))
                                .flatten()
                                .filter(|c| {
                                    matches!(c["type"].as_str(), Some("output_text" | "refusal"))
                                })
                                .filter_map(|c| {
                                    c.get("text")
                                        .or_else(|| c.get("refusal"))
                                        .and_then(Value::as_str)
                                })
                                .collect::<String>()
                        });
                    if let Some(text) = canonical.filter(|t| !t.is_empty()) {
                        self.text = text;
                    }
                }
            }
            Wire::Ollama => {
                if let Some(text) = value.pointer("/message/content").and_then(Value::as_str) {
                    self.text.push_str(text);
                    events.push(ProviderEvent::chunk(id, text));
                }
                if value
                    .pointer("/message/thinking")
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.is_empty())
                {
                    events.push(event(id, ProviderEventType::Thinking, None));
                }
                self.input = count(&value, "/prompt_eval_count").or(self.input);
                self.output = count(&value, "/eval_count").or(self.output);
                if value["done"].as_bool() == Some(true) {
                    self.completed = true;
                }
            }
            Wire::Google => {
                let v = value.get("response").unwrap_or(&value);
                if let Some(parts) = v
                    .pointer("/candidates/0/content/parts")
                    .and_then(Value::as_array)
                {
                    for part in parts {
                        if part["thought"].as_bool() == Some(true) {
                            events.push(event(id, ProviderEventType::Thinking, None));
                        } else if let Some(text) = part["text"].as_str() {
                            self.text.push_str(text);
                            events.push(ProviderEvent::chunk(id, text));
                        }
                    }
                }
                self.input = count(v, "/usageMetadata/promptTokenCount").or(self.input);
                self.output = count(v, "/usageMetadata/candidatesTokenCount").or(self.output);
                if v.pointer("/candidates/0/finishReason").is_some() {
                    self.completed = true;
                }
            }
        }
        if previous_usage != (self.input, self.output) {
            let mut usage = event(id, ProviderEventType::Usage, None);
            usage.usage = Some(TokenUsage {
                input_tokens: self.input,
                output_tokens: self.output,
            });
            events.push(usage);
        }
        if self.text.len() > 2 * 1024 * 1024 {
            return Err(DomainError::Validation(
                "Provider output exceeded limit".into(),
            ));
        }
        Ok(events)
    }
}

pub fn unary_completion(id: &str, wire: Wire, value: &Value) -> Result<ProviderEvent, DomainError> {
    if value.get("error").is_some() {
        return Err(DomainError::Validation("Provider reported failure".into()));
    }
    let v = value.get("response").unwrap_or(value);
    let (text, input, output) = match wire {
        Wire::OpenAi => (
            v.pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .map(str::to_string),
            count(v, "/usage/prompt_tokens"),
            count(v, "/usage/completion_tokens"),
        ),
        Wire::Claude => (
            v["content"].as_array().map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"] == "text")
                    .filter_map(|b| b["text"].as_str())
                    .collect::<String>()
            }),
            count(v, "/usage/input_tokens"),
            count(v, "/usage/output_tokens"),
        ),
        Wire::Ollama => (
            v.pointer("/message/content")
                .and_then(Value::as_str)
                .map(str::to_string),
            count(v, "/prompt_eval_count"),
            count(v, "/eval_count"),
        ),
        Wire::Google => (
            v.pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|p| p["thought"].as_bool() != Some(true))
                        .filter_map(|p| p["text"].as_str())
                        .collect::<String>()
                }),
            count(v, "/usageMetadata/promptTokenCount"),
            count(v, "/usageMetadata/candidatesTokenCount"),
        ),
        Wire::Codex => (
            v["output"].as_array().map(|items| {
                items
                    .iter()
                    .filter_map(|i| i["content"].as_array())
                    .flatten()
                    .filter(|c| matches!(c["type"].as_str(), Some("output_text" | "refusal")))
                    .filter_map(|c| c["text"].as_str().or_else(|| c["refusal"].as_str()))
                    .collect::<String>()
            }),
            count(v, "/usage/input_tokens"),
            count(v, "/usage/output_tokens"),
        ),
    };
    let text = text
        .filter(|t| !t.is_empty())
        .ok_or_else(|| DomainError::Validation("Provider returned no answer".into()))?;
    let mut event = ProviderEvent::completed(
        id,
        Some(text),
        Some(TokenUsage {
            input_tokens: input,
            output_tokens: output,
        }),
    );
    event.model_id = actual_model(wire, v);
    event.tokens_used = input.zip(output).map(|(a, b)| a.saturating_add(b));
    Ok(event)
}
