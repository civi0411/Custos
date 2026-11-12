use async_trait::async_trait;
use std::sync::Arc;

use crate::base::{MessageStream, Provider};
use crate::conversation::message::Message;
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;
use crate::model::ModelConfig;
use crate::port::ModelProvider;
use crate::request::ProviderRequest;

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
            Usage::new(Some(in_tok as i32), Some(out_tok as i32), None),
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

