//! Native Local Model Adapter implementing ModelProvider (ModelPort)
//!
//! Bridges structured model execution (ModelTurnRequest, ModelTurnEvent)
//! directly to LocalInferenceProvider and llama.cpp/MLX runtime.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::base::Provider;
use custos_provider::conversation::message::{Message, MessageContentBlock};
use custos_provider::events::ProviderEvent;
use custos_provider::model::ModelConfig;
use custos_provider::port::ModelProvider;
use custos_provider::request::{ModelResponse, ProviderRequest};
use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest, TurnDelta};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use super::LocalInferenceProvider;

/// Adapter connecting LocalInferenceProvider into the Custos ModelPort convergence boundary.
pub struct NativeLocalModelAdapter {
    inner: Arc<LocalInferenceProvider>,
    provider_id: String,
}

impl NativeLocalModelAdapter {
    pub fn new(inner: Arc<LocalInferenceProvider>) -> Self {
        Self {
            inner,
            provider_id: "native-local".to_string(),
        }
    }

    pub fn provider(&self) -> Arc<LocalInferenceProvider> {
        self.inner.clone()
    }
}

#[async_trait]
impl ModelProvider for NativeLocalModelAdapter {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let clean_model = req
            .model
            .strip_prefix("native:")
            .unwrap_or(&req.model)
            .to_string();

        let mut config = ModelConfig::new(&clean_model);
        if let Some(t) = req.temperature {
            config = config.with_temperature(Some(t as f32));
        }
        if let Some(m) = req.max_tokens {
            config = config.with_max_tokens(Some(m as i32));
        }

        let user_msg = Message::user().with_text(&req.prompt);
        let mut stream = self
            .inner
            .stream(&config, "", &[user_msg], &[])
            .await
            .map_err(|e| DomainError::Validation(format!("Native local inference failed: {e}")))?;

        let mut content = String::new();
        let mut input_tokens = 0usize;
        let mut output_tokens = 0usize;

        while let Some(item_res) = stream.next().await {
            match item_res {
                Ok((maybe_msg, maybe_usage)) => {
                    if let Some(msg) = maybe_msg {
                        for block in msg.content {
                            if let MessageContentBlock::Text(t) = block {
                                content.push_str(&t.text);
                            }
                        }
                    }
                    if let Some(u) = maybe_usage {
                        input_tokens = u.usage.input_tokens.unwrap_or(0).max(0) as usize;
                        output_tokens = u.usage.output_tokens.unwrap_or(0).max(0) as usize;
                    }
                }
                Err(e) => {
                    return Err(DomainError::Validation(format!(
                        "Native local inference stream error: {e}"
                    )));
                }
            }
        }

        let total_tokens = input_tokens + output_tokens;
        Ok(ModelResponse {
            content,
            tokens_used: total_tokens,
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            total_tokens: Some(total_tokens),
            model_id: req.model.clone(),
            tool_calls: Vec::new(),
        })
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let clean_model = req
            .model
            .strip_prefix("native:")
            .unwrap_or(&req.model)
            .to_string();

        let mut config = ModelConfig::new(&clean_model);
        if let Some(t) = req.temperature {
            config = config.with_temperature(Some(t as f32));
        }
        if let Some(m) = req.max_tokens {
            config = config.with_max_tokens(Some(m as i32));
        }

        let user_msg = Message::user().with_text(&req.prompt);
        let mut inner_stream = self
            .inner
            .stream(&config, "", &[user_msg], &[])
            .await
            .map_err(|e| DomainError::Validation(format!("Native local inference failed: {e}")))?;

        let (tx, rx) = mpsc::channel(32);
        let req_id = req.request_id.clone();
        let model_id = req.model.clone();

        tokio::spawn(async move {
            let mut total_output = 0usize;
            let mut input_tokens = 0usize;

            while let Some(item_res) = inner_stream.next().await {
                match item_res {
                    Ok((maybe_msg, maybe_usage)) => {
                        if let Some(msg) = maybe_msg {
                            for block in msg.content {
                                if let MessageContentBlock::Text(t) = block {
                                    total_output += t.text.len();
                                    let mut event = ProviderEvent::chunk(req_id.clone(), t.text);
                                    event.model_id = Some(model_id.clone());
                                    if tx.send(event).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                        if let Some(u) = maybe_usage {
                            input_tokens = u.usage.input_tokens.unwrap_or(0).max(0) as usize;
                            if let Some(out) = u.usage.output_tokens {
                                total_output = out.max(0) as usize;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ProviderEvent::failed(
                                req_id.clone(),
                                format!("Local inference error: {e}"),
                            ))
                            .await;
                        return;
                    }
                }
            }

            let mut completed = ProviderEvent::completed(
                req_id,
                None,
                Some(custos_provider::events::TokenUsage {
                    input_tokens: Some(input_tokens),
                    output_tokens: Some(total_output),
                }),
            );
            completed.model_id = Some(model_id);
            completed.tokens_used = Some(input_tokens + total_output);
            let _ = tx.send(completed).await;
        });

        Ok(rx)
    }

    async fn execute_turn(
        &self,
        req: ModelTurnRequest,
    ) -> Result<mpsc::Receiver<ModelTurnEvent>, DomainError> {
        let attempt_id = req.attempt_id;
        let provider_id = self.provider_id.clone();
        let requested_model = req.requested_model.clone();
        let clean_model = req
            .requested_model
            .strip_prefix("native:")
            .unwrap_or(&req.requested_model)
            .to_string();

        let mut config = ModelConfig::new(&clean_model);
        if let Some(opts) = &req.decoding_options {
            if let Some(temp) = opts.get("temperature").and_then(|v| v.as_f64()) {
                config = config.with_temperature(Some(temp as f32));
            }
            if let Some(max_tokens) = opts.get("max_tokens").and_then(|v| v.as_i64()) {
                config = config.with_max_tokens(Some(max_tokens as i32));
            }
        }

        // Convert tool definitions to rmcp::model::Tool
        let tools: Vec<rmcp::model::Tool> = req
            .tool_schemas
            .iter()
            .map(|def| {
                let schema = match &def.parameters {
                    serde_json::Value::Object(map) => Arc::new(map.clone()),
                    _ => Arc::new(serde_json::Map::new()),
                };
                rmcp::model::Tool::new(def.name.clone(), def.description.clone(), schema)
            })
            .collect();

        let messages = req.structured_messages.clone();
        let mut inner_stream = self
            .inner
            .stream(&config, "", &messages, &tools)
            .await
            .map_err(|e| DomainError::Validation(format!("Native local inference failed: {e}")))?;

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let mut seq = 1u64;

            while let Some(item_res) = inner_stream.next().await {
                match item_res {
                    Ok((maybe_msg, maybe_usage)) => {
                        if let Some(msg) = maybe_msg {
                            for block in msg.content {
                                match block {
                                    MessageContentBlock::Text(t) => {
                                        if !t.text.is_empty() {
                                            let event = ModelTurnEvent {
                                                attempt_id,
                                                sequence: seq,
                                                actual_provider: Some(provider_id.clone()),
                                                actual_model: Some(requested_model.clone()),
                                                actual_transport: Some("llama.cpp".to_string()),
                                                delta: TurnDelta::Text { text: t.text },
                                                usage_certainty: Some("known".to_string()),
                                                provider_response_id: None,
                                            };
                                            seq += 1;
                                            if tx.send(event).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    MessageContentBlock::Thinking(th) => {
                                        if !th.thinking.is_empty() {
                                            let event = ModelTurnEvent {
                                                attempt_id,
                                                sequence: seq,
                                                actual_provider: Some(provider_id.clone()),
                                                actual_model: Some(requested_model.clone()),
                                                actual_transport: Some("llama.cpp".to_string()),
                                                delta: TurnDelta::Reasoning { text: th.thinking },
                                                usage_certainty: Some("known".to_string()),
                                                provider_response_id: None,
                                            };
                                            seq += 1;
                                            if tx.send(event).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    MessageContentBlock::ToolRequest(tr) => {
                                        if let Ok(tc) = &tr.tool_call {
                                            let call_id = tr.id.clone();
                                            let args_str = tc
                                                .arguments
                                                .as_ref()
                                                .map(|a| serde_json::to_string(a).unwrap_or_default())
                                                .unwrap_or_default();

                                            let call_event = ModelTurnEvent {
                                                attempt_id,
                                                sequence: seq,
                                                actual_provider: Some(provider_id.clone()),
                                                actual_model: Some(requested_model.clone()),
                                                actual_transport: Some("llama.cpp".to_string()),
                                                delta: TurnDelta::ToolCall {
                                                    id: call_id.clone(),
                                                    name: Some(tc.name.to_string()),
                                                    arguments_chunk: args_str,
                                                },
                                                usage_certainty: Some("known".to_string()),
                                                provider_response_id: None,
                                            };
                                            seq += 1;
                                            if tx.send(call_event).await.is_err() {
                                                return;
                                            }

                                            let complete_event = ModelTurnEvent {
                                                attempt_id,
                                                sequence: seq,
                                                actual_provider: Some(provider_id.clone()),
                                                actual_model: Some(requested_model.clone()),
                                                actual_transport: Some("llama.cpp".to_string()),
                                                delta: TurnDelta::ToolCallComplete { id: call_id },
                                                usage_certainty: Some("known".to_string()),
                                                provider_response_id: None,
                                            };
                                            seq += 1;
                                            if tx.send(complete_event).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    MessageContentBlock::Error(err) => {
                                        let fail_event = ModelTurnEvent {
                                            attempt_id,
                                            sequence: seq,
                                            actual_provider: Some(provider_id.clone()),
                                            actual_model: Some(requested_model.clone()),
                                            actual_transport: Some("llama.cpp".to_string()),
                                            delta: TurnDelta::Failed {
                                                error_class: "inference_error".to_string(),
                                                message: err.message,
                                            },
                                            usage_certainty: None,
                                            provider_response_id: None,
                                        };
                                        let _ = tx.send(fail_event).await;
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if let Some(u) = maybe_usage {
                            let usage_event = ModelTurnEvent {
                                attempt_id,
                                sequence: seq,
                                actual_provider: Some(provider_id.clone()),
                                actual_model: None,
                                actual_transport: None,
                                delta: TurnDelta::Usage {
                                    input_tokens: u.usage.input_tokens.unwrap_or(0).max(0) as u64,
                                    output_tokens: u.usage.output_tokens.unwrap_or(0).max(0) as u64,
                                    cache_read_tokens: None,
                                    cache_write_tokens: None,
                                },
                                usage_certainty: Some("known".to_string()),
                                provider_response_id: None,
                            };
                            seq += 1;
                            let _ = tx.send(usage_event).await;
                        }
                    }
                    Err(e) => {
                        let fail_event = ModelTurnEvent {
                            attempt_id,
                            sequence: seq,
                            actual_provider: Some(provider_id.clone()),
                            actual_model: Some(requested_model.clone()),
                            actual_transport: None,
                            delta: TurnDelta::Failed {
                                error_class: "local_inference_error".to_string(),
                                message: e.to_string(),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        };
                        let _ = tx.send(fail_event).await;
                        return;
                    }
                }
            }

            // Normal completion
            let completed_event = ModelTurnEvent {
                attempt_id,
                sequence: seq,
                actual_provider: None,
                actual_model: None,
                actual_transport: None,
                delta: TurnDelta::Completed {
                    finish_reason: Some("stop".to_string()),
                },
                usage_certainty: None,
                provider_response_id: None,
            };
            let _ = tx.send(completed_event).await;
        });

        Ok(rx)
    }
}
