//! Incremental provider framing. Text is decoded only after complete UTF-8 frames.
use custos_domain::DomainError;
use custos_provider::turn::{ModelTurnEvent, TurnDelta};
use custos_provider::{ProviderEvent, ProviderEventType, TokenUsage, ToolCall};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub enum Wire {
    OpenAi,
    Claude,
    Codex,
    Ollama,
    Google,
}

#[derive(Default)]
struct Decoder {
    model: Option<String>,
    text: String,
    input: Option<usize>,
    output: Option<usize>,
    completed: bool,
}
fn count(v: &Value, path: &str) -> Option<usize> {
    v.pointer(path)
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
}
fn event(id: &str, kind: ProviderEventType, text: Option<String>) -> ProviderEvent {
    let mut event = ProviderEvent::chunk(id, text.unwrap_or_default());
    event.event_type = kind;
    event
}

impl Decoder {
    fn completion(self, id: &str) -> ProviderEvent {
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
    fn decode(
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

#[cfg(test)]
pub fn stream_response(
    response: reqwest::Response,
    id: String,
    wire: Wire,
) -> mpsc::Receiver<ProviderEvent> {
    stream_response_with_idle_timeout(response, id, wire, Duration::from_secs(45))
}

#[cfg(test)]
fn stream_response_with_idle_timeout(
    response: reqwest::Response,
    id: String,
    wire: Wire,
    idle: Duration,
) -> mpsc::Receiver<ProviderEvent> {
    stream_response_observed_inner(response, id, wire, idle, None)
}

pub fn stream_response_observed(response: reqwest::Response, req: custos_provider::ProviderRequest, wire: Wire, resolver: std::sync::Arc<dyn super::router::CredentialResolver>) -> mpsc::Receiver<ProviderEvent> {
    stream_response_observed_inner(response, req.request_id.clone(), wire, Duration::from_secs(45), Some((resolver, req)))
}

type ResponseObserver = (std::sync::Arc<dyn super::router::CredentialResolver>, custos_provider::ProviderRequest);

async fn observe_frame(observer: &Option<ResponseObserver>, frame: &str) -> Result<(), DomainError> {
    let Some((resolver, req)) = observer else { return Ok(()); };
    let Some(account) = req.provider_account_id.as_deref() else { return Ok(()); };
    if let Ok(value) = serde_json::from_str::<Value>(frame) {
        let error = value.get("error").or_else(|| value.pointer("/response/error"));
        if let Some(error) = error.filter(|error| !error.is_null()) {
            let code = error.get("code").filter(|value| value.is_string()).or_else(|| error.get("type")).and_then(Value::as_str).unwrap_or("");
            let status = match code {
                "insufficient_quota" | "usage_limit_reached" | "quota_exhausted" | "rate_limit_exceeded" => 429,
                "model_not_found" | "model_not_supported" | "model_access_denied" => 404,
                "authentication_error" | "invalid_api_key" => 401,
                _ => error.get("code").and_then(Value::as_u64).and_then(|value| u16::try_from(value).ok()).unwrap_or(200),
            };
            resolver.observe_response(account, &req.model, status, &serde_json::json!({"error":error}).to_string(), None).await?;
        }
    }
    Ok(())
}

fn stream_response_observed_inner(mut response: reqwest::Response, id: String, wire: Wire, idle: Duration, observer: Option<ResponseObserver>) -> mpsc::Receiver<ProviderEvent> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let result=async {
            if response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_some_and(|value|value.split(';').next()==Some("application/json")) {
                let mut bytes=Vec::new();
                loop {
                    let chunk=tokio::select! {
                        _=tx.closed()=>return Ok::<(),DomainError>(()),
                        result=tokio::time::timeout(idle,response.chunk())=>result.map_err(|_|DomainError::Validation("Provider response timed out".into()))?.map_err(|_|DomainError::Validation("Provider response disconnected".into()))?,
                    };
                    let Some(chunk)=chunk else {break;};
                    if bytes.len().saturating_add(chunk.len())>2*1024*1024 {return Err(DomainError::Validation("Provider response exceeded limit".into()));}
                    bytes.extend_from_slice(&chunk);
                }
                let value:Value=serde_json::from_slice(&bytes).map_err(|_|DomainError::Validation("Malformed provider response".into()))?;
                observe_frame(&observer, &value.to_string()).await?;
                let completion=unary_completion(&id,wire,&value)?;
                let _=tx.send(completion).await;
                return Ok(());
            }
            let mut pending=Vec::new(); let mut data=String::new(); let mut decoder=Decoder::default();
            loop {
                let chunk=tokio::select! {
                    _=tx.closed()=>return Ok::<(),DomainError>(()),
                    result=tokio::time::timeout(idle,response.chunk())=>result.map_err(|_|DomainError::Validation("Provider stream timed out".into()))?.map_err(|_|DomainError::Validation("Provider stream disconnected".into()))?,
                };
                let eof=chunk.is_none();
                if let Some(chunk)=chunk { pending.extend_from_slice(&chunk); }
                if pending.len()>1024*1024 { return Err(DomainError::Validation("Provider frame exceeded limit".into())); }
                if eof && !pending.is_empty() { pending.push(b'\n'); }
                while let Some(end)=pending.iter().position(|b|*b==b'\n') {
                    let bytes:Vec<_>=pending.drain(..=end).collect();
                    let line=std::str::from_utf8(&bytes).map_err(|_|DomainError::Validation("Invalid provider UTF-8".into()))?.trim_end_matches(['\r','\n']);
                    let frame=if matches!(wire,Wire::Ollama) { if line.is_empty() { None } else {Some(line.to_string())} }
                    else if line.is_empty() { if data.is_empty() {None} else {Some(std::mem::take(&mut data))} }
                    else { if let Some(part)=line.strip_prefix("data:") { if !data.is_empty() {data.push('\n');} data.push_str(part.trim_start()); } None };
                    if data.len()>1024*1024 { return Err(DomainError::Validation("Provider frame exceeded limit".into())); }
                    if let Some(frame)=frame {
                        observe_frame(&observer, &frame).await?;
                        for event in decoder.decode(&id,wire,&frame)? { if tx.send(event).await.is_err() { return Ok(()); } }
                        if decoder.completed && (!matches!(wire,Wire::OpenAi) || frame.trim()=="[DONE]") {
                            let _=tx.send(decoder.completion(&id)).await;
                            return Ok(());
                        }
                    }
                }
                if eof {
                    if !data.is_empty() { observe_frame(&observer, &data).await?; for event in decoder.decode(&id,wire,&data)? { if tx.send(event).await.is_err() {return Ok(());} } }
                    if !decoder.completed { return Err(DomainError::Validation("Provider stream ended without completion".into())); }
                    let _=tx.send(decoder.completion(&id)).await;
                    return Ok(());
                }
            }
        }.await;
        if let Err(error) = result {
            if error.to_string().contains("disconnected") || error.to_string().contains("timed out") {
                if let Some((resolver, req)) = &observer { if let Some(account) = &req.provider_account_id {
                    if let Err(failure) = resolver.observe_response(account, &req.model, 0, "", None).await { tracing::debug!(%failure, "Stream availability projection failed"); }
                } }
            }
            let _ = tx.send(ProviderEvent::failed(&id, error.to_string())).await;
        }
    });
    rx
}

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
                        if let Some(thinking) = value.pointer("/delta/thinking").and_then(Value::as_str) {
                            if !thinking.is_empty() {
                                events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Reasoning { text: thinking.to_string() }));
                            }
                        }
                    } else if delta_type == Some("text_delta") {
                        if let Some(text) = value.pointer("/delta/text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Text { text: text.to_string() }));
                            }
                        }
                    } else if delta_type == Some("input_json_delta") {
                        if let Some(partial) = value.pointer("/delta/partial_json").and_then(Value::as_str) {
                            let idx = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                            events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::ToolCall {
                                id: format!("block-{idx}"),
                                name: None,
                                arguments_chunk: partial.to_string(),
                            }));
                        }
                    }
                } else if kind == "content_block_start" {
                    if value.pointer("/content_block/type").and_then(Value::as_str) == Some("tool_use") {
                        let block = &value["content_block"];
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::ToolCall {
                            id: block["id"].as_str().unwrap_or("tool").to_string(),
                            name: block["name"].as_str().map(String::from),
                            arguments_chunk: String::new(),
                        }));
                    }
                } else if kind == "content_block_stop" {
                    let idx = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::ToolCallComplete {
                        id: format!("block-{idx}"),
                    }));
                } else if kind == "message_delta" || kind == "message_stop" {
                    let input = count(&value, "/message/usage/input_tokens").or_else(|| count(&value, "/usage/input_tokens"));
                    let output = count(&value, "/usage/output_tokens").or_else(|| count(&value, "/message/usage/output_tokens"));
                    let cache_read = count(&value, "/message/usage/cache_read_input_tokens")
                        .or_else(|| count(&value, "/usage/cache_read_input_tokens"));
                    let cache_write = count(&value, "/message/usage/cache_creation_input_tokens")
                        .or_else(|| count(&value, "/usage/cache_creation_input_tokens"));
                    if input.is_some() || output.is_some() || cache_read.is_some() || cache_write.is_some() {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Usage {
                            input_tokens: input.unwrap_or(0) as u64,
                            output_tokens: output.unwrap_or(0) as u64,
                            cache_read_tokens: cache_read.map(|c| c as u64),
                            cache_write_tokens: cache_write.map(|c| c as u64),
                        }));
                    }
                    if kind == "message_stop" {
                        self.completed = true;
                        let stop_reason = value.pointer("/delta/stop_reason").and_then(Value::as_str).unwrap_or("stop");
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Completed {
                            finish_reason: Some(stop_reason.to_string()),
                        }));
                    }
                }
            }
            Wire::OpenAi => {
                if let Some(reasoning) = value.pointer("/choices/0/delta/reasoning_content")
                    .or_else(|| value.pointer("/choices/0/delta/reasoning"))
                    .and_then(Value::as_str)
                {
                    if !reasoning.is_empty() {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Reasoning { text: reasoning.to_string() }));
                    }
                }
                if let Some(text) = value.pointer("/choices/0/delta/content").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Text { text: text.to_string() }));
                    }
                }
                if let Some(calls) = value.pointer("/choices/0/delta/tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        let id = call.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                        let name = call.pointer("/function/name").and_then(Value::as_str).map(String::from);
                        let chunk = call.pointer("/function/arguments").and_then(Value::as_str).unwrap_or("").to_string();
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::ToolCall {
                            id,
                            name,
                            arguments_chunk: chunk,
                        }));
                    }
                }
                if let (Some(input), Some(output)) = (count(&value, "/usage/prompt_tokens"), count(&value, "/usage/completion_tokens")) {
                    let cache_read = count(&value, "/usage/prompt_tokens_details/cached_tokens");
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Usage {
                        input_tokens: input as u64,
                        output_tokens: output as u64,
                        cache_read_tokens: cache_read.map(|c| c as u64),
                        cache_write_tokens: None,
                    }));
                }
                if let Some(reason) = value.pointer("/choices/0/finish_reason").and_then(Value::as_str) {
                    self.completed = true;
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Completed {
                        finish_reason: Some(reason.to_string()),
                    }));
                }
            }
            Wire::Google => {
                let v = value.get("response").unwrap_or(&value);
                if let Some(parts) = v.pointer("/candidates/0/content/parts").and_then(Value::as_array) {
                    for part in parts {
                        if part.get("thought").and_then(Value::as_bool) == Some(true) {
                            if let Some(thought) = part.get("text").and_then(Value::as_str) {
                                if !thought.is_empty() {
                                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Reasoning { text: thought.to_string() }));
                                }
                            }
                        } else if let Some(text) = part.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Text { text: text.to_string() }));
                            }
                        } else if let Some(func) = part.get("functionCall") {
                            events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::ToolCall {
                                id: "gemini_call".to_string(),
                                name: func.get("name").and_then(Value::as_str).map(String::from),
                                arguments_chunk: func.get("args").map(|a| a.to_string()).unwrap_or_default(),
                            }));
                        }
                    }
                }
                if let (Some(input), Some(output)) = (
                    count(v, "/usageMetadata/promptTokenCount"),
                    count(v, "/usageMetadata/candidatesTokenCount"),
                ) {
                    let cache_read = count(v, "/usageMetadata/cachedContentTokenCount");
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Usage {
                        input_tokens: input as u64,
                        output_tokens: output as u64,
                        cache_read_tokens: cache_read.map(|c| c as u64),
                        cache_write_tokens: None,
                    }));
                }
                if let Some(reason) = v.pointer("/candidates/0/finishReason").and_then(Value::as_str) {
                    self.completed = true;
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Completed {
                        finish_reason: Some(reason.to_string()),
                    }));
                }
            }
            Wire::Ollama => {
                if let Some(thinking) = value.pointer("/message/thinking").and_then(Value::as_str) {
                    if !thinking.is_empty() {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Reasoning { text: thinking.to_string() }));
                    }
                }
                if let Some(text) = value.pointer("/message/content").and_then(Value::as_str) {
                    if !text.is_empty() {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Text { text: text.to_string() }));
                    }
                }
                if value.get("done").and_then(Value::as_bool) == Some(true) {
                    self.completed = true;
                    let input = count(&value, "/prompt_eval_count").unwrap_or(0) as u64;
                    let output = count(&value, "/eval_count").unwrap_or(0) as u64;
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Usage {
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: None,
                        cache_write_tokens: None,
                    }));
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Completed {
                        finish_reason: Some("stop".to_string()),
                    }));
                }
            }
            Wire::Codex => {
                if let Some(delta) = value.pointer("/delta").and_then(Value::as_str) {
                    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
                    if kind.contains("reasoning") {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Reasoning { text: delta.to_string() }));
                    } else {
                        events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Text { text: delta.to_string() }));
                    }
                }
                if value.get("type").and_then(Value::as_str) == Some("response.completed") {
                    self.completed = true;
                    let input = count(&value, "/response/usage/input_tokens").unwrap_or(0) as u64;
                    let output = count(&value, "/response/usage/output_tokens").unwrap_or(0) as u64;
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Usage {
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: None,
                        cache_write_tokens: None,
                    }));
                    events.push(self.make_event(attempt_id, actual_provider, &model, TurnDelta::Completed {
                        finish_reason: Some("stop".to_string()),
                    }));
                }
            }
        }

        Ok(events)
    }
}

pub fn stream_turn_response_observed(
    mut response: reqwest::Response,
    attempt_id: Uuid,
    actual_provider: String,
    actual_model: String,
    wire: Wire,
    idle: Duration,
) -> mpsc::Receiver<ModelTurnEvent> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let mut decoder = ModelTurnStreamDecoder::default();
        let prov = actual_provider.clone();
        let modl = actual_model.clone();
        let result = async {
            let mut pending = Vec::new();
            let mut data = String::new();
            loop {
                let chunk = tokio::select! {
                    _ = tx.closed() => return Ok::<(), DomainError>(()),
                    res = tokio::time::timeout(idle, response.chunk()) => {
                        res.map_err(|_| DomainError::Validation("Provider stream timed out".into()))?
                           .map_err(|_| DomainError::Validation("Provider stream disconnected".into()))?
                    }
                };
                let eof = chunk.is_none();
                if let Some(chunk) = chunk { pending.extend_from_slice(&chunk); }
                if pending.len() > 1024 * 1024 { return Err(DomainError::Validation("Provider frame exceeded limit".into())); }
                if eof && !pending.is_empty() { pending.push(b'\n'); }
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    let bytes: Vec<_> = pending.drain(..=end).collect();
                    let line = std::str::from_utf8(&bytes).map_err(|_| DomainError::Validation("Invalid provider UTF-8".into()))?.trim_end_matches(['\r', '\n']);
                    let frame = if matches!(wire, Wire::Ollama) {
                        if line.is_empty() { None } else { Some(line.to_string()) }
                    } else if line.is_empty() {
                        if data.is_empty() { None } else { Some(std::mem::take(&mut data)) }
                    } else {
                        if let Some(part) = line.strip_prefix("data:") {
                            if !data.is_empty() { data.push('\n'); }
                            data.push_str(part.trim_start());
                        }
                        None
                    };
                    if let Some(frame) = frame {
                        let evts = decoder.decode(attempt_id, &prov, &modl, wire, &frame)?;
                        for evt in evts {
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                        if decoder.completed && (!matches!(wire, Wire::OpenAi) || frame.trim() == "[DONE]") {
                            return Ok(());
                        }
                    }
                }
                if eof {
                    if !data.is_empty() {
                        let evts = decoder.decode(attempt_id, &prov, &modl, wire, &data)?;
                        for evt in evts {
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                    }
                    if !decoder.completed {
                        decoder.sequence += 1;
                        let _ = tx.send(ModelTurnEvent {
                            attempt_id,
                            sequence: decoder.sequence,
                            actual_provider: Some(prov.clone()),
                            actual_model: Some(modl.clone()),
                            actual_transport: Some("turn_sse".to_string()),
                            delta: TurnDelta::Completed { finish_reason: Some("stop".to_string()) },
                            usage_certainty: Some("estimated".to_string()),
                            provider_response_id: None,
                        }).await;
                    }
                    return Ok(());
                }
            }
        }.await;
        if let Err(err) = result {
            decoder.sequence += 1;
            let _ = tx.send(ModelTurnEvent {
                attempt_id,
                sequence: decoder.sequence,
                actual_provider: Some(prov),
                actual_model: Some(modl),
                actual_transport: Some("turn_sse".to_string()),
                delta: TurnDelta::Failed {
                    error_class: "stream_error".to_string(),
                    message: err.to_string(),
                },
                usage_certainty: None,
                provider_response_id: None,
            }).await;
        }
    });
    rx
}

fn actual_model(wire: Wire, value: &Value) -> Option<String> {
    let model = match wire {
        Wire::OpenAi | Wire::Ollama => value.get("model"),
        Wire::Claude => value
            .pointer("/message/model")
            .or_else(|| value.get("model")),
        Wire::Codex => value
            .pointer("/response/model")
            .or_else(|| value.get("model")),
        Wire::Google => None,
    }
    .and_then(Value::as_str)?;
    Some(if matches!(wire, Wire::Ollama) {
        format!("ollama/{model}")
    } else {
        model.to_string()
    })
}

fn unary_completion(id: &str, wire: Wire, value: &Value) -> Result<ProviderEvent, DomainError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    async fn response(chunks: Vec<Vec<u8>>) -> reqwest::Response {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            for chunk in chunks {
                socket
                    .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                    .await
                    .unwrap();
                socket.write_all(&chunk).await.unwrap();
                socket.write_all(b"\r\n").await.unwrap();
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            socket.write_all(b"0\r\n\r\n").await.unwrap();
        });
        reqwest::get(format!("http://{address}")).await.unwrap()
    }
    #[tokio::test]
    async fn utf8_split_delta_precedes_completion_with_usage() {
        let body="data: {\"choices\":[{\"delta\":{\"content\":\"Xin chào\"}}]}\n\ndata: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3}}\n\ndata: [DONE]\n\n".as_bytes();
        let split = body.iter().position(|b| *b == 0xc3).unwrap() + 1;
        let mut receiver = stream_response(
            response(vec![body[..split].to_vec(), body[split..].to_vec()]).await,
            "req".into(),
            Wire::OpenAi,
        );
        let first = receiver.recv().await.unwrap();
        assert_eq!(first.event_type, ProviderEventType::Chunk);
        assert_eq!(first.text.as_deref(), Some("Xin chào"));
        let usage = receiver.recv().await.unwrap();
        assert_eq!(usage.event_type, ProviderEventType::Usage);
        let done = receiver.recv().await.unwrap();
        assert_eq!(done.event_type, ProviderEventType::Completed);
        assert_eq!(done.usage.unwrap().output_tokens, Some(3));
    }
    #[tokio::test]
    async fn midstream_failure_keeps_prior_delta() {
        let mut receiver = stream_response(
            response(vec![
                b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec(),
                b"data: {\"error\":{\"message\":\"secret\"}}\n\n".to_vec(),
            ])
            .await,
            "req".into(),
            Wire::OpenAi,
        );
        assert_eq!(
            receiver.recv().await.unwrap().text.as_deref(),
            Some("partial")
        );
        assert_eq!(
            receiver.recv().await.unwrap().event_type,
            ProviderEventType::Failed
        );
    }
    #[tokio::test]
    async fn idle_timeout_reports_failure_after_partial_output() {
        let response = response(vec![
            b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec(),
        ])
        .await;
        let mut receiver = stream_response_with_idle_timeout(
            response,
            "req".into(),
            Wire::OpenAi,
            Duration::from_millis(10),
        );
        assert_eq!(
            receiver.recv().await.unwrap().event_type,
            ProviderEventType::Chunk
        );
        let failure = receiver.recv().await.unwrap();
        assert_eq!(failure.event_type, ProviderEventType::Failed);
        assert!(failure.error.unwrap().contains("timed out"));
    }
    #[test]
    fn canonical_response_excludes_private_reasoning_and_keeps_public_refusal() {
        let mut decoder = Decoder::default();
        decoder.decode("req",Wire::Codex,r#"{"type":"response.completed","response":{"output":[{"content":[{"type":"reasoning_text","text":"private chain"},{"type":"output_text","text":"public answer"}]}]}}"#).unwrap();
        assert_eq!(decoder.text, "public answer");
        let mut decoder = Decoder::default();
        let events=decoder.decode("req",Wire::Claude,r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","text":"private chain","thinking":"private chain"}}"#).unwrap();
        assert!(events
            .iter()
            .all(|e| e.text.as_deref() != Some("private chain")));
        let events = decoder
            .decode(
                "req",
                Wire::Codex,
                r#"{"type":"response.refusal.delta","delta":"public refusal"}"#,
            )
            .unwrap();
        assert_eq!(events[0].text.as_deref(), Some("public refusal"));
    }
    #[test]
    fn unary_fallback_emits_only_canonical_completion() {
        let event=unary_completion("req",Wire::OpenAi,&serde_json::json!({"choices":[{"message":{"content":"whole answer"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}})).unwrap();
        assert_eq!(event.event_type, ProviderEventType::Completed);
        assert_eq!(event.text.as_deref(), Some("whole answer"));
        assert_eq!(event.usage.unwrap().input_tokens, Some(3));
    }
    #[test]
    fn ollama_claude_google_and_public_summary_usage() {
        for (wire, frame) in [
            (
                Wire::Ollama,
                r#"{"message":{"content":"ok"},"done":true,"prompt_eval_count":2,"eval_count":1}"#,
            ),
            (
                Wire::Google,
                r#"{"candidates":[{"content":{"parts":[{"text":"ok"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":1}}"#,
            ),
        ] {
            let mut decoder = Decoder::default();
            assert_eq!(
                decoder.decode("req", wire, frame).unwrap()[0]
                    .text
                    .as_deref(),
                Some("ok")
            );
            assert!(decoder.completed);
            assert_eq!(decoder.output, Some(1));
        }
        let mut decoder = Decoder::default();
        let events=decoder.decode("req",Wire::Claude,r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"private"}}"#).unwrap();
        assert_eq!(events[0].event_type, ProviderEventType::Thinking);
        assert_eq!(events[0].text.as_deref(), Some(""));
        let events = decoder
            .decode(
                "req",
                Wire::Codex,
                r#"{"type":"response.reasoning_summary_text.delta","delta":"public"}"#,
            )
            .unwrap();
        assert_eq!(events[0].event_type, ProviderEventType::ReasoningSummary);
    }

    #[test]
    fn model_turn_stream_decoder_separates_reasoning_and_text() {
        let mut decoder = ModelTurnStreamDecoder::default();
        let attempt = Uuid::new_v4();

        // 1. Claude thinking delta followed by text delta
        let events = decoder.decode(attempt, "anthropic", "claude-3-7-sonnet", Wire::Claude,
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"let me think"}}"#).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].delta {
            TurnDelta::Reasoning { text } => assert_eq!(text, "let me think"),
            other => panic!("expected Reasoning delta, got {:?}", other),
        }

        let events = decoder.decode(attempt, "anthropic", "claude-3-7-sonnet", Wire::Claude,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello world"}}"#).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].delta {
            TurnDelta::Text { text } => assert_eq!(text, "hello world"),
            other => panic!("expected Text delta, got {:?}", other),
        }

        // 2. OpenAI reasoning_content followed by content
        let mut oai_decoder = ModelTurnStreamDecoder::default();
        let events = oai_decoder.decode(attempt, "openai", "o3-mini", Wire::OpenAi,
            r#"{"choices":[{"delta":{"reasoning_content":"step 1 reasoning"}}]}"#).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].delta {
            TurnDelta::Reasoning { text } => assert_eq!(text, "step 1 reasoning"),
            other => panic!("expected Reasoning delta, got {:?}", other),
        }

        let events = oai_decoder.decode(attempt, "openai", "o3-mini", Wire::OpenAi,
            r#"{"choices":[{"delta":{"content":"final answer"}}]}"#).unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].delta {
            TurnDelta::Text { text } => assert_eq!(text, "final answer"),
            other => panic!("expected Text delta, got {:?}", other),
        }

        // 3. Google Gemini thought part
        let mut gemini_decoder = ModelTurnStreamDecoder::default();
        let events = gemini_decoder.decode(attempt, "google", "gemini-2.5-flash", Wire::Google,
            r#"{"candidates":[{"content":{"parts":[{"text":"pondering deep questions","thought":true},{"text":"clear answer"}]}}]}"#).unwrap();
        assert_eq!(events.len(), 2);
        match &events[0].delta {
            TurnDelta::Reasoning { text } => assert_eq!(text, "pondering deep questions"),
            other => panic!("expected Reasoning delta, got {:?}", other),
        }
        match &events[1].delta {
            TurnDelta::Text { text } => assert_eq!(text, "clear answer"),
            other => panic!("expected Text delta, got {:?}", other),
        }
    }
}
