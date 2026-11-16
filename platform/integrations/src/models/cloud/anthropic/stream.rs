use anyhow::Result;
use async_stream::try_stream;
use futures::TryStreamExt;
use serde_json::{json, Value};
use std::io;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::types::AnthropicProvider;
use crate::cloud::transport::api_client::ApiClient;
use crate::base::MessageStream;
use crate::cloud::transport::openai_compatible::handle_status;
use crate::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::cloud::codecs::anthropic::{
    block_binding_behavior, create_request_for_model, is_thinking_signature_error,
    response_to_streaming_message, AnthropicFormatOptions, PrefixMismatchBehavior,
    ANTHROPIC_PROVIDER_NAME, INPUT_TRANSFORMATIONS_FIELD, THINKING_BINDING_CONTROLS_BETA,
};
use crate::model::ModelConfig;
use crate::request_log::{start_log, LoggerHandleExt};
use rmcp::model::Tool;

impl AnthropicProvider {
    pub(crate) fn streaming_payload(
        &self,
        model_config: &ModelConfig,
        wire_model: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        format_options: AnthropicFormatOptions,
    ) -> Result<Value, ProviderError> {
        let mut payload = create_request_for_model(
            ANTHROPIC_PROVIDER_NAME,
            model_config,
            wire_model,
            system,
            messages,
            tools,
            format_options,
        )?;
        payload["stream"] = Value::Bool(true);
        Ok(payload)
    }

    pub(crate) async fn post_messages(
        &self,
        model_config: &ModelConfig,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let beta_header = beta_header_value(&self.api_client, model_config, payload);
        self.with_retry(|| async {
            let mut request = self
                .api_client
                .request("v1/messages")
                .model_headers(model_config)?;
            if let Some(beta) = &beta_header {
                request = request.header("anthropic-beta", beta)?;
            }
            handle_status(request.streaming(true).response_post(payload).await?).await
        })
        .await
    }

    pub async fn stream_for_model(
        &self,
        model_config: &ModelConfig,
        wire_model: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = self.streaming_payload(
            model_config,
            wire_model,
            system,
            messages,
            tools,
            self.format_options.clone(),
        )?;
        let mut log = start_log(model_config, &payload)?;
        let response = match self.post_messages(model_config, &payload).await {
            Err(ProviderError::RequestFailed(message))
                if is_thinking_signature_error(&message)
                    && !self.format_options.strip_thinking_history
                    && block_binding_behavior(&payload) != Some(PrefixMismatchBehavior::Error) =>
            {
                tracing::warn!(
                    error = %message,
                    "API rejected replayed thinking blocks; retrying once with thinking history stripped. \
                     The rejected blocks stay in the session, so later requests may repeat this retry"
                );
                let _ = log.error(&message);
                let stripped = AnthropicFormatOptions {
                    strip_thinking_history: true,
                    ..self.format_options.clone()
                };
                let payload =
                    self.streaming_payload(model_config, wire_model, system, messages, tools, stripped)?;
                log = start_log(model_config, &payload)?;
                self.post_messages(model_config, &payload).await
            }
            other => other,
        }
        .inspect_err(|e| {
            let _ = log.error(e);
        })?;
        let stream = response.bytes_stream().map_err(io::Error::other);
        Ok(Box::pin(try_stream! {
            let reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);
            let messages = response_to_streaming_message(framed);
            pin!(messages);
            while let Some(message) = futures::StreamExt::next(&mut messages).await {
                let (message, usage) = message.map_err(ProviderError::from_stream_error)?;
                if let Some(transformations) = usage
                    .as_ref()
                    .and_then(|usage| usage.additional_data.as_ref())
                    .and_then(|data| data.get(INPUT_TRANSFORMATIONS_FIELD))
                {
                    log.write(&json!({ INPUT_TRANSFORMATIONS_FIELD: transformations }), None)?;
                }
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}

pub(crate) fn beta_header_value(
    api_client: &ApiClient,
    model_config: &ModelConfig,
    payload: &Value,
) -> Option<String> {
    let mut features: Vec<String> = model_config
        .request_headers
        .as_ref()
        .and_then(|headers| {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("anthropic-beta"))
                .map(|(_, value)| value.as_str())
        })
        .or_else(|| api_client.default_header("anthropic-beta"))
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if block_binding_behavior(payload).is_some()
        && !features.iter().any(|f| f == THINKING_BINDING_CONTROLS_BETA)
    {
        features.push(THINKING_BINDING_CONTROLS_BETA.to_string());
    }
    (!features.is_empty()).then(|| features.join(","))
}
