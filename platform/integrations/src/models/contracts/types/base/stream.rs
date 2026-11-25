//! Streaming types and response aggregation utilities.

use crate::{
    conversation::{
        message::{Message, MessageContentBlock},
        token_usage::{ProviderUsage, Usage},
    },
    errors::ProviderError,
};
use futures::Stream;
use std::pin::Pin;

/// A message stream yields partial text content but complete tool calls, all within the Message object
pub type MessageStream = Pin<
    Box<dyn Stream<Item = Result<(Option<Message>, Option<ProviderUsage>), ProviderError>> + Send>,
>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionRouting {
    ActionRequired,
    Noop,
}

/// Collect all chunks from a MessageStream into a single Message and ProviderUsage
pub async fn collect_stream(
    mut stream: MessageStream,
) -> Result<(Message, ProviderUsage), ProviderError> {
    use futures::StreamExt;

    let mut final_message: Option<Message> = None;
    let mut final_usage: Option<ProviderUsage> = None;

    while let Some(result) = stream.next().await {
        let (msg_opt, usage_opt) = result?;

        if let Some(msg) = msg_opt {
            final_message = Some(match final_message {
                Some(mut prev) => {
                    let is_single_block_delta = msg.content.len() == 1;
                    for new_content in msg.content {
                        match (&mut prev.content.last_mut(), &new_content) {
                            // Coalesce consecutive text blocks
                            (
                                Some(MessageContentBlock::Text(last_text)),
                                MessageContentBlock::Text(new_text),
                            ) if last_text
                                .annotations
                                .as_ref()
                                .and_then(|a| a.audience.as_ref())
                                == new_text
                                    .annotations
                                    .as_ref()
                                    .and_then(|a| a.audience.as_ref()) =>
                            {
                                last_text.text.push_str(&new_text.text);
                            }
                            // Coalesce consecutive thinking blocks
                            (
                                Some(MessageContentBlock::Thinking(last_thinking)),
                                MessageContentBlock::Thinking(new_thinking),
                            ) if is_single_block_delta
                                && (last_thinking.signature.is_empty()
                                    || new_thinking.signature == last_thinking.signature) =>
                            {
                                last_thinking.thinking.push_str(&new_thinking.thinking);
                                if !new_thinking.signature.is_empty() {
                                    last_thinking.signature = new_thinking.signature.clone();
                                }
                            }
                            _ => {
                                prev.content.push(new_content);
                            }
                        }
                    }
                    prev
                }
                None => msg,
            });
        }

        if let Some(usage) = usage_opt {
            final_usage = Some(usage);
        }
    }

    match (final_message, final_usage) {
        (Some(msg), usage) => {
            let usage = usage
                .unwrap_or_else(|| ProviderUsage::new("unknown".to_string(), Usage::default()));
            Ok((msg, usage))
        }
        (None, Some(usage)) => Ok((
            Message::new(
                rmcp::model::Role::Assistant,
                chrono::Utc::now().timestamp(),
                Vec::new(),
            ),
            usage,
        )),
        (None, None) => Err(ProviderError::ExecutionError(
            "Stream yielded no message".to_string(),
        )),
    }
}

pub fn stream_from_single_message(message: Message, usage: ProviderUsage) -> MessageStream {
    let stream = futures::stream::once(async move { Ok((Some(message), Some(usage))) });
    Box::pin(stream)
}
