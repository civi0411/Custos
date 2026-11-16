use super::super::*;
use crate::conversation::message::MessageContentBlock;
use tokio_stream::{self, StreamExt};

#[tokio::test]
async fn test_streaming_response_extracts_inline_think_blocks() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"<thi\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"nk>x</thi\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"nk>y\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n",
        "data: [DONE]\n"
    );

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut text = String::new();
    let mut thinking = String::new();

    while let Some(result) = messages.next().await {
        let (message, _) = result?;
        if let Some(message) = message {
            for item in message.content {
                match item {
                    MessageContentBlock::Text(text_content) => {
                        text.push_str(&text_content.text)
                    }
                    MessageContentBlock::Thinking(thinking_content) => {
                        thinking.push_str(&thinking_content.thinking)
                    }
                    _ => {}
                }
            }
        }
    }

    assert_eq!(text, "y");
    assert_eq!(thinking, "x");

    Ok(())
}

#[tokio::test]
async fn test_streaming_truncated_inline_think_preserves_output_token_limit(
) -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"chunk-1\",\"model\":\"test-model\",\"choices\":[{\"delta\":{\"content\":\"<think>unfinished reasoning\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{},\"index\":0,\"finish_reason\":\"length\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n",
        "data: [DONE]\n"
    );

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
    let mut streamed_messages = Vec::new();
    let mut usage_count = 0;

    while let Some(result) = messages.next().await {
        let (message, usage) = result?;
        if usage.is_some() {
            usage_count += 1;
        }
        if let Some(message) = message {
            streamed_messages.push(message);
        }
    }

    assert_eq!(usage_count, 1);
    assert_eq!(
        streamed_messages
            .iter()
            .filter(|message| message.metadata.output_token_limit_reached)
            .count(),
        1
    );
    let trailing_message = streamed_messages
        .last()
        .expect("expected trailing thinking");
    assert!(trailing_message.metadata.output_token_limit_reached);
    assert!(matches!(
        trailing_message.content.as_slice(),
        [MessageContentBlock::Thinking(thinking)]
            if thinking.thinking == "unfinished reasoning"
    ));

    Ok(())
}

#[tokio::test]
async fn test_streaming_partial_think_tag_emits_one_output_limit_marker() -> anyhow::Result<()>
{
    let response_lines = concat!(
        "data: {\"id\":\"chunk-1\",\"model\":\"test-model\",\"choices\":[{\"delta\":{\"content\":\"<thi\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{},\"index\":0,\"finish_reason\":\"length\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n",
        "data: [DONE]\n"
    );

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
    let mut streamed_messages = Vec::new();
    let mut usage_count = 0;

    while let Some(result) = messages.next().await {
        let (message, usage) = result?;
        usage_count += usize::from(usage.is_some());
        if let Some(message) = message {
            streamed_messages.push(message);
        }
    }

    assert_eq!(usage_count, 1);
    let marked_messages: Vec<_> = streamed_messages
        .iter()
        .filter(|message| message.metadata.output_token_limit_reached)
        .collect();
    assert_eq!(marked_messages.len(), 1);
    assert_eq!(marked_messages[0].id.as_deref(), Some("chunk-1"));
    assert_eq!(marked_messages[0].as_concat_text(), "<thi");

    Ok(())
}

#[tokio::test]
async fn test_streaming_suppresses_inline_think_when_structured_reasoning_follows(
) -> anyhow::Result<()> {
    // Inline <think>...</think> arrives in an early content chunk, then
    // reasoning_content arrives in a later chunk. The inline thinking
    // should be discarded in favor of the structured reasoning so users
    // do not get duplicated reasoning output.
    let response_lines = concat!(
        "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"<think>inline reasoning</think>Hi\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-2\",\"choices\":[{\"delta\":{\"reasoning_content\":\"structured reasoning\"},\"index\":0,\"finish_reason\":null}]}\n",
        "data: {\"id\":\"chunk-3\",\"choices\":[{\"delta\":{\"content\":\" there\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n",
        "data: [DONE]\n"
    );

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut text = String::new();
    let mut thinking = String::new();

    while let Some(result) = messages.next().await {
        let (message, _) = result?;
        if let Some(message) = message {
            for item in message.content {
                match item {
                    MessageContentBlock::Text(text_content) => {
                        text.push_str(&text_content.text)
                    }
                    MessageContentBlock::Thinking(thinking_content) => {
                        thinking.push_str(&thinking_content.thinking)
                    }
                    _ => {}
                }
            }
        }
    }

    assert_eq!(text, "Hi there");
    assert_eq!(thinking, "structured reasoning");

    Ok(())
}

