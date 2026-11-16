use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::token_usage::Usage;
use anyhow::Result;
use rmcp::object;
use serde_json::json;
#[test]
fn test_parse_text_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello! How can I assist you today?"
        }],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 12,
            "output_tokens": 15,
            "cache_creation_input_tokens": 12,
            "cache_read_input_tokens": 0
        }
    });

    let message = response_to_message(&response)?;
    let usage = get_usage(&response)?;

    if let MessageContentBlock::Text(text) = &message.content[0] {
        assert_eq!(text.text, "Hello! How can I assist you today?");
    } else {
        panic!("Expected Text content");
    }

    assert_eq!(usage.input_tokens, Some(24)); // 12 + 12 = 24 actual tokens
    assert_eq!(usage.output_tokens, Some(15));
    assert_eq!(usage.total_tokens, Some(39)); // 24 + 15
    assert_eq!(usage.cache_read_input_tokens, Some(0));
    assert_eq!(usage.cache_write_input_tokens, Some(12));

    Ok(())
}

#[test]
fn test_parse_tool_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "tool_use",
            "id": "tool_1",
            "name": "calculator",
            "input": {
                "expression": "2 + 2"
            }
        }],
        "model": "claude-3-sonnet-20240229",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 15,
            "output_tokens": 20,
            "cache_creation_input_tokens": 15,
            "cache_read_input_tokens": 0,
        }
    });

    let message = response_to_message(&response)?;
    let usage = get_usage(&response)?;

    if let MessageContentBlock::ToolRequest(tool_request) = &message.content[0] {
        let tool_call = tool_request.tool_call.as_ref().unwrap();
        assert_eq!(tool_call.name, "calculator");
        assert_eq!(tool_call.arguments, Some(object!({"expression": "2 + 2"})));
    } else {
        panic!("Expected ToolRequest content");
    }

    assert_eq!(usage.input_tokens, Some(30)); // 15 + 15 = 30 actual tokens
    assert_eq!(usage.output_tokens, Some(20));
    assert_eq!(usage.total_tokens, Some(50)); // 30 + 20
    assert_eq!(usage.cache_read_input_tokens, Some(0));
    assert_eq!(usage.cache_write_input_tokens, Some(15));

    Ok(())
}

#[test]
fn test_parse_unsigned_thinking_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "thinking",
            "thinking": "internal reasoning"
        }],
        "model": "glm-4.7",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 12,
            "output_tokens": 15
        }
    });

    let message = response_to_message(&response)?;

    if let MessageContentBlock::Thinking(thinking) = &message.content[0] {
        assert_eq!(thinking.thinking, "internal reasoning");
        assert_eq!(thinking.signature, "");
    } else {
        panic!("Expected Thinking content");
    }

    Ok(())
}

#[test]
fn test_cache_pricing_calculation() -> Result<()> {
    // Test realistic cache scenario: small fresh input, large cached content
    let response = json!({
        "id": "msg_cache_test",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Based on the cached context, here's my response."
        }],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 7,        // Small fresh input
            "output_tokens": 50,      // Output tokens
            "cache_creation_input_tokens": 10000, // Large cache creation
            "cache_read_input_tokens": 5000       // Large cache read
        }
    });

    let usage = get_usage(&response)?;

    // ACTUAL input tokens should be:
    // 7 + 10000 + 5000 = 15007 total actual tokens
    assert_eq!(usage.input_tokens, Some(15007));
    assert_eq!(usage.output_tokens, Some(50));
    assert_eq!(usage.total_tokens, Some(15057)); // 15007 + 50
    assert_eq!(usage.cache_read_input_tokens, Some(5000));
    assert_eq!(usage.cache_write_input_tokens, Some(10000));

    Ok(())
}

#[test]
fn test_merge_delta_usage_raw_input_inherits_start_cache() {
    let start =
        Usage::new(Some(15007), Some(3), None).with_cache_tokens(Some(5000), Some(10000));
    let delta_data = json!({"input_tokens": 8, "output_tokens": 510});
    let delta = get_usage(&delta_data).unwrap();

    let merged = merge_delta_usage(&start, &delta, &delta_data);
    assert_eq!(merged.input_tokens, Some(15008)); // 8 + 5000 + 10000
    assert_eq!(merged.output_tokens, Some(510));
    assert_eq!(merged.cache_read_input_tokens, Some(5000));
    assert_eq!(merged.cache_write_input_tokens, Some(10000));
}

