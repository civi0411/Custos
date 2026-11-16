use crate::conversation::message::{Message, MessageContentBlock};
use rmcp::model::{CallToolRequestParams, ErrorCode, ErrorData, Role};
use rmcp::object;
use serde_json::Value;

#[test]
fn test_message_serialization() {
    let message = Message::assistant()
        .with_text("Hello, I'll help you with that.")
        .with_tool_request(
            "tool123",
            Ok(CallToolRequestParams::new("test_tool")
                .with_arguments(object!({"param": "value"}))),
        );

    let json_str = serde_json::to_string_pretty(&message).unwrap();

    let value: Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(value["role"], "assistant");
    assert!(value["created"].is_i64());
    assert!(value["content"].is_array());

    let content = &value["content"];

    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "Hello, I'll help you with that.");

    assert_eq!(content[1]["type"], "toolRequest");
    assert_eq!(content[1]["id"], "tool123");

    assert_eq!(content[1]["toolCall"]["status"], "success");
    assert_eq!(content[1]["toolCall"]["value"]["name"], "test_tool");
    assert_eq!(
        content[1]["toolCall"]["value"]["arguments"]["param"],
        "value"
    );
}

#[test]
fn test_error_serialization() {
    let message = Message::assistant().with_tool_request(
        "tool123",
        Err(ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: std::borrow::Cow::from("Something went wrong".to_string()),
            data: None,
        }),
    );

    let json_str = serde_json::to_string_pretty(&message).unwrap();

    let value: Value = serde_json::from_str(&json_str).unwrap();

    let tool_call = &value["content"][0]["toolCall"];
    assert_eq!(tool_call["status"], "error");
    assert_eq!(tool_call["error"], "-32603: Something went wrong");
}

#[test]
fn test_deserialization() {
    let json_str = r#"{
        "role": "assistant",
        "created": 1740171566,
        "content": [
            {
                "type": "text",
                "text": "I'll help you with that."
            },
            {
                "type": "toolRequest",
                "id": "tool123",
                "toolCall": {
                    "status": "success",
                    "value": {
                        "name": "test_tool",
                        "arguments": {"param": "value"}
                    }
                }
            }
        ],
        "metadata": { "agentVisible": true, "userVisible": true }
    }"#;

    let message: Message = serde_json::from_str(json_str).unwrap();

    assert_eq!(message.role, Role::Assistant);
    assert_eq!(message.created, 1740171566);
    assert_eq!(message.content.len(), 2);

    if let MessageContentBlock::Text(text) = &message.content[0] {
        assert_eq!(text.text, "I'll help you with that.");
    } else {
        panic!("Expected Text content");
    }

    if let MessageContentBlock::ToolRequest(req) = &message.content[1] {
        assert_eq!(req.id, "tool123");
        if let Ok(tool_call) = &req.tool_call {
            assert_eq!(tool_call.name, "test_tool");
            assert_eq!(tool_call.arguments, Some(object!({"param": "value"})))
        } else {
            panic!("Expected successful tool call");
        }
    } else {
        panic!("Expected ToolRequest content");
    }
}

#[test]
fn test_deserialization_migrates_reasoning_to_thinking() {
    let json = serde_json::json!({
        "role": "assistant",
        "created": 1740171566,
        "content": [
            { "type": "reasoning", "text": "step by step" },
            { "type": "text", "text": "final answer" }
        ],
        "metadata": { "agentVisible": true, "userVisible": true }
    });

    let message: Message = serde_json::from_value(json).unwrap();
    assert_eq!(message.content.len(), 2);

    let MessageContentBlock::Thinking(thinking) = &message.content[0] else {
        panic!("Expected Thinking content");
    };
    assert_eq!(thinking.thinking, "step by step");
    assert!(thinking.signature.is_empty());
}

#[test]
fn test_deserialization_drops_invalid_reasoning_blocks() {
    let json = serde_json::json!({
        "role": "assistant",
        "created": 1740171566,
        "content": [
            { "type": "reasoning" },
            { "type": "reasoning", "text": 42 },
            { "type": "text", "text": "still here" }
        ],
        "metadata": { "agentVisible": true, "userVisible": true }
    });

    let message: Message = serde_json::from_value(json).unwrap();
    assert_eq!(message.content.len(), 1);

    let MessageContentBlock::Text(text) = &message.content[0] else {
        panic!("Expected Text content");
    };
    assert_eq!(text.text, "still here");
}

#[test]
fn test_message_deserialization_sanitizes_text_content() {
    let malicious_text = "Hello\u{E0041}\u{E0042}\u{E0043}world";
    let malicious_json = format!(
        r#"{{
        "id": "test-id",
        "role": "user",
        "created": 1640995200,
        "content": [
            {{
                "type": "text",
                "text": "{}"
            }},
            {{
                "type": "image",
                "data": "base64data",
                "mimeType": "image/png"
            }}
        ],
        "metadata": {{ "agentVisible": true, "userVisible": true }}
    }}"#,
        malicious_text
    );

    let message: Message = serde_json::from_str(&malicious_json).unwrap();

    assert_eq!(message.as_concat_text(), "Helloworld");

    if let MessageContentBlock::Image(img) = &message.content[1] {
        assert_eq!(img.data, "base64data");
        assert_eq!(img.mime_type, "image/png");
    } else {
        panic!("Expected ImageContent");
    }
}

#[test]
fn test_legitimate_unicode_preserved_during_message_deserialization() {
    let clean_json = r#"{
        "id": "test-id",
        "role": "user",
        "created": 1640995200,
        "content": [{
            "type": "text",
            "text": "Hello world 世界 🌍"
        }],
        "metadata": { "agentVisible": true, "userVisible": true }
    }"#;

    let message: Message = serde_json::from_str(clean_json).unwrap();

    assert_eq!(message.as_concat_text(), "Hello world 世界 🌍");
}

#[test]
fn test_legacy_tool_response_deserialization() {
    let legacy_json = r#"{
        "role": "user",
        "created": 1640995200,
        "content": [{
            "type": "toolResponse",
            "id": "tool123",
            "toolResult": {
                "status": "success",
                "value": [
                    {
                        "type": "text",
                        "text": "Tool output text"
                    }
                ]
            }
        }],
        "metadata": { "agentVisible": true, "userVisible": true }
    }"#;

    let message: Message = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(message.content.len(), 1);

    if let MessageContentBlock::ToolResponse(response) = &message.content[0] {
        assert_eq!(response.id, "tool123");
        if let Ok(result) = &response.tool_result {
            assert_eq!(result.content.len(), 1);
            assert_eq!(
                result.content[0].as_text().unwrap().text,
                "Tool output text"
            );
        } else {
            panic!("Expected successful tool result");
        }
    } else {
        panic!("Expected ToolResponse content");
    }
}

#[test]
fn test_new_tool_response_deserialization() {
    let new_json = r#"{
        "role": "user",
        "created": 1640995200,
        "content": [{
            "type": "toolResponse",
            "id": "tool456",
            "toolResult": {
                "status": "success",
                "value": {
                    "content": [
                        {
                            "type": "text",
                            "text": "New format output"
                        }
                    ],
                    "isError": false
                }
            }
        }],
        "metadata": { "agentVisible": true, "userVisible": true }
    }"#;

    let message: Message = serde_json::from_str(new_json).unwrap();
    assert_eq!(message.content.len(), 1);

    if let MessageContentBlock::ToolResponse(response) = &message.content[0] {
        assert_eq!(response.id, "tool456");
        if let Ok(result) = &response.tool_result {
            assert_eq!(result.content.len(), 1);
            assert_eq!(
                result.content[0].as_text().unwrap().text,
                "New format output"
            );
        } else {
            panic!("Expected successful tool result");
        }
    } else {
        panic!("Expected ToolResponse content");
    }
}

#[test]
fn test_tool_request_with_value_arguments_backward_compatibility() {
    struct TestCase {
        name: &'static str,
        arguments_json: &'static str,
        expected: Option<Value>,
    }

    let test_cases = [
        TestCase {
            name: "string",
            arguments_json: r#""string_argument""#,
            expected: Some(serde_json::json!({"value": "string_argument"})),
        },
        TestCase {
            name: "array",
            arguments_json: r#"["a", "b", "c"]"#,
            expected: Some(serde_json::json!({"value": ["a", "b", "c"]})),
        },
        TestCase {
            name: "number",
            arguments_json: "42",
            expected: Some(serde_json::json!({"value": 42})),
        },
        TestCase {
            name: "null",
            arguments_json: "null",
            expected: None,
        },
        TestCase {
            name: "object",
            arguments_json: r#"{"key": "value", "number": 123}"#,
            expected: Some(serde_json::json!({"key": "value", "number": 123})),
        },
    ];

    for tc in test_cases {
        let json = format!(
            r#"{{
                "role": "assistant",
                "created": 1640995200,
                "content": [{{
                    "type": "toolRequest",
                    "id": "tool123",
                    "toolCall": {{
                        "status": "success",
                        "value": {{
                            "name": "test_tool",
                            "arguments": {}
                        }}
                    }}
                }}],
                "metadata": {{ "agentVisible": true, "userVisible": true }}
            }}"#,
            tc.arguments_json
        );

        let message: Message = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("{}: parse failed: {}", tc.name, e));

        let MessageContentBlock::ToolRequest(request) = &message.content[0] else {
            panic!("{}: expected ToolRequest content", tc.name);
        };

        let Ok(tool_call) = &request.tool_call else {
            panic!("{}: expected successful tool call", tc.name);
        };

        assert_eq!(tool_call.name, "test_tool", "{}: wrong tool name", tc.name);

        match (&tool_call.arguments, &tc.expected) {
            (None, None) => {}
            (Some(args), Some(expected)) => {
                let args_value = serde_json::to_value(args).unwrap();
                assert_eq!(&args_value, expected, "{}: arguments mismatch", tc.name);
            }
            (actual, expected) => {
                panic!("{}: expected {:?}, got {:?}", tc.name, expected, actual);
            }
        }
    }
}
