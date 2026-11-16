use super::super::*;
use super::helpers::*;
use crate::conversation::message::{Message, MessageContent, MessageContentBlock};
use rmcp::model::{
    CallToolRequestParams, ContentBlock, ResourceContents, Tool,
};
use rmcp::object;
use serde_json::json;
#[test]
fn test_message_to_anthropic_spec() {
    let messages = vec![
        Message::user().with_text("Hello"),
        Message::assistant().with_text("Hi there"),
        Message::user().with_text("How are you?"),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 3);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(spec[0]["content"][0]["type"], "text");
    assert_eq!(spec[0]["content"][0]["text"], "Hello");
    assert_eq!(spec[1]["role"], "assistant");
    assert_eq!(spec[1]["content"][0]["text"], "Hi there");
    assert_eq!(spec[2]["role"], "user");
    assert_eq!(spec[2]["content"][0]["text"], "How are you?");
}

#[test]
fn test_message_to_anthropic_spec_skips_unsigned_thinking() {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::assistant().with_text("Hi there"),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["content"][0]["type"], "text");
    assert_eq!(spec[0]["content"][0]["text"], "Hi there");
}

#[test]
fn test_message_to_anthropic_spec_preserves_unsigned_thinking_when_enabled() {
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::assistant().with_text("Hi there"),
    ];

    let spec = format_messages_with_options(
        &messages,
        &AnthropicFormatOptions {
            preserve_unsigned_thinking: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["content"][0]["type"], "thinking");
    assert_eq!(spec[0]["content"][0]["thinking"], "internal");
    assert!(spec[0]["content"][0].get("signature").is_none());
    assert_eq!(spec[1]["content"][0]["text"], "Hi there");
}

#[test]
fn strip_thinking_history_removes_signed_blocks() {
    let messages = vec![Message::assistant()
        .with_content(MessageContent::thinking("", "sig"))
        .with_text("answer")];
    let opts = AnthropicFormatOptions {
        strip_thinking_history: true,
        ..Default::default()
    };
    let spec = format_messages_with_options(&messages, &opts);
    assert_eq!(
        spec[0]["content"],
        json!([{"type": "text", "text": "answer"}])
    );
}

#[test]
fn drops_signed_thinking_from_a_different_model() {
    let messages = vec![signed_thinking_from_model("claude-opus-4-1")];
    let opts = AnthropicFormatOptions {
        current_model: Some("claude-sonnet-4-5".to_string()),
        ..Default::default()
    };
    let spec = format_messages_with_options(&messages, &opts);
    let types: Vec<&str> = spec[0]["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["type"].as_str().unwrap())
        .collect();
    assert!(
        !types.contains(&"thinking"),
        "stale thinking must be dropped"
    );
    assert!(types.contains(&"text"), "text content must be preserved");
}

#[test]
fn keeps_signed_thinking_from_the_same_model() {
    let messages = vec![signed_thinking_from_model("claude-sonnet-4-5")];
    let opts = AnthropicFormatOptions {
        current_model: Some("claude-sonnet-4-5".to_string()),
        ..Default::default()
    };
    let spec = format_messages_with_options(&messages, &opts);
    assert_eq!(spec[0]["content"][0]["type"], "thinking");
    assert_eq!(spec[0]["content"][0]["signature"], "sig-abc");
}

#[test]
fn keeps_signed_thinking_when_provenance_unknown() {
    let messages =
        vec![Message::assistant().with_content(MessageContent::thinking("internal", "sig"))];
    let opts = AnthropicFormatOptions {
        current_model: Some("claude-sonnet-4-5".to_string()),
        ..Default::default()
    };
    let spec = format_messages_with_options(&messages, &opts);
    assert_eq!(spec[0]["content"][0]["type"], "thinking");
}

#[test]
fn test_tools_to_anthropic_spec() {
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

    let spec = format_tools(&tools, &AnthropicFormatOptions::default());

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[0]["name"], "calculator");
    assert_eq!(spec[0]["description"], "Calculate mathematical expressions");
    assert_eq!(spec[1]["name"], "weather");
    assert_eq!(spec[1]["description"], "Get weather information");

    // Verify cache control is added to last tool
    assert!(spec[1].get("cache_control").is_some());
}

#[test]
fn test_system_to_anthropic_spec() {
    let system = "You are a helpful assistant.";
    let spec = format_system(system, &AnthropicFormatOptions::default());

    assert!(spec.is_array());
    let spec_array = spec.as_array().unwrap();
    assert_eq!(spec_array.len(), 1);
    assert_eq!(spec_array[0]["type"], "text");
    assert_eq!(spec_array[0]["text"], system);
    assert!(spec_array[0].get("cache_control").is_some());
}

#[test]
fn test_tool_error_handling_maintains_pairing() {
    use crate::conversation::message::Message;
    use rmcp::model::{ErrorCode, ErrorData};

    let messages = vec![
        Message::assistant().with_tool_request(
            "tool_1",
            Ok(CallToolRequestParams::new("calculator")
                .with_arguments(object!({"expression": "2 + 2"}))),
        ),
        Message::user().with_tool_response(
            "tool_1",
            Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Tool failed".to_string(),
                None,
            )),
        ),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 2);

    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["content"][0]["type"], "tool_use");
    assert_eq!(spec[0]["content"][0]["id"], "tool_1");
    assert_eq!(spec[0]["content"][0]["name"], "calculator");

    assert_eq!(spec[1]["role"], "user");
    assert_eq!(spec[1]["content"][0]["type"], "tool_result");
    assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
    assert_eq!(
        spec[1]["content"][0]["content"],
        "Error: -32603: Tool failed"
    );
    assert_eq!(spec[1]["content"][0]["is_error"], true);
}

#[test]
fn test_whitespace_only_text_blocks_are_skipped() {
    let messages = vec![
        Message::user().with_text("Hello"),
        Message::assistant().with_text("").with_tool_request(
            "tool_1",
            Ok(CallToolRequestParams::new("search").with_arguments(object!({"query": "test"}))),
        ),
        Message::user()
            .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 3);

    let assistant_content = spec[1]["content"].as_array().unwrap();
    assert_eq!(assistant_content.len(), 1);
    assert_eq!(assistant_content[0]["type"], "tool_use");
}

#[test]
fn test_tool_response_with_resource_content() {
    use rmcp::model::{CallToolResult, ContentBlock};

    let resource_content = ContentBlock::embedded_text(
        "file:///test/file.txt",
        "This is the file content from a resource",
    );

    let messages = vec![
        Message::assistant().with_tool_request(
            "tool_1",
            Ok(CallToolRequestParams::new("view_file")
                .with_arguments(object!({"path": "/test/file.txt"}))),
        ),
        Message::user().with_tool_response(
            "tool_1",
            Ok(CallToolResult::success(vec![resource_content])),
        ),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[1]["role"], "user");
    assert_eq!(spec[1]["content"][0]["type"], "tool_result");
    assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
    assert_eq!(
        spec[1]["content"][0]["content"],
        "This is the file content from a resource"
    );
}

#[test]
fn test_tool_response_with_mixed_content() {
    use rmcp::model::{CallToolResult, ContentBlock};

    let text_content = ContentBlock::text("Summary: file loaded");
    let resource_content =
        ContentBlock::embedded_text("file:///test/file.txt", "File content here");

    let messages = vec![
        Message::assistant().with_tool_request(
            "tool_1",
            Ok(CallToolRequestParams::new("view_file")
                .with_arguments(object!({"path": "/test/file.txt"}))),
        ),
        Message::user().with_tool_response(
            "tool_1",
            Ok(CallToolResult::success(vec![
                text_content,
                resource_content,
            ])),
        ),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec[1]["content"][0]["type"], "tool_result");
    assert_eq!(
        spec[1]["content"][0]["content"],
        "Summary: file loaded\nFile content here"
    );
}

#[test]
fn test_tool_response_forwards_image_resource_as_image_block() {
    use rmcp::model::CallToolResult;

    let image = ContentBlock::resource(ResourceContents::BlobResourceContents {
        uri: "file:///shot.png".to_string(),
        mime_type: Some("image/png".to_string()),
        blob: "aGVsbG8=".to_string(),
        meta: None,
    });

    let messages = vec![
        Message::assistant()
            .with_tool_request("tool_1", Ok(CallToolRequestParams::new("screenshot"))),
        Message::user().with_tool_response("tool_1", Ok(CallToolResult::success(vec![image]))),
    ];

    let spec = format_messages(&messages);

    let block = &spec[1]["content"][0]["content"][0];
    assert_eq!(block["type"], "image");
    assert_eq!(block["source"]["type"], "base64");
    assert_eq!(block["source"]["media_type"], "image/png");
    assert_eq!(block["source"]["data"], "aGVsbG8=");
}

#[test]
fn test_tool_response_unsupported_image_mime_falls_back_to_text() {
    use rmcp::model::CallToolResult;

    // image/svg+xml is not a Claude-supported image type, so it must fall
    // through to text rather than an image block.
    let svg = ContentBlock::resource(ResourceContents::BlobResourceContents {
        uri: "file:///diagram.svg".to_string(),
        mime_type: Some("image/svg+xml".to_string()),
        blob: "aGVsbG8=".to_string(),
        meta: None,
    });

    let messages = vec![
        Message::assistant()
            .with_tool_request("tool_1", Ok(CallToolRequestParams::new("render"))),
        Message::user().with_tool_response("tool_1", Ok(CallToolResult::success(vec![svg]))),
    ];

    let spec = format_messages(&messages);

    // Serializer contract: an unsupported image type is not emitted as an
    // image block — the content collapses to a text string.
    assert!(spec[1]["content"][0]["content"].is_string());
}

#[test]
fn test_tool_response_forwards_raw_image_as_image_block() {
    use rmcp::model::CallToolResult;

    let image = ContentBlock::image("aGVsbG8=", "image/png");

    let messages = vec![
        Message::assistant()
            .with_tool_request("tool_1", Ok(CallToolRequestParams::new("screenshot"))),
        Message::user().with_tool_response("tool_1", Ok(CallToolResult::success(vec![image]))),
    ];

    let spec = format_messages(&messages);

    let block = &spec[1]["content"][0]["content"][0];
    assert_eq!(block["type"], "image");
    assert_eq!(block["source"]["type"], "base64");
    assert_eq!(block["source"]["media_type"], "image/png");
    assert_eq!(block["source"]["data"], "aGVsbG8=");
}

#[test]
fn test_tool_response_unsupported_raw_image_mime_falls_back_to_text() {
    use rmcp::model::CallToolResult;

    // image/svg+xml is not a Claude-supported image type, so a raw image with
    // that mime must fall back to a text marker rather than an image block
    // (which the provider would reject).
    let image = ContentBlock::image("aGVsbG8=", "image/svg+xml");

    let messages = vec![
        Message::assistant()
            .with_tool_request("tool_1", Ok(CallToolRequestParams::new("render"))),
        Message::user().with_tool_response("tool_1", Ok(CallToolResult::success(vec![image]))),
    ];

    let spec = format_messages(&messages);

    assert_eq!(spec[1]["content"][0]["content"], "[Image: image/svg+xml]");
}

#[test]
fn test_args_to_input_value_returns_empty_object_for_none() {
    let value = args_to_input_value(None);
    assert!(value.is_object(), "expected JSON object, got {value:?}");
    assert_eq!(value, json!({}));
    assert!(!value.is_null());
}

#[test]
fn test_unparseable_tool_request_emits_placeholder_tool_use() {
    use rmcp::model::{ErrorCode, ErrorData};

    let err = ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        "Tool arguments for id call_bad must be a JSON object".to_string(),
        None,
    );
    let mut response = Message::user();
    response.add_tool_response_with_metadata("call_bad", Err(err.clone()), None);
    let messages = vec![
        Message::assistant().with_tool_request("call_bad", Err(err)),
        response,
    ];

    let spec = format_messages(&messages);

    let mut open = std::collections::HashSet::new();
    for m in &spec {
        for block in m["content"].as_array().into_iter().flatten() {
            match block["type"].as_str() {
                Some("tool_use") => {
                    open.insert(block["id"].as_str().unwrap().to_string());
                }
                Some("tool_result") => {
                    let id = block["tool_use_id"].as_str().unwrap();
                    assert!(open.contains(id), "orphan tool_result for id {id:?}");
                }
                _ => {}
            }
        }
    }
    assert!(open.contains("call_bad"));
}

#[test]
fn test_args_to_input_value_preserves_existing_args() {
    let args = object!({"query": "rust"});
    let value = args_to_input_value(Some(args));
    assert_eq!(value, json!({"query": "rust"}));
}

#[test]
fn test_parameterless_tool_request_serializes_input_as_empty_object() {
    // Regression test for #9287: when arguments is None (parameterless
    // MCP tool, session reload, or provider switching) the `input` field
    // must serialize as `{}` so the Anthropic API does not reject the
    // replayed tool_use block with a 400 error.
    let messages = vec![
        Message::assistant()
            .with_tool_request("tool_1", Ok(CallToolRequestParams::new("list_things"))),
        Message::user()
            .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
    ];

    let spec = format_messages(&messages);

    let input = &spec[0]["content"][0]["input"];
    assert!(input.is_object(), "expected object, got {input:?}");
    assert!(!input.is_null());
    assert_eq!(input, &json!({}));
}

