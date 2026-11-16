use super::super::*;
use crate::conversation::message::{Message, MessageContent, MessageContentBlock};
use crate::images::ImageFormat;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, Tool};
use rmcp::object;
use serde_json::Value;

#[test]
fn test_format_messages() -> anyhow::Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0].role, "user");
        assert_eq!(spec[0].content, "Hello");
        Ok(())
    }

    #[test]
    fn keeps_reasoning_block_from_the_same_model() {
        use crate::conversation::message::InferenceMetadata;
        let message = Message::assistant()
            .with_content(MessageContent::thinking("internal", "sig-xyz"))
            .with_text("answer")
            .with_inference(InferenceMetadata {
                provider: "databricks".to_string(),
                requested_model: "databricks-claude-opus-4-1".to_string(),
                resolved_model: None,
                provider_session_id: None,
            });

        let spec = format_messages(
            &[message],
            &ImageFormat::OpenAi,
            Some("databricks-claude-opus-4-1"),
            false,
        );
        let has_reasoning = spec[0]
            .content
            .as_array()
            .map(|a| a.iter().any(|c| c["type"] == "reasoning"))
            .unwrap_or(false);
        assert!(has_reasoning, "same-model reasoning must be kept");
    }

    #[test]
    fn drops_reasoning_block_from_a_different_model() {
        use crate::conversation::message::InferenceMetadata;
        let message = Message::assistant()
            .with_content(MessageContent::thinking("internal", "sig-xyz"))
            .with_text("answer")
            .with_inference(InferenceMetadata {
                provider: "databricks".to_string(),
                requested_model: "databricks-claude-opus-4-1".to_string(),
                resolved_model: None,
                provider_session_id: None,
            });

        let spec = format_messages(
            &[message],
            &ImageFormat::OpenAi,
            Some("databricks-claude-sonnet-4-5"),
            false,
        );
        let has_reasoning = spec[0]
            .content
            .as_array()
            .map(|a| a.iter().any(|c| c["type"] == "reasoning"))
            .unwrap_or(false);
        assert!(!has_reasoning, "stale reasoning block must be dropped");
        assert_eq!(spec[0].content, Value::String("answer".to_string()));
    }

    #[test]
    fn keeps_reasoning_when_endpoint_matches_despite_upstream_resolved_name() {
        use crate::conversation::message::InferenceMetadata;
        let message = Message::assistant()
            .with_content(MessageContent::thinking("internal", "sig-xyz"))
            .with_text("answer")
            .with_inference(InferenceMetadata {
                provider: "databricks".to_string(),
                requested_model: "databricks-claude-opus-4-1".to_string(),
                resolved_model: Some("claude-opus-4.1".to_string()),
                provider_session_id: None,
            });

        let spec = format_messages(
            &[message],
            &ImageFormat::OpenAi,
            Some("databricks-claude-opus-4-1"),
            false,
        );
        let has_reasoning = spec[0]
            .content
            .as_array()
            .map(|a| a.iter().any(|c| c["type"] == "reasoning"))
            .unwrap_or(false);
        assert!(
            has_reasoning,
            "same-endpoint reasoning must be kept even when resolved_model differs"
        );
    }

    #[test]
    fn keeps_reasoning_when_current_model_matches_upstream_resolved_name() {
        use crate::conversation::message::InferenceMetadata;
        let message = Message::assistant()
            .with_content(MessageContent::thinking("internal", "sig-xyz"))
            .with_text("answer")
            .with_inference(InferenceMetadata {
                provider: "databricks".to_string(),
                requested_model: "my-claude-endpoint".to_string(),
                resolved_model: Some("claude-opus-4.1".to_string()),
                provider_session_id: None,
            });

        let spec = format_messages(
            &[message],
            &ImageFormat::OpenAi,
            Some("claude-opus-4.1"),
            false,
        );
        let has_reasoning = spec[0]
            .content
            .as_array()
            .map(|a| a.iter().any(|c| c["type"] == "reasoning"))
            .unwrap_or(false);
        assert!(
            has_reasoning,
            "reasoning must be kept when current_model matches the upstream resolved_model"
        );
    }

    #[test]
    fn test_format_messages_sanitizes_resource_tool_response() {
        let message = Message::user().with_tool_response(
            "tool1",
            Ok(CallToolResult::success(vec![ContentBlock::embedded_text(
                "file:///result.txt",
                "visible\u{E0041}text",
            )])),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);

        assert_eq!(spec[0].content, "visibletext");
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let spec = format_tools(std::slice::from_ref(&tool), "gpt-4o")?;
        assert_eq!(
            spec[0]["function"]["parameters"]["$schema"],
            "http://json-schema.org/draft-07/schema#"
        );

        // Databricks Gemini endpoints use OpenAI-compatible format, so tools use
        // "parameters" (not "parametersJsonSchema") regardless of model family.
        let spec = format_tools(std::slice::from_ref(&tool), "gemini-2-5-flash")?;
        assert!(spec[0]["function"].get("parametersJsonSchema").is_none());
        assert!(spec[0]["function"].get("parameters").is_some());
        assert_eq!(spec[0]["function"]["parameters"]["type"], "object");

        let spec = format_tools(&[tool], "databricks-gemini-3-pro")?;
        assert!(spec[0]["function"].get("parametersJsonSchema").is_none());
        assert!(spec[0]["function"].get("parameters").is_some());
        assert_eq!(spec[0]["function"]["parameters"]["type"], "object");

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

        let tool_id = if let MessageContentBlock::ToolRequest(request) = &messages[2].content[0] {
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![ContentBlock::text("Result")])),
        ));

        let as_value = serde_json::to_value(format_messages(
            &messages,
            &ImageFormat::OpenAi,
            None,
            false,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();

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

        let tool_id = if let MessageContentBlock::ToolRequest(request) = &messages[0].content[0] {
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![ContentBlock::text("Result")])),
        ));

        let as_value = serde_json::to_value(format_messages(
            &messages,
            &ImageFormat::OpenAi,
            None,
            false,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[1]["role"], "tool");
        assert_eq!(spec[1]["content"], "Result");
        assert_eq!(spec[1]["tool_call_id"], spec[0]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_tools_duplicate() -> anyhow::Result<()> {
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let tool2 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let result = format_tools(&[tool1, tool2], "gpt-4o");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Create message with image path
        let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let as_value = serde_json::to_value(format_messages(
            &[message],
            &ImageFormat::OpenAi,
            None,
            true,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();

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

        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path_passthrough_when_not_vision() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Without vision support the path must pass through as plain text —
        // a non-vision model can forward it to a vision subagent instead of
        // 400ing on an injected image_url block.
        let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let as_value = serde_json::to_value(format_messages(
            &[message],
            &ImageFormat::OpenAi,
            None,
            false,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        let content = spec[0]["content"].as_str().unwrap();
        assert!(content.contains(png_path_str));
        assert!(!content.contains("image_url"));
        assert!(!content.contains("data:image"));

        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_block_passthrough_when_not_vision() -> anyhow::Result<()> {
        let message = Message::user().with_image("aW1hZ2VkYXRh", "image/png");

        // Non-vision: explicit image content is replaced with a text placeholder
        // at format time — session history is untouched.
        let as_value = serde_json::to_value(format_messages(
            std::slice::from_ref(&message),
            &ImageFormat::OpenAi,
            None,
            false,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();
        let content = spec[0]["content"].as_str().unwrap();
        assert_eq!(content, "[image omitted: model does not support vision]");
        assert!(!content.contains("image_url"));

        // Vision: the image is converted to an image_url block (existing behavior).
        let as_value = serde_json::to_value(format_messages(
            &[message],
            &ImageFormat::OpenAi,
            None,
            true,
        ))
        .unwrap();
        let spec = as_value.as_array().unwrap();
        let content = spec[0]["content"].as_array().unwrap();
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

        // Non-vision: no separate user image message is emitted — this is what
        // un-bricks sessions (a converted image in the next request 400s).
        let as_value = serde_json::to_value(format_messages(
            std::slice::from_ref(&tool_response),
            &ImageFormat::OpenAi,
            None,
            false,
        ))
        .unwrap();
        let serialized = as_value.to_string();
        assert!(!serialized.contains("image_url"));
        assert!(serialized.contains(
            "This tool result included an image that was omitted as the model does not support vision."
        ));

        // Vision: the separate user image message IS emitted (existing behavior).
        let as_value = serde_json::to_value(format_messages(
            &[tool_response],
            &ImageFormat::OpenAi,
            None,
            true,
        ))
        .unwrap();
        let serialized = as_value.to_string();
        assert!(serialized.contains("image_url"));
        assert!(serialized
            .contains("This tool result included an image that is uploaded in the next message."));

        Ok(())
    }
