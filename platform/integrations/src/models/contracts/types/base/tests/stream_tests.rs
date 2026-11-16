use crate::base::collect_stream;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;
use futures::Stream;
use test_case::test_case;

fn content_from_str(s: String) -> MessageContentBlock {
    if let Some(img_data) = s.strip_prefix("*img:") {
        MessageContentBlock::image(format!("http://example.com/{}", img_data), "image/png")
    } else if let Some(tool_name) = s.strip_prefix("*tool:") {
        let tool_call = Ok(
            rmcp::model::CallToolRequestParams::new(tool_name.to_string())
                .with_arguments(serde_json::Map::new()),
        );
        MessageContentBlock::tool_request(format!("tool_{}", tool_name), tool_call)
    } else {
        MessageContentBlock::text(s)
    }
}

fn create_test_stream(
    items: Vec<String>,
) -> impl Stream<Item = Result<(Option<Message>, Option<ProviderUsage>), ProviderError>> {
    use futures::stream;
    stream::iter(items.into_iter().map(|item| {
        let content = content_from_str(item);
        let message = Message::new(
            rmcp::model::Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![content],
        );
        Ok((Some(message), None))
    }))
}

fn content_to_strings(msg: &Message) -> Vec<String> {
    msg.content
        .iter()
        .map(|c| match c {
            MessageContentBlock::Text(t) => t.text.clone(),
            MessageContentBlock::Image(_) => "*img".to_string(),
            MessageContentBlock::ToolRequest(tr) => {
                if let Ok(call) = &tr.tool_call {
                    format!("*tool:{}", call.name)
                } else {
                    "*tool:error".to_string()
                }
            }
            _ => "*other".to_string(),
        })
        .collect()
}

#[test_case(
    vec!["Hello", " ", "world"],
    vec!["Hello world"]
    ; "consecutive text coalesces"
)]
#[test_case(
    vec!["Hello", "*img:pic1", "world"],
    vec!["Hello", "*img", "world"]
    ; "non-text breaks coalescing"
)]
#[test_case(
    vec!["A", "B", "*img:pic1", "C", "D", "*tool:read", "E", "F"],
    vec!["AB", "*img", "CD", "*tool:read", "EF"]
    ; "multiple text groups"
)]
#[test_case(
    vec!["Text1", "*img:pic", "Text2"],
    vec!["Text1", "*img", "Text2"]
    ; "mixed content in chunk"
)]
#[tokio::test]
async fn test_collect_stream_coalescing(input_items: Vec<&str>, expected: Vec<&str>) {
    let items: Vec<String> = input_items.into_iter().map(|s| s.to_string()).collect();
    let stream = create_test_stream(items);
    let (msg, _) = collect_stream(Box::pin(stream)).await.unwrap();
    assert_eq!(content_to_strings(&msg), expected);
}

#[tokio::test]
async fn test_collect_stream_defaults_usage() {
    let stream = create_test_stream(vec!["Hello".to_string()]);
    let (msg, usage) = collect_stream(Box::pin(stream)).await.unwrap();
    assert_eq!(content_to_strings(&msg), vec!["Hello"]);
    assert_eq!(usage.model, "unknown");
}

#[tokio::test]
async fn test_collect_stream_usage_only_yields_empty_message() {
    let usage = ProviderUsage::new("claude-sonnet-4".to_string(), Usage::default());
    let stream = futures::stream::once(async move { Ok((None, Some(usage))) });
    let (msg, usage) = collect_stream(Box::pin(stream)).await.unwrap();
    assert!(msg.content.is_empty());
    assert_eq!(usage.model, "claude-sonnet-4");
}

#[tokio::test]
async fn test_collect_stream_no_message_no_usage_errors() {
    let stream = futures::stream::empty();
    let result = collect_stream(Box::pin(stream)).await;
    assert!(matches!(result, Err(ProviderError::ExecutionError(_))));
}

#[tokio::test]
async fn test_collect_stream_preserves_text_audience_boundaries() {
    use futures::stream;
    use rmcp::model::{Annotations, Role, TextContent};

    let message = |text: &str, audience| {
        Message::assistant().with_content(MessageContentBlock::Text(
            TextContent::new(text)
                .with_annotations(Annotations::default().with_audience(vec![audience])),
        ))
    };
    let stream = stream::iter([
        Ok((Some(message("public", Role::User)), None)),
        Ok((Some(message("private", Role::Assistant)), None)),
    ]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(message.content.len(), 2);
    assert_eq!(message.user_visible_content().as_concat_text(), "public");
    assert_eq!(message.agent_visible_content().as_concat_text(), "private");
}

#[tokio::test]
async fn test_collect_stream_coalesces_thinking_deltas() {
    use futures::stream;

    let delta = |t: &str| Ok((Some(Message::assistant().with_thinking(t, "")), None));
    let stream = stream::iter([delta("Thinking"), delta(" Process"), delta(":")]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(
        message.content.len(),
        1,
        "thinking deltas must coalesce into one block, got {:?}",
        message.content
    );
    match &message.content[0] {
        MessageContentBlock::Thinking(t) => assert_eq!(t.thinking, "Thinking Process:"),
        other => panic!("expected Thinking, got {:?}", other),
    }
}

#[tokio::test]
async fn test_collect_stream_never_merges_distinctly_signed_thinking_blocks() {
    use futures::stream;

    let delta = |t: &str, sig: &str| Ok((Some(Message::assistant().with_thinking(t, sig)), None));
    let stream = stream::iter([delta("first", "sig-a"), delta("second", "sig-b")]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(message.content.len(), 2);
    match (&message.content[0], &message.content[1]) {
        (MessageContentBlock::Thinking(a), MessageContentBlock::Thinking(b)) => {
            assert_eq!(a.thinking, "first");
            assert_eq!(a.signature, "sig-a");
            assert_eq!(b.thinking, "second");
            assert_eq!(b.signature, "sig-b");
        }
        other => panic!("expected two Thinking blocks, got {:?}", other),
    }
}

#[tokio::test]
async fn test_collect_stream_unsigned_body_adopts_closing_signature() {
    use futures::stream;

    let delta = |t: &str, sig: &str| Ok((Some(Message::assistant().with_thinking(t, sig)), None));
    let stream = stream::iter([
        delta("Thinking", ""),
        delta(" more", ""),
        delta("", "sig-final"),
    ]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(message.content.len(), 1);
    match &message.content[0] {
        MessageContentBlock::Thinking(t) => {
            assert_eq!(t.thinking, "Thinking more");
            assert_eq!(t.signature, "sig-final");
        }
        other => panic!("expected Thinking, got {:?}", other),
    }
}

#[tokio::test]
async fn test_collect_stream_unsigned_thinking_after_signed_starts_a_new_block() {
    use futures::stream;

    let delta = |t: &str, sig: &str| Ok((Some(Message::assistant().with_thinking(t, sig)), None));
    let stream = stream::iter([delta("first", "sig-a"), delta("second", "")]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(message.content.len(), 2);
    match (&message.content[0], &message.content[1]) {
        (MessageContentBlock::Thinking(a), MessageContentBlock::Thinking(b)) => {
            assert_eq!(a.thinking, "first");
            assert_eq!(a.signature, "sig-a");
            assert_eq!(b.thinking, "second");
            assert_eq!(b.signature, "");
        }
        other => panic!("expected two Thinking blocks, got {:?}", other),
    }
}

#[tokio::test]
async fn test_collect_stream_multi_block_chunk_does_not_merge_into_prior_thinking() {
    use futures::stream;

    let unsigned_delta = |t: &str| Ok((Some(Message::assistant().with_thinking(t, "")), None));
    let multi_block = Ok((
        Some(
            Message::assistant()
                .with_thinking("next", "sig")
                .with_text("after"),
        ),
        None,
    ));
    let stream = stream::iter([unsigned_delta("prior"), multi_block]);

    let (message, _) = collect_stream(Box::pin(stream)).await.unwrap();

    assert_eq!(message.content.len(), 3, "got {:?}", message.content);
    match (
        &message.content[0],
        &message.content[1],
        &message.content[2],
    ) {
        (
            MessageContentBlock::Thinking(a),
            MessageContentBlock::Thinking(b),
            MessageContentBlock::Text(c),
        ) => {
            assert_eq!(a.thinking, "prior");
            assert_eq!(
                a.signature, "",
                "prior block must stay unsigned and unmerged"
            );
            assert_eq!(b.thinking, "next");
            assert_eq!(b.signature, "sig");
            assert_eq!(c.text, "after");
        }
        other => panic!("expected Thinking, Thinking, Text, got {:?}", other),
    }
}
