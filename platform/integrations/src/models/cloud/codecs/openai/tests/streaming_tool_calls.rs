use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::images::ImageFormat;
use rmcp::object;
use tokio_stream::{self, StreamExt};

#[tokio::test]
async fn test_streaming_chunk_with_only_reasoning_content() -> anyhow::Result<()> {
    let response_lines = "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"hi\"},\"finish_reason\":null}]}\ndata: [DONE]";
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
    while let Some(result) = messages.next().await {
        result?;
    }
    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_does_not_duplicate_yielded_reasoning() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think \"},\"finish_reason\":null}]}\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"once\"},\"finish_reason\":null}]}\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut thinking = String::new();
    let mut tool_calls = 0;
    let mut history = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                match content {
                    MessageContentBlock::Thinking(t) => thinking.push_str(&t.thinking),
                    MessageContentBlock::ToolRequest(_) => tool_calls += 1,
                    _ => {}
                }
            }
            history.push(msg);
        }
    }

    assert_eq!(thinking, "think once");
    assert_eq!(tool_calls, 1);

    let spec = format_messages_with_options(
        &history,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );
    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["reasoning_content"], "think once");
    assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_merges_yielded_reasoning_with_suffix() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think \"},\"finish_reason\":null}]}\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"once\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut history = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            history.push(msg);
        }
    }

    let spec = format_messages_with_options(
        &history,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );
    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["reasoning_content"], "think once");
    assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_preserves_unyielded_reasoning() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"tool thought\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut thinking = String::new();
    let mut tool_calls = 0;
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                match content {
                    MessageContentBlock::Thinking(t) => thinking.push_str(&t.thinking),
                    MessageContentBlock::ToolRequest(_) => tool_calls += 1,
                    _ => {}
                }
            }
        }
    }

    assert_eq!(thinking, "tool thought");
    assert_eq!(tool_calls, 1);
    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_without_tool_call_index() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"id\":\"functions.get_weather:0\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\": \\\"Paris\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut tool_calls = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(request) = content {
                    let tool_call = request.tool_call.as_ref().expect("tool call should parse");
                    tool_calls.push((tool_call.name.to_string(), tool_call.arguments.clone()));
                }
            }
        }
    }

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].0, "get_weather");
    assert_eq!(tool_calls[0].1, Some(object!({"city": "Paris"})));
    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_dotted_name_passes_through() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"tc1\",\"type\":\"function\",\"function\":{\"name\":\"ext__db.query\",\"arguments\":\"{\\\"command\\\": \\\"ls\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut tool_calls = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(request) = content {
                    let tool_call = request.tool_call.as_ref().expect("tool call should parse");
                    tool_calls.push((tool_call.name.to_string(), tool_call.arguments.clone()));
                }
            }
        }
    }

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].0, "ext__db.query");
    assert_eq!(tool_calls[0].1, Some(object!({"command": "ls"})));
    Ok(())
}

#[tokio::test]
async fn test_streaming_tool_call_degenerate_name_passes_through() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"tc1\",\"type\":\"function\",\"function\":{\"name\":\"???\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut names = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(request) = content {
                    names.push(
                        request
                            .tool_call
                            .as_ref()
                            .expect("passes through")
                            .name
                            .to_string(),
                    );
                }
            }
        }
    }

    assert_eq!(names, vec!["???".to_string()]);
    Ok(())
}

#[tokio::test]
async fn test_streaming_multiple_tool_calls_without_tool_call_index() -> anyhow::Result<()> {
    let response_lines = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"id\":\"functions.get_weather:0\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\": \\\"Paris\\\"}\"}},{\"id\":\"functions.get_weather:1\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\": \\\"Tokyo\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut tool_calls = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(request) = content {
                    let tool_call = request.tool_call.as_ref().expect("tool call should parse");
                    tool_calls.push((tool_call.name.to_string(), tool_call.arguments.clone()));
                }
            }
        }
    }

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0].0, "get_weather");
    assert_eq!(tool_calls[0].1, Some(object!({"city": "Paris"})));
    assert_eq!(tool_calls[1].0, "get_weather");
    assert_eq!(tool_calls[1].1, Some(object!({"city": "Tokyo"})));
    Ok(())
}

// Streaming counterpart: both fields in one delta must parse and yield
// thinking content, not fail with "duplicate field `reasoning_content`".
#[tokio::test]
async fn test_streaming_chunk_with_both_reasoning_fields() -> anyhow::Result<()> {
    let response_lines = "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning\":\"thinking...\",\"reasoning_content\":\"thinking...\"},\"finish_reason\":null}]}\ndata: [DONE]";
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut saw_thinking = false;
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for c in &msg.content {
                if let MessageContentBlock::Thinking(t) = c {
                    assert_eq!(t.thinking, "thinking...");
                    saw_thinking = true;
                }
            }
        }
    }
    assert!(
        saw_thinking,
        "expected thinking content from merged reasoning fields"
    );
    Ok(())
}

#[test]
fn test_delta_tool_call_function_accepts_null_arguments() {
    let raw = r#"{"arguments":null}"#;
    let parsed: DeltaToolCallFunction =
        serde_json::from_str(raw).expect("null arguments must deserialize");
    assert_eq!(parsed.arguments, "");

    let raw = r#"{}"#;
    let parsed: DeltaToolCallFunction =
        serde_json::from_str(raw).expect("missing arguments must deserialize");
    assert_eq!(parsed.arguments, "");

    let raw = r#"{"arguments":"{\"k\":1}"}"#;
    let parsed: DeltaToolCallFunction = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.arguments, "{\"k\":1}");
}

