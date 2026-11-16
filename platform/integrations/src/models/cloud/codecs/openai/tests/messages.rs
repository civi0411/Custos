use super::super::*;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock};
use rmcp::object;
use serde_json::Value;

#[test]
fn test_format_messages() -> anyhow::Result<()> {
    let message = Message::user().with_text("Hello");
    let spec = format_messages(&[message], &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(spec[0]["content"], "Hello");
    Ok(())
}

#[test]
fn test_format_messages_complex() -> anyhow::Result<()> {
    let mut messages = vec![
        Message::assistant().with_text("Hello!"),
        Message::user().with_text("How are you?"),
        Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams::new("example")
                .with_arguments(object!({"param1": "value1"}))),
        ),
    ];

    // Get the ID from the tool request to use in the response
    let tool_id = if let MessageContentBlock::ToolRequest(request) = &messages[2].content[0] {
        request.id.clone()
    } else {
        panic!("should be tool request");
    };

    messages.push(Message::user().with_tool_response(
        tool_id,
        Ok(CallToolResult::success(vec![ContentBlock::text("Result")])),
    ));

    let spec = format_messages(&messages, &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 4);
    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["content"], "Hello!");
    assert_eq!(spec[1]["role"], "user");
    assert_eq!(spec[1]["content"], "How are you?");
    assert_eq!(spec[2]["role"], "assistant");
    assert!(spec[2]["tool_calls"].is_array());
    assert_eq!(spec[3]["role"], "tool");
    assert_eq!(spec[3]["content"], "Result");
    assert_eq!(spec[3]["tool_call_id"], spec[2]["tool_calls"][0]["id"]);

    Ok(())
}

#[test]
fn test_format_messages_multiple_content() -> anyhow::Result<()> {
    let mut messages = vec![Message::assistant().with_tool_request(
        "tool1",
        Ok(CallToolRequestParams::new("example").with_arguments(object!({"param1": "value1"}))),
    )];

    // Get the ID from the tool request to use in the response
    let tool_id = if let MessageContentBlock::ToolRequest(request) = &messages[0].content[0] {
        request.id.clone()
    } else {
        panic!("should be tool request");
    };

    messages.push(Message::user().with_tool_response(
        tool_id,
        Ok(CallToolResult::success(vec![ContentBlock::text("Result")])),
    ));

    let spec = format_messages(&messages, &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 2);
    assert_eq!(spec[0]["role"], "assistant");
    assert!(spec[0]["tool_calls"].is_array());
    assert_eq!(spec[1]["role"], "tool");
    assert_eq!(spec[1]["content"], "Result");
    assert_eq!(spec[1]["tool_call_id"], spec[0]["tool_calls"][0]["id"]);

    Ok(())
}

#[test]
fn test_format_messages_with_image_path() -> anyhow::Result<()> {
    // Create a temporary PNG file with valid PNG magic numbers
    let temp_dir = tempfile::tempdir()?;
    let png_path = temp_dir.path().join("test.png");
    let png_data = [
        0x89, 0x50, 0x4E, 0x47, // PNG magic number
        0x0D, 0x0A, 0x1A, 0x0A, // PNG header
        0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
    ];
    std::fs::write(&png_path, png_data)?;
    let png_path_str = png_path.to_str().unwrap();

    // Create user message with image path - should load the image when vision is supported
    let user_message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
    let spec = format_messages_with_options(
        &[user_message],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");

    // Content should be an array with text and image
    let content = spec[0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"].as_str().unwrap().contains(png_path_str));
    assert_eq!(content[1]["type"], "image_url");
    assert!(content[1]["image_url"]["url"]
        .as_str()
        .unwrap()
        .starts_with("data:image/png;base64,"));

    // Create assistant message with same text - should NOT load the image
    let assistant_message =
        Message::assistant().with_text(format!("I saved the output to {}", png_path_str));
    let spec = format_messages_with_options(
        &[assistant_message],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");

    // Content should be plain text, NOT an array with image
    let content = spec[0]["content"].as_str();
    assert!(
        content.is_some(),
        "Assistant message content should be a string, not an array with image"
    );
    assert!(content.unwrap().contains(png_path_str));

    Ok(())
}

#[test]
fn test_format_messages_with_image_path_passthrough_when_not_vision() -> anyhow::Result<()> {
    // Create a temporary PNG file with valid PNG magic numbers
    let temp_dir = tempfile::tempdir()?;
    let png_path = temp_dir.path().join("test.png");
    let png_data = [
        0x89, 0x50, 0x4E, 0x47, // PNG magic number
        0x0D, 0x0A, 0x1A, 0x0A, // PNG header
        0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
    ];
    std::fs::write(&png_path, png_data)?;
    let png_path_str = png_path.to_str().unwrap();

    // User message with image path: with vision NOT supported, the path must
    // pass through as plain text (a non-vision model can forward it to a
    // vision subagent instead of 400ing on an injected image_url block).
    let user_message = Message::user().with_text(format!("Here is an image: {}", png_path_str));

    let spec = format_messages_with_options(
        std::slice::from_ref(&user_message),
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: false,
            ..Default::default()
        },
    );
    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");
    // Single text block collapses to a plain string — the path survives verbatim.
    let content = spec[0]["content"].as_str().unwrap();
    assert!(content.contains(png_path_str));
    assert!(!content.contains("image_url"));
    assert!(!content.contains("data:image"));

    // Default options (bare format_messages wrapper) must behave the same:
    // unaffirmed vision -> passthrough.
    let spec = format_messages(&[user_message], &ImageFormat::OpenAi);
    let content = spec[0]["content"].as_str().unwrap();
    assert!(content.contains(png_path_str));
    assert!(!content.contains("image_url"));

    Ok(())
}

#[test]
fn test_create_request_threads_supports_vision() -> anyhow::Result<()> {
    // Create a temporary PNG file with valid PNG magic numbers
    let temp_dir = tempfile::tempdir()?;
    let png_path = temp_dir.path().join("test.png");
    let png_data = [
        0x89, 0x50, 0x4E, 0x47, // PNG magic number
        0x0D, 0x0A, 0x1A, 0x0A, // PNG header
        0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
    ];
    std::fs::write(&png_path, png_data)?;
    let png_path_str = png_path.to_str().unwrap();
    let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));

    // Vision affirmed: path is converted to an image_url block.
    let vision = ModelConfig::new("gpt-4o").with_vision_support(true);
    let request = create_request(
        &vision,
        "system",
        std::slice::from_ref(&message),
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let messages = request["messages"].as_array().unwrap();
    let content = messages[1]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[1]["type"], "image_url");

    // Unknown (None): passthrough, no image_url.
    let unknown = ModelConfig::new("gpt-4o");
    let request = create_request(
        &unknown,
        "system",
        std::slice::from_ref(&message),
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let messages = request["messages"].as_array().unwrap();
    let content = messages[1]["content"].as_str().unwrap();
    assert!(content.contains(png_path_str));
    assert!(!content.contains("image_url"));

    // Explicitly non-vision: passthrough, no image_url.
    let non_vision = ModelConfig::new("gpt-4o").with_vision_support(false);
    let request = create_request(
        &non_vision,
        "system",
        &[message],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let messages = request["messages"].as_array().unwrap();
    let content = messages[1]["content"].as_str().unwrap();
    assert!(!content.contains("image_url"));

    Ok(())
}

#[test]
fn test_format_messages_with_image_block_passthrough_when_not_vision() -> anyhow::Result<()> {
    let user_message = Message::user().with_image("aW1hZ2VkYXRh", "image/png");

    // Non-vision: explicit image content is replaced with a text placeholder
    // at format time — session history is untouched, so a vision model (or a
    // delegated vision subagent) can still see the real image later.
    let spec = format_messages_with_options(
        std::slice::from_ref(&user_message),
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: false,
            ..Default::default()
        },
    );
    assert_eq!(spec.len(), 1);
    let content = spec[0]["content"].as_str().unwrap();
    assert_eq!(content, "[image omitted: model does not support vision]");
    assert!(!content.contains("image_url"));
    assert!(!content.contains("data:image"));

    // Vision: the image is converted to an image_url block (existing behavior).
    let spec = format_messages_with_options(
        &[user_message],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );
    let content = spec[0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "image_url");
    assert!(content[0]["image_url"]["url"]
        .as_str()
        .unwrap()
        .starts_with("data:image/png;base64,"));

    Ok(())
}

#[test]
fn test_tool_response_image_omitted_when_not_vision() -> anyhow::Result<()> {
    let tool_response = Message::user().with_tool_response(
        "tool1",
        Ok(rmcp::model::CallToolResult::success(vec![
            rmcp::model::ContentBlock::image("aW1hZ2VkYXRh", "image/png"),
        ])),
    );

    // Non-vision: the separate user image message is NOT emitted — this is
    // what un-bricks sessions (a converted image in the next request 400s).
    let spec = format_messages_with_options(
        std::slice::from_ref(&tool_response),
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: false,
            ..Default::default()
        },
    );
    let serialized = serde_json::to_value(&spec).unwrap().to_string();
    assert!(!serialized.contains("image_url"));
    assert!(serialized.contains(
        "This tool result included an image that was omitted as the model does not support vision."
    ));

    // Vision: the separate user image message IS emitted (existing behavior).
    let spec = format_messages_with_options(
        &[tool_response],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );
    let serialized = serde_json::to_value(&spec).unwrap().to_string();
    assert!(serialized.contains("image_url"));
    assert!(serialized
        .contains("This tool result included an image that is uploaded in the next message."));

    Ok(())
}

#[test]
fn test_format_messages_with_text_and_image_preserves_order() {
    // Text before image: order should be [text, image]
    let msg_text_first = Message::user()
        .with_text("Describe this image")
        .with_image("aW1hZ2VkYXRh", "image/png");

    let spec = format_messages_with_options(
        &[msg_text_first],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );
    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");

    let content = spec[0]["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(content.len(), 2, "expected text + image entries");
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "Describe this image");
    assert_eq!(content[1]["type"], "image_url");

    // Image before text: order should be [image, text]
    let msg_image_first = Message::user()
        .with_image("aW1hZ2VkYXRh", "image/png")
        .with_text("What do you see?");

    let spec2 = format_messages_with_options(
        &[msg_image_first],
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: true,
            ..Default::default()
        },
    );
    let content2 = spec2[0]["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(content2.len(), 2, "expected image + text entries");
    assert_eq!(content2[0]["type"], "image_url");
    assert_eq!(content2[1]["type"], "text");
    assert_eq!(content2[1]["text"], "What do you see?");
}

#[test]
fn test_format_messages_tool_request_with_none_arguments() -> anyhow::Result<()> {
    // Test that tool calls with None arguments are formatted as "{}" string
    let message = Message::assistant()
        .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool")));

    let spec = format_messages(&[message], &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    assert!(spec[0]["tool_calls"].is_array());

    let tool_call = &spec[0]["tool_calls"][0];
    assert_eq!(tool_call["id"], "tool1");
    assert_eq!(tool_call["type"], "function");
    assert_eq!(tool_call["function"]["name"], "test_tool");
    // This should be the string "{}", not null
    assert_eq!(tool_call["function"]["arguments"], "{}");

    Ok(())
}

#[test]
fn test_format_messages_tool_request_with_some_arguments() -> anyhow::Result<()> {
    // Test that tool calls with Some arguments are properly JSON-serialized
    let message = Message::assistant().with_tool_request(
        "tool1",
        Ok(CallToolRequestParams::new("test_tool")
            .with_arguments(object!({"param": "value", "number": 42}))),
    );

    let spec = format_messages(&[message], &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    assert!(spec[0]["tool_calls"].is_array());

    let tool_call = &spec[0]["tool_calls"][0];
    assert_eq!(tool_call["id"], "tool1");
    assert_eq!(tool_call["type"], "function");
    assert_eq!(tool_call["function"]["name"], "test_tool");
    // This should be a JSON string representation
    let args_str = tool_call["function"]["arguments"].as_str().unwrap();
    let parsed_args: Value = serde_json::from_str(args_str)?;
    assert_eq!(parsed_args["param"], "value");
    assert_eq!(parsed_args["number"], 42);

    Ok(())
}

#[test]
fn test_format_messages_multiple_text_blocks() -> anyhow::Result<()> {
    let message = Message::user()
        .with_text("--- Resource: file:///test.md ---\n# Test\n\n---\n")
        .with_text(" What is in the file?");

    let spec = format_messages(&[message], &ImageFormat::OpenAi);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");
    assert_eq!(
        spec[0]["content"],
        "--- Resource: file:///test.md ---\n# Test\n\n---\n\n What is in the file?"
    );
    Ok(())
}

