use super::super::*;
use crate::conversation::message::MessageContentBlock;
use serde_json::json;

#[test]
fn test_merge_split_tool_calls_with_reasoning() {
    let mut messages = vec![
        json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
        json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
    ];
    merge_split_tool_call_messages(&mut messages);

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(messages[1]["role"], "tool");
    assert_eq!(messages[2]["role"], "tool");
}

#[test]
fn test_no_merge_without_reasoning() {
    let mut messages = vec![
        json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}]}),
        json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
        json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}]}),
        json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
    ];
    merge_split_tool_call_messages(&mut messages);

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 1);
    assert_eq!(messages[2]["tool_calls"].as_array().unwrap().len(), 1);
}

#[test]
fn test_merge_split_tool_calls_with_image_gap() {
    let mut messages = vec![
        json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "screenshot", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc1", "content": "This tool result included an image that is uploaded in the next message."}),
        json!({"role": "user", "content": [{"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}]}),
        json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "click", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc2", "content": "clicked"}),
    ];
    merge_split_tool_call_messages(&mut messages);

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(messages[0]["role"], "assistant");
    assert_eq!(messages[1]["role"], "tool");
    assert_eq!(messages[1]["tool_call_id"], "tc1");
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["tool_call_id"], "tc2");
}

#[test]
fn test_merge_does_not_skip_real_user_message() {
    let mut messages = vec![
        json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
        json!({"role": "user", "content": "what happened?"}),
        json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
        json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
    ];
    merge_split_tool_call_messages(&mut messages);

    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 1);
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "what happened?");
    assert_eq!(messages[3]["tool_calls"].as_array().unwrap().len(), 1);
}

// vLLM serving gpt-oss emits both `reasoning` and `reasoning_content`
// in the same payload; the non-streaming path handles it fine today.
#[test]
fn test_response_to_message_with_both_reasoning_fields() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "answer",
                "reasoning": "thinking...",
                "reasoning_content": "thinking..."
            }
        }]
    });

    let message = response_to_message(&response)?;
    assert_eq!(message.content.len(), 2);
    if let MessageContentBlock::Thinking(t) = &message.content[0] {
        assert_eq!(t.thinking, "thinking...");
    } else {
        panic!("Expected Thinking content, got {:?}", message.content[0]);
    }
    Ok(())
}

// ---- metadata-only SSE frames (gateway trace/guardrail objects) -----------------------
//
// Some OpenAI-compatible gateways interleave objects that have no `choices` key at all
// with the real chunks. A Portkey gateway sends a `hook_results` guardrail trace as the
// FIRST frame whenever strict-openai-compliance is off. Failing on such a frame killed
// the whole turn.

