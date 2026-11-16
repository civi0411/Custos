//! Bidirectional bridge between the canonical hexagonal `ModelPort` (`ModelProvider`)
//! and the compatibility `Provider` (Goose/MCP conversation provider) surface.

use async_trait::async_trait;
use custos_domain::DomainError;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::base::Provider;
use crate::conversation::message::Message;
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

        let input_tokens = usage.usage.input_tokens.unwrap_or(0).max(0) as usize;
        let output_tokens = usage.usage.output_tokens.unwrap_or(0).max(0) as usize;
        let total_tokens = input_tokens.saturating_add(output_tokens);

        Ok(
            ModelResponse::text(content, &req.model, total_tokens).with_usage(
                Some(input_tokens),
                Some(output_tokens),
                Some(total_tokens),
            ),
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
                            let _ = tx.send(ProviderEvent::chunk(req_id.clone(), text)).await;
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
            match inner
                .stream(&model_config, "", &req.structured_messages, &[])
                .await
            {
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
                                    let _ = tx
                                        .send(crate::turn::ModelTurnEvent {
                                            attempt_id,
                                            sequence: seq,
                                            actual_provider: Some(provider_id.clone()),
                                            actual_model: Some(requested_model.clone()),
                                            actual_transport: Some("provider_bridge".to_string()),
                                            delta: crate::turn::TurnDelta::Text { text },
                                            usage_certainty: Some("exact".to_string()),
                                            provider_response_id: None,
                                        })
                                        .await;
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
                                let _ = tx
                                    .send(crate::turn::ModelTurnEvent {
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
                                    })
                                    .await;
                                return;
                            }
                        }
                    }

                    // Send usage delta
                    let _ = tx
                        .send(crate::turn::ModelTurnEvent {
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
                        })
                        .await;
                    seq += 1;

                    // Send completed delta
                    let _ = tx
                        .send(crate::turn::ModelTurnEvent {
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
                        })
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(crate::turn::ModelTurnEvent {
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
                        })
                        .await;
                }
            }
        });

        Ok(rx)
    }
}

#[path = "bridge_outbound.rs"]
mod bridge_outbound;
pub use bridge_outbound::ModelPortToProvider;

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;
