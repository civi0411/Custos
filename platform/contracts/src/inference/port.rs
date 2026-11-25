//! Provider Port Trait
//!
//! Contract defining how the Custos runtime interacts with AI model providers.

use crate::identity::error::DomainError;
use crate::inference::events::ProviderEvent;
use crate::inference::request::{ModelResponse, ProviderRequest};
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Unique provider identifier (e.g., "fake", "claude", "codex", "antigravity").
    fn provider_id(&self) -> &str;

    /// Unary synchronous generation call.
    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError>;

    /// Streaming generation returning an asynchronous receiver of ProviderEvents.
    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(4);
        let resp = self.generate(req).await?;
        let req_id = req.request_id.clone();
        tokio::spawn(async move {
            let mut event = ProviderEvent::completed(
                req_id,
                Some(resp.content),
                Some(crate::inference::events::TokenUsage {
                    input_tokens: resp.input_tokens,
                    output_tokens: resp.output_tokens,
                }),
            );
            event.model_id = Some(resp.model_id);
            event.tokens_used = Some(resp.tokens_used);
            let _ = tx.send(event).await;
        });
        Ok(rx)
    }

    /// Structured Model Turn execution (supporting tools, reasoning, token delta streaming).
    /// Default implementation bridges to `generate` for backwards-compatibility.
    async fn execute_turn(
        &self,
        req: crate::inference::turn::ModelTurnRequest,
    ) -> Result<mpsc::Receiver<crate::inference::turn::ModelTurnEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(16);
        let attempt_id = req.attempt_id;
        let provider_id = self.provider_id().to_string();
        let requested_model = req.requested_model.clone();

        let flat_prompt = req
            .structured_messages
            .iter()
            .map(|m| {
                if let Some(s) = m.as_str() {
                    s.to_string()
                } else if let Some(content) = m.get("content") {
                    if let Some(text) = content.as_str() {
                        text.to_string()
                    } else if let Some(arr) = content.as_array() {
                        arr.iter()
                            .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        content.to_string()
                    }
                } else {
                    m.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let legacy_req = crate::inference::request::ProviderRequest::new(
            req.attempt_id.to_string(),
            req.task_id.to_string(),
            1,
            flat_prompt,
            req.requested_model.clone(),
        );

        let resp_res = self.generate(&legacy_req).await;
        tokio::spawn(async move {
            match resp_res {
                Ok(resp) => {
                    let _ = tx
                        .send(crate::inference::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 1,
                            actual_provider: Some(provider_id.clone()),
                            actual_model: Some(requested_model),
                            actual_transport: Some("legacy_bridge".to_string()),
                            delta: crate::inference::turn::TurnDelta::Text { text: resp.content },
                            usage_certainty: Some("estimated".to_string()),
                            provider_response_id: None,
                        })
                        .await;
                    let _ = tx
                        .send(crate::inference::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 2,
                            actual_provider: Some(provider_id),
                            actual_model: None,
                            actual_transport: None,
                            delta: crate::inference::turn::TurnDelta::Usage {
                                input_tokens: 0,
                                output_tokens: resp.tokens_used as u64,
                                cache_read_tokens: None,
                                cache_write_tokens: None,
                            },
                            usage_certainty: Some("estimated".to_string()),
                            provider_response_id: None,
                        })
                        .await;
                    let _ = tx
                        .send(crate::inference::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 3,
                            actual_provider: None,
                            actual_model: None,
                            actual_transport: None,
                            delta: crate::inference::turn::TurnDelta::Completed {
                                finish_reason: Some("stop".to_string()),
                            },
                            usage_certainty: None,
                            provider_response_id: None,
                        })
                        .await;
                }
                Err(err) => {
                    let _ = tx
                        .send(crate::inference::turn::ModelTurnEvent {
                            attempt_id,
                            sequence: 1,
                            actual_provider: Some(provider_id),
                            actual_model: Some(requested_model),
                            actual_transport: None,
                            delta: crate::inference::turn::TurnDelta::Failed {
                                error_class: "bridge_error".to_string(),
                                message: err.to_string(),
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

pub use ModelProvider as ModelPort;
