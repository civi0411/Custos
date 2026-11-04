//! Bidirectional bridge between the canonical hexagonal `ModelPort` (`ModelProvider`)
//! and the compatibility `Provider` (Goose/MCP conversation provider) surface.

use std::sync::Arc;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use custos_domain::DomainError;

use crate::base::{MessageStream, Provider};
use crate::conversation::message::Message;
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;
use crate::events::ProviderEvent;
use crate::model::ModelConfig;
use crate::port::ModelProvider;
use crate::request::{ModelResponse, ProviderRequest};

/// Adapts an `Arc<dyn Provider>` into a canonical `ModelProvider` (`ModelPort`).
pub struct ProviderToModelPort {
    inner: Arc<dyn Provider>,
}

impl ProviderToModelPort {
    pub fn new(inner: Arc<dyn Provider>) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &Arc<dyn Provider> {
        &self.inner
    }
}

#[async_trait]
impl ModelProvider for ProviderToModelPort {
    fn provider_id(&self) -> &str {
        self.inner.get_name()
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let model_config = ModelConfig::new(&req.model);
        let messages = vec![Message::user().with_text(&req.prompt)];
        let (msg, usage) = self
            .inner
            .complete(&model_config, "", &messages, &[])
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let content = msg
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        let input_tokens = usage
            .usage
            .input_tokens
            .unwrap_or(0)
            .max(0) as usize;
        let output_tokens = usage
            .usage
            .output_tokens
            .unwrap_or(0)
            .max(0) as usize;
        let total_tokens = input_tokens.saturating_add(output_tokens);

        Ok(
            ModelResponse::text(content, &req.model, total_tokens)
                .with_usage(Some(input_tokens), Some(output_tokens), Some(total_tokens))
        )
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let model_config = ModelConfig::new(&req.model);
        let messages = vec![Message::user().with_text(&req.prompt)];
        let mut stream = self
            .inner
            .stream(&model_config, "", &messages, &[])
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let (tx, rx) = mpsc::channel(16);
        let req_id = req.request_id.clone();
        let model_id = req.model.clone();

        tokio::spawn(async move {
            let mut total_output_tokens: usize = 0;
            let mut total_input_tokens: usize = 0;
            let mut accumulated_text = String::new();

            while let Some(item) = stream.next().await {
                match item {
                    Ok((Some(msg), usage)) => {
                        let text = msg
                            .content
                            .iter()
                            .filter_map(|c| c.as_text().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                            .join("");
                        if !text.is_empty() {
                            accumulated_text.push_str(&text);
                            let _ = tx
                                .send(ProviderEvent::chunk(req_id.clone(), text))
                                .await;
                        }
                        if let Some(u) = usage {
                            if let Some(in_tok) = u.usage.input_tokens {
                                total_input_tokens = in_tok.max(0) as usize;
                            }
                            if let Some(out_tok) = u.usage.output_tokens {
                                total_output_tokens = out_tok.max(0) as usize;
                            }
                        }
                    }
                    Ok((None, usage)) => {
                        if let Some(u) = usage {
                            if let Some(in_tok) = u.usage.input_tokens {
                                total_input_tokens = in_tok.max(0) as usize;
                            }
                            if let Some(out_tok) = u.usage.output_tokens {
                                total_output_tokens = out_tok.max(0) as usize;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ProviderEvent::failed(req_id.clone(), e.to_string()))
                            .await;
                        return;
                    }
                }
            }

            let mut completed = ProviderEvent::completed(
                req_id,
                Some(accumulated_text),
                Some(crate::events::TokenUsage {
                    input_tokens: Some(total_input_tokens),
                    output_tokens: Some(total_output_tokens),
                }),
            );
            completed.model_id = Some(model_id);
            let _ = tx.send(completed).await;
        });

        Ok(rx)
    }

    async fn execute_turn(
        &self,
        req: crate::turn::ModelTurnRequest,
    ) -> Result<mpsc::Receiver<crate::turn::ModelTurnEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(16);
        let attempt_id = req.attempt_id;
        let provider_id = self.provider_id().to_string();
        let requested_model = req.requested_model.clone();
        let model_config = ModelConfig::new(&requested_model);

        let inner = self.inner.clone();
        tokio::spawn(async move {
            match inner.stream(&model_config, "", &req.structured_messages, &[]).await {
                Ok(mut stream) => {
                    let mut seq: u64 = 1;
                    let mut total_output_tokens: u64 = 0;
                    let mut total_input_tokens: u64 = 0;

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok((Some(msg), usage)) => {
                                let text = msg
                                    .content
                                    .iter()
                                    .filter_map(|c| c.as_text().map(|s| s.to_string()))
                                    .collect::<Vec<_>>()
                                    .join("");
                                if !text.is_empty() {
                                    let _ = tx.send(crate::turn::ModelTurnEvent {
                                        attempt_id,
                                        sequence: seq,
                                        actual_provider: Some(provider_id.clone()),
                                        actual_model: Some(requested_model.clone()),
                                        actual_transport: Some("provider_bridge".to_string()),
                                        delta: crate::turn::TurnDelta::Text { text },
                                        usage_certainty: Some("exact".to_string()),
                                        provider_response_id: None,
                                    }).await;
                                    seq += 1;
                                }
                                if let Some(u) = usage {
                                    if let Some(in_tok) = u.usage.input_tokens {
                                        total_input_tokens = in_tok.max(0) as u64;
                                    }
                                    if let Some(out_tok) = u.usage.output_tokens {
                                        total_output_tokens = out_tok.max(0) as u64;
                                    }
                                }
                            }
                            Ok((None, usage)) => {
                                if let Some(u) = usage {
                                    if let Some(in_tok) = u.usage.input_tokens {
                                        total_input_tokens = in_tok.max(0) as u64;
                                    }
                                    if let Some(out_tok) = u.usage.output_tokens {
                                        total_output_tokens = out_tok.max(0) as u64;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(crate::turn::ModelTurnEvent {
                                    attempt_id,
                                    sequence: seq,
                                    actual_provider: Some(provider_id.clone()),
                                    actual_model: Some(requested_model.clone()),
                                    actual_transport: None,
                                    delta: crate::turn::TurnDelta::Failed {
                                        error_class: "provider_error".to_string(),
                                        message: e.to_string(),
                                    },
                                    usage_certainty: None,
                                    provider_response_id: None,
                                }).await;
                                return;
                            }
                        }
                    }

                    // Send usage delta
                    let _ = tx.send(crate::turn::ModelTurnEvent {
                        attempt_id,
                        sequence: seq,
                        actual_provider: Some(provider_id.clone()),
                        actual_model: None,
                        actual_transport: None,
                        delta: crate::turn::TurnDelta::Usage {
                            input_tokens: total_input_tokens,
                            output_tokens: total_output_tokens,
                            cache_read_tokens: None,
                            cache_write_tokens: None,
                        },
                        usage_certainty: Some("exact".to_string()),
                        provider_response_id: None,
                    }).await;
                    seq += 1;

                    // Send completed delta
                    let _ = tx.send(crate::turn::ModelTurnEvent {
                        attempt_id,
                        sequence: seq,
                        actual_provider: None,
                        actual_model: None,
                        actual_transport: None,
                        delta: crate::turn::TurnDelta::Completed {
                            finish_reason: Some("stop".to_string()),
                        },
                        usage_certainty: None,
                        provider_response_id: None,
                    }).await;
                }
                Err(e) => {
                    let _ = tx.send(crate::turn::ModelTurnEvent {
                        attempt_id,
                        sequence: 1,
                        actual_provider: Some(provider_id),
                        actual_model: Some(requested_model),
                        actual_transport: None,
                        delta: crate::turn::TurnDelta::Failed {
                            error_class: "provider_error".to_string(),
                            message: e.to_string(),
                        },
                        usage_certainty: None,
                        provider_response_id: None,
                    }).await;
                }
            }
        });

        Ok(rx)
    }
}

/// Adapts an `Arc<dyn ModelProvider>` (`ModelPort`) into a compatibility `Provider`.
pub struct ModelPortToProvider {
    inner: Arc<dyn ModelProvider>,
    context_limit: usize,
}

impl ModelPortToProvider {
    pub fn new(inner: Arc<dyn ModelProvider>) -> Self {
        Self {
            inner,
            context_limit: 128_000,
        }
    }

    pub fn with_context_limit(mut self, limit: usize) -> Self {
        self.context_limit = limit;
        self
    }

    pub fn inner(&self) -> &Arc<dyn ModelProvider> {
        &self.inner
    }

    fn build_prompt(system: &str, messages: &[Message]) -> String {
        let mut prompt = String::new();
        if !system.trim().is_empty() {
            prompt.push_str("System: ");
            prompt.push_str(system);
            prompt.push_str("\n\n");
        }
        for msg in messages {
            let role_str = match msg.role {
                rmcp::model::Role::User => "User",
                rmcp::model::Role::Assistant => "Assistant",
            };
            let text = msg
                .content
                .iter()
                .filter_map(|c| c.as_text())
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                prompt.push_str(role_str);
                prompt.push_str(": ");
                prompt.push_str(&text);
                prompt.push('\n');
            }
        }
        prompt
    }
}

#[async_trait]
impl Provider for ModelPortToProvider {
    fn get_name(&self) -> &str {
        self.inner.provider_id()
    }

    async fn complete(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[rmcp::model::Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let prompt = Self::build_prompt(system, messages);
        let req = ProviderRequest::new(
            custos_domain::new_id("req_bridge"),
            custos_domain::new_id("task_bridge"),
            1,
            prompt,
            model_config.model_name.clone(),
        );

        let resp = self
            .inner
            .generate(&req)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        let msg = Message::assistant().with_text(resp.content);
        let in_tok = resp.input_tokens.unwrap_or(0);
        let out_tok = resp.output_tokens.unwrap_or(0);
        let usage = ProviderUsage::new(
            model_config.model_name.clone(),
            Usage::new(
                Some(in_tok as i32),
                Some(out_tok as i32),
                None,
            ),
        );
        Ok((msg, usage))
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError> {
        let prompt = Self::build_prompt(system, messages);
        let req = ProviderRequest::new(
            custos_domain::new_id("req_bridge"),
            custos_domain::new_id("task_bridge"),
            1,
            prompt,
            model_config.model_name.clone(),
        );

        let mut rx = self
            .inner
            .stream(&req)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        let model_name = model_config.model_name.clone();
        let s = async_stream::stream! {
            while let Some(event) = rx.recv().await {
                match event.event_type {
                    crate::events::ProviderEventType::Chunk => {
                        if let Some(chunk_text) = event.text {
                            yield Ok((Some(Message::assistant().with_text(chunk_text)), None));
                        }
                    }
                    crate::events::ProviderEventType::Completed => {
                        let usage = event.usage.map(|u| {
                            ProviderUsage::new(
                                model_name.clone(),
                                Usage::new(
                                    u.input_tokens.map(|t| t as i32),
                                    u.output_tokens.map(|t| t as i32),
                                    None,
                                ),
                            )
                        });
                        yield Ok((None, usage));
                        break;
                    }
                    crate::events::ProviderEventType::Failed => {
                        yield Err(ProviderError::ExecutionError(
                            event.error.unwrap_or_else(|| "Provider stream failed".to_string())
                        ));
                        break;
                    }
                    _ => {}
                }
            }
        };

        Ok(Box::pin(s))
    }

    async fn get_context_limit(&self, _model: &str, override_limit: Option<usize>) -> usize {
        override_limit.unwrap_or(self.context_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn get_name(&self) -> &str {
            "mock-provider"
        }

        async fn complete(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[rmcp::model::Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            let msg = Message::assistant().with_text("mocked response");
            let usage = ProviderUsage::new("mock-model".to_string(), Usage::new(Some(10), Some(20), None));
            Ok((msg, usage))
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[rmcp::model::Tool],
        ) -> Result<MessageStream, ProviderError> {
            let s = async_stream::stream! {
                yield Ok((Some(Message::assistant().with_text("streamed")), None));
            };
            Ok(Box::pin(s))
        }

        async fn get_context_limit(&self, _model: &str, override_limit: Option<usize>) -> usize {
            override_limit.unwrap_or(4096)
        }
    }

    struct MockModelPort;

    #[async_trait]
    impl ModelProvider for MockModelPort {
        fn provider_id(&self) -> &str {
            "mock-port"
        }

        async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            Ok(ModelResponse::text(format!("echo: {}", req.prompt), &req.model, 30)
                .with_usage(Some(10), Some(20), Some(30)))
        }

        async fn stream(
            &self,
            req: &ProviderRequest,
        ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
            let (tx, rx) = mpsc::channel(4);
            let req_id = req.request_id.clone();
            tokio::spawn(async move {
                let _ = tx.send(ProviderEvent::chunk(req_id.clone(), "chunk1")).await;
                let _ = tx.send(ProviderEvent::completed(req_id, Some("chunk1".to_string()), None)).await;
            });
            Ok(rx)
        }
    }

    #[tokio::test]
    async fn test_provider_to_model_port_roundtrip() {
        let provider = Arc::new(MockProvider);
        let port = ProviderToModelPort::new(provider);

        assert_eq!(port.provider_id(), "mock-provider");

        let req = ProviderRequest::new("req_1", "task_1", 1, "hello", "test-model");
        let resp = port.generate(&req).await.unwrap();
        assert_eq!(resp.content, "mocked response");
        assert_eq!(resp.input_tokens, Some(10));
        assert_eq!(resp.output_tokens, Some(20));
    }

    #[tokio::test]
    async fn test_model_port_to_provider_roundtrip() {
        let port = Arc::new(MockModelPort);
        let provider = ModelPortToProvider::new(port);

        assert_eq!(provider.get_name(), "mock-port");

        let config = ModelConfig::new("test-model");
        let messages = vec![Message::user().with_text("ping")];
        let (msg, usage) = provider.complete(&config, "system prompt", &messages, &[]).await.unwrap();

        let text = msg.content[0].as_text().unwrap();
        assert!(text.contains("echo: System: system prompt"));
        assert!(text.contains("User: ping"));
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(20));
    }
}

