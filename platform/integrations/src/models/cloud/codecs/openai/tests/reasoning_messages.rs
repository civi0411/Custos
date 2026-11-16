use super::super::*;
use super::helpers::*;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::images::ImageFormat;
use rmcp::model::CallToolRequestParams;
use rmcp::object;
use serde_json::json;

#[test]
fn test_response_to_message_with_reasoning_content() -> anyhow::Result<()> {
    // Test capturing reasoning_content from DeepSeek reasoning models
    let response = json!({
        "choices": [{
            "role": "assistant",
            "message": {
                "reasoning_content": "Let me think about this step by step...",
                "content": "The answer is 9.11 is greater than 9.8"
            }
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 25,
            "total_tokens": 35
        }
    });

    let message = response_to_message(&response)?;
    assert_eq!(message.content.len(), 2);

    // First should be thinking content (reasoning is mapped to thinking)
    if let MessageContentBlock::Thinking(thinking) = &message.content[0] {
        assert_eq!(thinking.thinking, "Let me think about this step by step...");
    } else {
        panic!("Expected Thinking content, got {:?}", message.content[0]);
    }

    // Second should be text content
    if let MessageContentBlock::Text(text) = &message.content[1] {
        assert_eq!(text.text, "The answer is 9.11 is greater than 9.8");
    } else {
        panic!("Expected Text content");
    }

    Ok(())
}

#[test]
fn test_response_to_message_extracts_inline_think_blocks() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "role": "assistant",
            "message": {
                "content": "<think>internal reasoning</think>Visible answer"
            }
        }]
    });

    let message = response_to_message(&response)?;
    assert_eq!(message.content.len(), 2);

    if let MessageContentBlock::Thinking(thinking) = &message.content[0] {
        assert_eq!(thinking.thinking, "internal reasoning");
    } else {
        panic!("Expected Thinking content, got {:?}", message.content[0]);
    }

    if let MessageContentBlock::Text(text) = &message.content[1] {
        assert_eq!(text.text, "Visible answer");
    } else {
        panic!("Expected Text content");
    }

    Ok(())
}

#[test]
fn test_response_to_message_prefers_structured_reasoning_over_inline_think(
) -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "role": "assistant",
            "message": {
                "reasoning_content": "structured reasoning",
                "content": "<think>inline reasoning</think>Visible answer"
            }
        }]
    });

    let message = response_to_message(&response)?;
    assert_eq!(message.content.len(), 2);

    if let MessageContentBlock::Thinking(thinking) = &message.content[0] {
        assert_eq!(thinking.thinking, "structured reasoning");
    } else {
        panic!("Expected Thinking content");
    }

    if let MessageContentBlock::Text(text) = &message.content[1] {
        assert_eq!(text.text, "Visible answer");
    } else {
        panic!("Expected Text content");
    }

    Ok(())
}

#[test]
fn test_format_messages_with_reasoning_content() -> anyhow::Result<()> {
    // Test that reasoning_content is properly included in formatted messages
    let mut message = Message::assistant()
        .with_content(MessageContentBlock::thinking(
            "Thinking through the problem...",
            "",
        ))
        .with_text("The result is 42");

    // Add a tool call to test that reasoning_content works with tool calls
    message = message.with_tool_request(
        "tool1",
        Ok(rmcp::model::CallToolRequestParams::new("test_tool")
            .with_arguments(rmcp::object!({"param": "value"}))),
    );

    let spec = format_messages_with_options(
        &[message],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");

    // Should have reasoning_content field
    assert!(spec[0].get("reasoning_content").is_some());
    assert_eq!(
        spec[0]["reasoning_content"],
        "Thinking through the problem..."
    );

    // Should have content
    assert_eq!(spec[0]["content"], "The result is 42");

    // Should have tool_calls
    assert!(spec[0]["tool_calls"].is_array());
    assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

    Ok(())
}

#[test]
fn test_format_messages_preserves_reasoning_content_for_legacy_compat() -> anyhow::Result<()> {
    let message = Message::assistant()
        .with_content(MessageContentBlock::thinking(
            "Thinking through the problem...",
            "",
        ))
        .with_text("The result is 42");

    let spec = format_messages(&[message], &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["content"], "The result is 42");
    assert_eq!(
        spec[0]["reasoning_content"],
        "Thinking through the problem..."
    );

    Ok(())
}

#[test]
fn test_format_messages_with_options_can_omit_reasoning_content() -> anyhow::Result<()> {
    let message = Message::assistant()
        .with_content(MessageContentBlock::thinking(
            "Thinking through the problem...",
            "",
        ))
        .with_text("The result is 42");

    let spec = format_messages_with_options(
        &[message],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: false,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["content"], "The result is 42");
    assert!(spec[0].get("reasoning_content").is_none());

    Ok(())
}

#[test]
fn test_create_request_preserves_reasoning_content_for_legacy_compat() -> anyhow::Result<()> {
    let model_config = test_model_config("deepseek-reasoner").with_max_tokens(Some(1024));
    let message = Message::assistant()
        .with_content(MessageContentBlock::thinking("preserve this", ""))
        .with_tool_request(
            "tool1",
            Ok(rmcp::model::CallToolRequestParams::new("test_tool")
                .with_arguments(rmcp::object!({}))),
        );

    let request = create_request(
        &model_config,
        "system",
        &[message],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    assert_eq!(request["messages"][1]["reasoning_content"], "preserve this");

    Ok(())
}

#[test]
fn test_format_messages_carries_thinking_only_chunks_to_tool_call() -> anyhow::Result<()> {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("think ", "")),
        Message::assistant().with_content(MessageContentBlock::thinking("once", "")),
        Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool")
                .with_arguments(object!({"param": "value"}))),
        ),
    ];

    let spec = format_messages_with_options(
        &messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["reasoning_content"], "think once");
    assert_eq!(spec[0]["content"], json!(null));
    assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

    Ok(())
}

#[test]
fn test_format_messages_does_not_duplicate_pending_thinking() -> anyhow::Result<()> {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("think once", "")),
        Message::assistant()
            .with_content(MessageContentBlock::thinking("think once", ""))
            .with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("test_tool")
                    .with_arguments(object!({"param": "value"}))),
            ),
    ];

    let spec = format_messages_with_options(
        &messages,
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

#[test]
fn test_format_messages_merges_pending_thinking_with_tool_call_suffix() -> anyhow::Result<()> {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("think ", "")),
        Message::assistant()
            .with_content(MessageContentBlock::thinking("once", ""))
            .with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("test_tool")
                    .with_arguments(object!({"param": "value"}))),
            ),
    ];

    let spec = format_messages_with_options(
        &messages,
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

#[test]
fn test_format_messages_does_not_carry_thinking_across_user_message() -> anyhow::Result<()> {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("stale", "")),
        Message::user().with_text("new turn"),
        Message::assistant()
            .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool"))),
    ];

    let spec = format_messages_with_options(
        &messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(spec[1]["role"], "assistant");
    assert!(spec[1].get("reasoning_content").is_none());

    Ok(())
}

#[test]
fn test_format_messages_carries_reasoning_through_text_only_chunks() -> anyhow::Result<()> {
    // Scenario B from the streaming bug: thinking arrives first, then multiple
    // text-only assistant messages, then a tool call with thinking re-attached
    // by agent.rs (via the earlier-chunk lookback).
    // Text-only messages set tool_call_turn_reasoning="" (line 453 else-branch),
    // but the TC's own Thinking content must repopulate it.
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("reason", "")),
        Message::assistant().with_text("partial answer"),
        Message::assistant().with_text("more text"),
        // agent.rs attaches the earlier thinking to the TC message
        Message::assistant()
            .with_content(MessageContentBlock::thinking("reason", ""))
            .with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("test_tool").with_arguments(object!({}))),
            ),
    ];

    let spec = format_messages_with_options(
        &messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    let tool_call_msgs: Vec<_> = spec
        .iter()
        .filter(|m| {
            m.get("tool_calls")
                .and_then(|tc| tc.as_array())
                .is_some_and(|a| !a.is_empty())
        })
        .collect();

    assert_eq!(tool_call_msgs.len(), 1);
    assert_eq!(
        tool_call_msgs[0]["reasoning_content"], "reason",
        "reasoning_content must survive text-only chunks between thinking and tool call"
    );

    Ok(())
}

#[test]
fn test_format_messages_carries_reasoning_to_all_split_tool_calls() -> anyhow::Result<()> {
    // Simulates DeepSeek/Kimi streaming: a thinking-only chunk arrives first,
    // then the agent splits two tool calls into separate messages, each with
    // the same reasoning attached (as agent.rs does via response_thinking).
    // The formatter must keep reasoning_content on both so that
    // merge_split_tool_call_messages can reunite them into one assistant message.
    let tool_result1 = Message::user().with_tool_response(
        "tool1",
        Ok(rmcp::model::CallToolResult::success(vec![
            rmcp::model::ContentBlock::text("result1"),
        ])),
    );
    let messages = vec![
        // Standalone thinking message (created by agent.rs alongside request_msgs)
        Message::assistant().with_content(MessageContentBlock::thinking("reasoning", "")),
        // Each request_msg has thinking explicitly attached (agent.rs behaviour)
        Message::assistant()
            .with_content(MessageContentBlock::thinking("reasoning", ""))
            .with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("tool_a").with_arguments(object!({}))),
            ),
        tool_result1,
        Message::assistant()
            .with_content(MessageContentBlock::thinking("reasoning", ""))
            .with_tool_request(
                "tool2",
                Ok(CallToolRequestParams::new("tool_b").with_arguments(object!({}))),
            ),
    ];

    let spec = format_messages_with_options(
        &messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    // After merge: one assistant message with both tool calls
    let assistant_msgs: Vec<_> = spec
        .iter()
        .filter(|m| m.get("role") == Some(&json!("assistant")))
        .collect();
    assert_eq!(assistant_msgs.len(), 1);
    assert_eq!(assistant_msgs[0]["reasoning_content"], "reasoning");
    let tool_calls = assistant_msgs[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 2);

    Ok(())
}

#[test]
fn test_sequential_tool_calls_not_merged() -> anyhow::Result<()> {
    // Verifies that two tool calls from *different* turns are never merged,
    // even when the second call carries no fresh reasoning (the previous
    // turn's reasoning must not leak into it).
    let tool_result1 = Message::user().with_tool_response(
        "tool1",
        Ok(rmcp::model::CallToolResult::success(vec![
            rmcp::model::ContentBlock::text("result1"),
        ])),
    );
    let messages = vec![
        // Turn 1: thinking then tool call
        Message::assistant().with_content(MessageContentBlock::thinking("turn1_reasoning", "")),
        Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams::new("tool_a").with_arguments(object!({}))),
        ),
        tool_result1,
        // Turn 2: new tool call, no fresh thinking
        Message::assistant().with_tool_request(
            "tool2",
            Ok(CallToolRequestParams::new("tool_b").with_arguments(object!({}))),
        ),
    ];

    let spec = format_messages_with_options(
        &messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    );

    let assistant_msgs: Vec<_> = spec
        .iter()
        .filter(|m| m.get("role") == Some(&json!("assistant")))
        .collect();

    // Must remain two separate assistant messages — not merged across turns.
    assert_eq!(
        assistant_msgs.len(),
        2,
        "sequential tool calls must not be merged"
    );

    // Turn 1 carries reasoning; turn 2 must not inherit it.
    assert_eq!(assistant_msgs[0]["reasoning_content"], "turn1_reasoning");
    assert!(
        assistant_msgs[1].get("reasoning_content").is_none()
            || assistant_msgs[1]["reasoning_content"].is_null(),
        "turn 2 must not inherit stale reasoning from turn 1"
    );

    // The tool result must appear between the two assistant messages.
    let tool_idx = spec
        .iter()
        .position(|m| m.get("role") == Some(&json!("tool")))
        .expect("tool result must be present");
    let asst1_idx = spec
        .iter()
        .position(|m| m.get("role") == Some(&json!("assistant")))
        .unwrap();
    let asst2_idx = spec
        .iter()
        .rposition(|m| m.get("role") == Some(&json!("assistant")))
        .unwrap();
    assert!(
        asst1_idx < tool_idx && tool_idx < asst2_idx,
        "tool result must sit between the two assistant messages"
    );

    Ok(())
}

