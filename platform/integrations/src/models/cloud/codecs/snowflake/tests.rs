use super::*;
use crate::conversation::message::{Message, MessageContentBlock};
use anyhow::Result;
use rmcp::model::{CallToolRequestParams, Tool};
use rmcp::object;
use serde_json::json;

#[test]
fn test_parse_text_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content_list": [{
            "type": "text",
            "text": "Hello! How can I assist you today?"
        }],
        "model": "claude-4-sonnet",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 12,
            "output_tokens": 15
        }
    });

    let message = response_to_message(&response)?;
    let usage = get_usage(&response)?;

    if let MessageContentBlock::Text(text) = &message.content[0] {
        assert_eq!(text.text, "Hello! How can I assist you today?");
    } else {
        panic!("Expected Text content");
    }

    assert_eq!(usage.input_tokens, Some(12));
    assert_eq!(usage.output_tokens, Some(15));
    assert_eq!(usage.total_tokens, Some(27)); // 12 + 15

    Ok(())
}

#[test]
fn test_parse_tool_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content_list": [{
            "type": "tool_use",
            "tool_use_id": "tool_1",
            "name": "calculator",
            "input": {"expression": "2 + 2"}
        }],
        "model": "claude-4-sonnet",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 15,
            "output_tokens": 20
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

    assert_eq!(usage.input_tokens, Some(15));
    assert_eq!(usage.output_tokens, Some(20));
    assert_eq!(usage.total_tokens, Some(35)); // 15 + 20

    Ok(())
}

#[test]
fn test_message_to_snowflake_spec() {
    let messages = vec![
        Message::user().with_text("Hello"),
        Message::assistant().with_text("Hi there"),
        Message::user().with_text("How are you?"),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 3);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(spec[0]["content"], "Hello");
    assert_eq!(spec[1]["role"], "assistant");
    assert_eq!(spec[1]["content"], "Hi there");
    assert_eq!(spec[2]["role"], "user");
    assert_eq!(spec[2]["content"], "How are you?");
}

#[test]
fn test_tools_to_snowflake_spec() {
    let tools = vec![
        Tool::new(
            "calculator",
            "Calculate mathematical expressions",
            object!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The mathematical expression to evaluate"
                    }
                }
            }),
        ),
        Tool::new(
            "weather",
            "Get weather information",
            object!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The location to get weather for"
                    }
                }
            }),
        ),
    ];

    let spec = format_tools(&tools);

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[0]["tool_spec"]["name"], "calculator");
    assert_eq!(
        spec[0]["tool_spec"]["description"],
        "Calculate mathematical expressions"
    );
    assert_eq!(spec[1]["tool_spec"]["name"], "weather");
    assert_eq!(
        spec[1]["tool_spec"]["description"],
        "Get weather information"
    );
}

#[test]
fn test_system_to_snowflake_spec() {
    let system = "You are a helpful assistant.";
    let spec = format_system(system);

    assert_eq!(spec["role"], "system");
    assert_eq!(spec["content"], system);
}

#[test]
fn test_parse_streaming_response() -> Result<()> {
    let sse_data = r#"data: {"id":"a9537c2c-2017-4906-9817-2456168d89fa","model":"claude-sonnet-4-20250514","choices":[{"delta":{"type":"text","content":"I","content_list":[{"type":"text","text":"I"}],"text":"I"}}],"usage":{}}

data: {"id":"a9537c2c-2017-4906-9817-2456168d89fa","model":"claude-sonnet-4-20250514","choices":[{"delta":{"type":"text","content":"'ll help you check Nvidia's current","content_list":[{"type":"text","text":"'ll help you check Nvidia's current"}],"text":"'ll help you check Nvidia's current"}}],"usage":{}}

data: {"id":"a9537c2c-2017-4906-9817-2456168d89fa","model":"claude-sonnet-4-20250514","choices":[{"delta":{"type":"tool_use","tool_use_id":"tooluse_FB_nOElDTAOKa-YnVWI5Uw","name":"get_stock_price","content_list":[{"tool_use_id":"tooluse_FB_nOElDTAOKa-YnVWI5Uw","name":"get_stock_price"}],"text":""}}],"usage":{}}

data: {"id":"a9537c2c-2017-4906-9817-2456168d89fa","model":"claude-sonnet-4-20250514","choices":[{"delta":{"type":"tool_use","input":"{\"symbol\":\"NVDA\"}","content_list":[{"input":"{\"symbol\":\"NVDA\"}"}],"text":""}}],"usage":{"prompt_tokens":397,"completion_tokens":65,"total_tokens":462}}
"#;

    let message = parse_streaming_response(sse_data)?;

    // Should have both text and tool request
    assert_eq!(message.content.len(), 2);

    if let MessageContentBlock::Text(text) = &message.content[0] {
        assert!(text.text.contains("I'll help you check Nvidia's current"));
    } else {
        panic!("Expected Text content first");
    }

    if let MessageContentBlock::ToolRequest(tool_request) = &message.content[1] {
        let tool_call = tool_request.tool_call.as_ref().unwrap();
        assert_eq!(tool_call.name, "get_stock_price");
        assert_eq!(tool_call.arguments, Some(object!({"symbol": "NVDA"})));
        assert_eq!(tool_request.id, "tooluse_FB_nOElDTAOKa-YnVWI5Uw");
    } else {
        panic!("Expected ToolRequest content second");
    }

    Ok(())
}

#[test]
fn test_create_request_format() -> Result<()> {
    use crate::conversation::message::Message;
    use crate::model::ModelConfig;

    let model_config = ModelConfig::new("claude-4-sonnet").with_canonical_limits("snowflake");

    let system = "You are a helpful assistant that can use tools to get information.";
    let messages = vec![Message::user().with_text("What is the stock price of Nvidia?")];

    let tools = vec![Tool::new(
        "get_stock_price",
        "Get stock price information",
        object!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "The symbol for the stock ticker, e.g. Snowflake = SNOW"
                }
            },
            "required": ["symbol"]
        }),
    )];

    let request = create_request(&model_config, system, &messages, &tools)?;

    // Check basic structure
    assert_eq!(request["model"], "claude-4-sonnet");

    let messages_array = request["messages"].as_array().unwrap();
    assert_eq!(messages_array.len(), 2); // system + user message

    // First message should be system with simple content
    assert_eq!(messages_array[0]["role"], "system");
    assert_eq!(
        messages_array[0]["content"],
        "You are a helpful assistant that can use tools to get information."
    );

    // Second message should be user with simple content
    assert_eq!(messages_array[1]["role"], "user");
    assert_eq!(
        messages_array[1]["content"],
        "What is the stock price of Nvidia?"
    );

    // Tools should have tool_spec wrapper
    let tools_array = request["tools"].as_array().unwrap();
    assert_eq!(tools_array[0]["tool_spec"]["name"], "get_stock_price");

    Ok(())
}

#[test]
fn test_parse_mixed_text_and_tool_response() -> Result<()> {
    let response = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": "I'll help you with that calculation.",
        "content_list": [
            {
                "type": "text",
                "text": "I'll help you with that calculation."
            },
            {
                "type": "tool_use",
                "tool_use_id": "tool_1",
                "name": "calculator",
                "input": {"expression": "2 + 2"}
            }
        ],
        "model": "claude-4-sonnet",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 15
        }
    });

    let message = response_to_message(&response)?;

    // Should have both text and tool request content
    assert_eq!(message.content.len(), 2);

    if let MessageContentBlock::Text(text) = &message.content[0] {
        assert_eq!(text.text, "I'll help you with that calculation.");
    } else {
        panic!("Expected Text content first");
    }

    if let MessageContentBlock::ToolRequest(tool_request) = &message.content[1] {
        let tool_call = tool_request.tool_call.as_ref().unwrap();
        assert_eq!(tool_call.name, "calculator");
        assert_eq!(tool_request.id, "tool_1");
    } else {
        panic!("Expected ToolRequest content second");
    }

    Ok(())
}

#[test]
fn test_empty_tools_array() {
    let tools: Vec<Tool> = vec![];
    let spec = format_tools(&tools);
    assert_eq!(spec.len(), 0);
}

#[test]
fn test_create_request_excludes_tools_for_description() -> Result<()> {
    use crate::conversation::message::Message;
    use crate::model::ModelConfig;

    let model_config = ModelConfig::new("claude-4-sonnet").with_canonical_limits("snowflake");
    let system = "Reply with only a description in four words or less";
    let messages = vec![Message::user().with_text("Test message")];
    let tools = vec![Tool::new(
        "test_tool",
        "Test tool",
        object!({"type": "object", "properties": {}}),
    )];

    let request = create_request(&model_config, system, &messages, &tools)?;

    // Should not include tools for description requests
    assert!(request.get("tools").is_none());

    Ok(())
}

#[test]
fn test_message_formatting_skips_tool_requests() {
    // Create a conversation with text, tool requests, and tool responses
    let tool_call =
        CallToolRequestParams::new("calculator").with_arguments(object!({"expression": "2 + 2"}));

    let messages = vec![
        Message::user().with_text("Calculate 2 + 2"),
        Message::assistant()
            .with_text("I'll help you calculate that.")
            .with_tool_request("tool_1", Ok(tool_call)),
        Message::user().with_text("Thanks!"),
    ];

    let spec = format_messages(&messages);

    // Should only have 3 messages - the tool request should be skipped
    assert_eq!(spec.len(), 3);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(spec[0]["content"], "Calculate 2 + 2");
    assert_eq!(spec[1]["role"], "assistant");
    assert_eq!(spec[1]["content"], "I'll help you calculate that.");
    assert_eq!(spec[2]["role"], "user");
    assert_eq!(spec[2]["content"], "Thanks!");

    // Verify no tool request content is in the message history
    for message in &spec {
        let content = message["content"].as_str().unwrap();
        assert!(!content.contains("Using tool:"));
        assert!(!content.contains("calculator"));
    }
}
