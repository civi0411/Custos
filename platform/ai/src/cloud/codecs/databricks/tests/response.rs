use super::super::*;
use super::*;
use crate::conversation::message::MessageContentBlock;
use rmcp::model::{ErrorCode, ErrorData, Role};
use rmcp::object;
use serde_json::{json, Value};

    #[test]
    fn test_response_to_message_text() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "Hello from John Cena!"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 1);
        if let MessageContentBlock::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello from John Cena!");
        } else {
            panic!("Expected Text content");
        }
        assert!(matches!(message.role, Role::Assistant));

        Ok(())
    }

    #[test]
    fn test_response_to_message_valid_toolrequest() -> anyhow::Result<()> {
        let response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 1);
        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({"param": "value"})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_invalid_func_name() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
            json!("invalid fn");

        let message = response_to_message(&response)?;

        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.starts_with("The provided function name"));
                }
                _ => panic!("Expected ToolNotFound error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_json_decode_error() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("invalid json {");

        let message = response_to_message(&response)?;

        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.contains("tool arguments") || msg.contains("truncated"));
                }
                _ => panic!("Expected InvalidParameters error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_non_object_arguments() -> anyhow::Result<()> {
        // Weaker models sometimes emit tool arguments that are valid JSON but
        // not an object (here, a bare array). This must surface as a tool error,
        // NOT panic via rmcp's `object()` debug-assert.
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("[1, 2, 3]");

        let message = response_to_message(&response)?;

        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.contains("must be a JSON object"));
                    assert!(
                        msg.contains("example_fn"),
                        "error must name the original tool so the model can retry it: {msg}"
                    );
                }
                _ => panic!("Expected InvalidParameters error for non-object args"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_empty_argument() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            serde_json::Value::String("".to_string());

        let message = response_to_message(&response)?;

        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]

    fn test_response_to_message_claude_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_text",
                                    "text": "Test thinking content",
                                    "signature": "test-signature"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContentBlock::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "Test thinking content");
            assert_eq!(thinking.signature, "test-signature");
        } else {
            panic!("Expected Thinking content");
        }

        if let MessageContentBlock::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_claude_encrypted_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "claude-sonnet-4-5-20250929",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_encrypted_text",
                                    "data": "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContentBlock::RedactedThinking(redacted) = &message.content[0] {
            assert_eq!(redacted.data, "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2");
        } else {
            panic!("Expected RedactedThinking content");
        }

        if let MessageContentBlock::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }
