use super::super::*;
use super::helpers::*;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::token_usage::{ProviderUsage, Usage};
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
fn test_record_response_metadata() {
    let response = json!({
        "id": "chatcmpl-123",
        "choices": [
            {"finish_reason": "stop"},
            {"finish_reason": "tool_calls"}
        ]
    });
    let mut usage = ProviderUsage::new("test-model".to_string(), Usage::default());

    record_response_metadata(&mut usage, &response);

    assert_eq!(usage.response_id.as_deref(), Some("chatcmpl-123"));
    assert_eq!(
        usage.finish_reasons,
        Some(vec!["stop".to_string(), "tool_calls".to_string()])
    );
}

#[test]
fn test_response_to_message_marks_length_finish_reason() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Partial answer"
            },
            "finish_reason": "length"
        }]
    });

    let message = response_to_message(&response)?;

    assert_eq!(message.as_concat_text(), "Partial answer");
    assert!(message.metadata.output_token_limit_reached);

    Ok(())
}

#[test]
fn test_response_to_message_rejects_length_terminated_tool_calls() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_empty",
                        "type": "function",
                        "function": {
                            "name": "empty_tool",
                            "arguments": ""
                        }
                    },
                    {
                        "id": "call_valid",
                        "type": "function",
                        "function": {
                            "name": "valid_tool",
                            "arguments": "{\"value\":true}"
                        }
                    }
                ]
            },
            "finish_reason": "length"
        }]
    });

    let message = response_to_message(&response)?;

    assert!(message.metadata.output_token_limit_reached);
    assert_eq!(message.content.len(), 2);
    for (content, expected_id) in message.content.iter().zip(["call_empty", "call_valid"]) {
        let MessageContentBlock::ToolRequest(request) = content else {
            panic!("expected tool request");
        };
        assert_eq!(request.id, expected_id);
        let error = request
            .tool_call
            .as_ref()
            .expect_err("length-terminated tool call must not be executable");
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("output token limit"));
    }

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
fn test_response_to_message_empty_func_name() -> anyhow::Result<()> {
    let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
    response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] = json!("");

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
            _ => panic!("Expected invalid-request error for empty name"),
        }
    } else {
        panic!("Expected ToolRequest content");
    }

    Ok(())
}

#[test]
fn test_response_to_message_passes_names_through_to_dispatch() -> anyhow::Result<()> {
    for name in [
        "developer.shell",
        "functions.example_fn",
        "example fn",
        "???!",
    ] {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] = json!(name);

        let message = response_to_message(&response)?;

        if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().expect("tool call should parse");
            assert_eq!(tool_call.name, name, "name must pass through verbatim");
        } else {
            panic!("Expected ToolRequest content");
        }
    }

    Ok(())
}

#[test]
fn test_response_to_message_fenced_arguments() -> anyhow::Result<()> {
    let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
    response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
        json!("```json\n{\"param\": \"value\"}\n```");

    let message = response_to_message(&response)?;

    if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
        let tool_call = request.tool_call.as_ref().expect("tool call should parse");
        assert_eq!(tool_call.arguments, Some(object!({"param": "value"})));
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
                assert!(msg.contains("an array"));
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
fn test_response_to_message_api_error() -> anyhow::Result<()> {
    // Test that API responses with an "error" field return the error message
    let response = json!({
        "error": {
            "message": "You have exceeded your quota",
            "type": "insufficient_quota",
            "code": "quota_exceeded"
        }
    });

    let result = response_to_message(&response);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("API error:"));
    assert!(err.to_string().contains("You have exceeded your quota"));

    Ok(())
}

#[test]
fn test_response_to_message_api_error_unknown() -> anyhow::Result<()> {
    // Test that API responses with an "error" field but no message return "Unknown error"
    let response = json!({
        "error": {
            "type": "some_error"
        }
    });

    let result = response_to_message(&response);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("API error:"));
    assert!(err.to_string().contains("Unknown error"));

    Ok(())
}

#[test]
fn test_response_to_message_no_choices() -> anyhow::Result<()> {
    // Test that responses without "choices" return an error
    let response = json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1234567890
    });

    let result = response_to_message(&response);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("No message in API response"));

    Ok(())
}

#[test]
fn test_response_to_message_with_nested_extra_content() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_456",
                    "type": "function",
                    "function": {
                        "name": "test_tool",
                        "arguments": "{}"
                    },
                    "extra_content": {
                        "google": {
                            "thought_signature": "nested_sig_xyz789"
                        }
                    }
                }]
            }
        }]
    });

    let message = response_to_message(&response)?;
    assert_eq!(message.content.len(), 1);

    if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
        assert!(request.tool_call.is_ok());
        assert!(request.metadata.is_some());
        let metadata = request.metadata.as_ref().unwrap();
        let extra_content = metadata.get("extra_content").unwrap();
        assert_eq!(
            extra_content["google"]["thought_signature"],
            "nested_sig_xyz789"
        );
    } else {
        panic!("Expected ToolRequest content");
    }

    Ok(())
}

#[test]
fn test_response_to_message_with_multiple_extra_fields() -> anyhow::Result<()> {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_789",
                    "type": "function",
                    "function": {
                        "name": "test_tool",
                        "arguments": "{}"
                    },
                    "thoughtSignature": "sig_top_level",
                    "extra_content": {
                        "google": {
                            "thought_signature": "sig_nested"
                        }
                    },
                    "custom_field": "custom_value"
                }]
            }
        }]
    });

    let message = response_to_message(&response)?;

    if let MessageContentBlock::ToolRequest(request) = &message.content[0] {
        let metadata = request.metadata.as_ref().unwrap();
        assert_eq!(metadata.get("thoughtSignature").unwrap(), "sig_top_level");
        assert_eq!(
            metadata.get("extra_content").unwrap()["google"]["thought_signature"],
            "sig_nested"
        );
        assert_eq!(metadata.get("custom_field").unwrap(), "custom_value");
    } else {
        panic!("Expected ToolRequest content");
    }

    Ok(())
}

