use super::super::*;
use crate::conversation::message::{Message, MessageContent, MessageContentBlock, ToolRequest};
use crate::conversation::token_usage::ProviderUsage;
use crate::errors::ProviderError;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use anyhow::Result;
use rmcp::model::Tool;
use serde_json::Value;

pub(crate) fn create_request_with_default_options(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    create_request_with_options_provider(
        model_config,
        system,
        messages,
        tools,
        AnthropicFormatOptions::default(),
    )
}

pub(crate) fn create_request_with_options_provider(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    options: AnthropicFormatOptions,
) -> Result<Value> {
    create_request(
        ANTHROPIC_PROVIDER_NAME,
        model_config,
        system,
        messages,
        tools,
        options,
    )
}

pub(crate) fn signed_thinking_from_model(model: &str) -> Message {
    use crate::conversation::message::InferenceMetadata;
    Message::assistant()
        .with_content(MessageContent::thinking("internal", "sig-abc"))
        .with_text("answer")
        .with_inference(InferenceMetadata {
            provider: "anthropic".to_string(),
            requested_model: model.to_string(),
            resolved_model: None,
            provider_session_id: None,
        })
}

pub(crate) fn cfg(name: &str) -> ModelConfig {
    ModelConfig::new(name)
}

pub(crate) fn cfg_with_effort(name: &str, effort: &str) -> ModelConfig {
    let effort: ThinkingEffort = effort.parse().unwrap();
    cfg(name).with_thinking_effort(effort)
}

#[derive(Default)]
pub(crate) struct StreamedParts {
    pub(crate) thinking: Vec<(String, String)>,
    pub(crate) redacted_thinking: Vec<String>,
    pub(crate) text: Vec<String>,
    pub(crate) tool_calls: Vec<String>,
    pub(crate) tool_errors: Vec<String>,
    pub(crate) output_token_limit_message_ids: Vec<Option<String>>,
}

pub(crate) async fn collect_stream(events: &str) -> StreamedParts {
    let mut parts = StreamedParts::default();

    for result in collect_stream_results(events).await {
        if let Ok((Some(msg), _usage)) = result {
            if msg.metadata.output_token_limit_reached {
                parts.output_token_limit_message_ids.push(msg.id.clone());
            }
            for c in &msg.content {
                match c {
                    MessageContentBlock::Thinking(t) => {
                        parts
                            .thinking
                            .push((t.thinking.clone(), t.signature.clone()));
                    }
                    MessageContentBlock::RedactedThinking(r) => {
                        parts.redacted_thinking.push(r.data.clone());
                    }
                    MessageContentBlock::Text(t) => {
                        parts.text.push(t.text.clone());
                    }
                    MessageContentBlock::ToolRequest(req) => match &req.tool_call {
                        Ok(call) => parts.tool_calls.push(call.name.to_string()),
                        Err(e) => parts.tool_errors.push(e.message.to_string()),
                    },
                    _ => {}
                }
            }
        }
    }
    parts
}

pub(crate) async fn collect_stream_results(
    events: &str,
) -> Vec<anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> {
    use futures::StreamExt;

    let lines: Vec<Result<String, anyhow::Error>> =
        events.lines().map(|l| Ok(l.to_string())).collect();
    let stream = Box::pin(futures::stream::iter(lines));
    response_to_streaming_message(stream).collect().await
}

pub(crate) async fn collect_tool_requests(events: &str) -> Vec<ToolRequest> {
    let mut requests = Vec::new();
    for result in collect_stream_results(events).await {
        if let Ok((Some(msg), _)) = result {
            for c in msg.content {
                if let MessageContentBlock::ToolRequest(req) = c {
                    requests.push(req);
                }
            }
        }
    }
    requests
}

pub(crate) async fn streamed_usage(events: &str) -> ProviderUsage {
    let results = collect_stream_results(events).await;
    results
        .into_iter()
        .find_map(|r| r.ok().and_then(|(_, u)| u))
        .expect("expected usage")
}

pub(crate) fn expect_refusal(
    results: Vec<anyhow::Result<(Option<Message>, Option<ProviderUsage>)>>,
) -> (String, Option<String>) {
    let err = results
        .into_iter()
        .find_map(|r| r.err())
        .expect("refusal should surface as a stream error");
    match err.downcast_ref::<ProviderError>() {
        Some(ProviderError::Refusal { details, category }) => {
            (details.clone(), category.clone())
        }
        other => panic!("expected ProviderError::Refusal, got {:?}", other),
    }
}
