use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::model::ModelConfig;
use rmcp::model::CallToolRequestParams;
use rmcp::object;
use serde_json::json;

    #[test]
    fn test_tool_response_with_image_serializes_as_typed_array() {
        use crate::conversation::message::Message;
        use rmcp::model::{CallToolResult, ContentBlock};

        let messages = vec![
            Message::user().with_content(MessageContentBlock::tool_response(
                "call_1",
                Ok(CallToolResult::success(vec![
                    ContentBlock::text("caption"),
                    ContentBlock::image("a+/=".to_string(), "image/png".to_string()),
                ])),
            )),
        ];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: Some(true),
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["type"], "function_call_output");
        assert_eq!(input[0]["call_id"], "call_1");

        let output = input[0]["output"].as_array().unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], json!({"type": "input_text", "text": "caption"}));
        assert_eq!(
            output[1],
            json!({"type": "input_image", "image_url": "data:image/png;base64,a+/="})
        );
    }

    #[test]
    fn test_tool_request_serializes_function_call_with_arguments() {
        use crate::conversation::message::Message;

        let messages = vec![Message::assistant().with_tool_request(
            "call_1",
            Ok(CallToolRequestParams::new("search")
                .with_arguments(object!({"q": "rust", "limit": 2}))),
        )];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["type"], "function_call");
        assert_eq!(input[0]["call_id"], "call_1");
        assert_eq!(input[0]["name"], "search");

        let args: serde_json::Value =
            serde_json::from_str(input[0]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["q"], "rust");
        assert_eq!(args["limit"], 2);
    }

    #[test]
    fn test_tool_request_none_arguments_serializes_empty_object() {
        use crate::conversation::message::Message;

        let messages = vec![Message::assistant()
            .with_tool_request("call_1", Ok(CallToolRequestParams::new("noop")))];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["type"], "function_call");
        assert_eq!(input[0]["name"], "noop");
        assert_eq!(input[0]["arguments"], "{}");
    }

    #[test]
    fn test_text_flushed_before_tool_request() {
        use crate::conversation::message::Message;

        let messages = vec![Message::assistant()
            .with_text("planning")
            .with_tool_request(
                "call_1",
                Ok(CallToolRequestParams::new("shell").with_arguments(object!({"command": "ls"}))),
            )];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input.len(), 2);
        assert_eq!(input[0]["role"], "assistant");
        assert_eq!(input[0]["content"][0]["type"], "output_text");
        assert_eq!(input[0]["content"][0]["text"], "planning");
        assert_eq!(input[1]["type"], "function_call");
    }

    #[test]
    fn test_text_flushed_before_tool_response() {
        use crate::conversation::message::Message;
        use rmcp::model::{CallToolResult, ContentBlock};

        let messages = vec![Message::user().with_text("context").with_content(
            MessageContentBlock::tool_response(
                "call_1",
                Ok(CallToolResult::success(vec![ContentBlock::text("done")])),
            ),
        )];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input.len(), 2);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"][0]["type"], "input_text");
        assert_eq!(input[0]["content"][0]["text"], "context");
        assert_eq!(input[1]["type"], "function_call_output");
        assert_eq!(input[1]["output"], "done");
    }

    #[test]
    fn test_tool_response_error_serializes_with_error_prefix() {
        use crate::conversation::message::Message;
        use rmcp::model::{ErrorCode, ErrorData};

        let messages = vec![
            Message::user().with_content(MessageContentBlock::tool_response(
                "call_err",
                Err(ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: "file not found".into(),
                    data: None,
                }),
            )),
        ];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["type"], "function_call_output");
        assert_eq!(input[0]["call_id"], "call_err");
        assert_eq!(input[0]["output"], "Error: file not found");
    }

    #[test]
    fn test_image_only_message_serializes() {
        use crate::conversation::message::Message;

        let messages = vec![Message::user().with_image("aW1n", "image/png")];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: Some(true),
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["role"], "user");
        let content = input[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "input_image");
        assert_eq!(content[0]["image_url"], "data:image/png;base64,aW1n");
    }

    #[test]
    fn test_multiple_images_preserved_in_order() {
        use crate::conversation::message::Message;

        let messages = vec![Message::user()
            .with_text("compare")
            .with_image("img1", "image/png")
            .with_image("img2", "image/jpeg")];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: Some(true),
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["role"], "user");
        let content = input[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 3);
        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[0]["text"], "compare");
        assert_eq!(content[1]["type"], "input_image");
        assert_eq!(content[1]["image_url"], "data:image/png;base64,img1");
        assert_eq!(content[2]["type"], "input_image");
        assert_eq!(content[2]["image_url"], "data:image/jpeg;base64,img2");
    }

    #[test]
    fn test_user_image_omitted_when_not_vision() {
        use crate::conversation::message::Message;

        let messages = vec![Message::user()
            .with_text("describe this image")
            .with_image("aW1hZ2VkYXRh", "image/png")];

        // Non-vision: explicit image content is replaced with a text placeholder
        // at format time — session history is untouched.
        let model_config = ModelConfig::new("o3-mini");
        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["role"], "user");
        let content = input[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[0]["text"], "describe this image");
        assert_eq!(content[1]["type"], "input_text");
        assert_eq!(
            content[1]["text"],
            "[image omitted: model does not support vision]"
        );

        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("input_image"));
        assert!(!serialized.contains("data:image"));
    }

    #[test]
    fn test_tool_response_image_omitted_when_not_vision() {
        use crate::conversation::message::Message;
        use rmcp::model::{CallToolResult, ContentBlock};

        let messages = vec![
            Message::user().with_content(MessageContentBlock::tool_response(
                "call_1",
                Ok(CallToolResult::success(vec![
                    ContentBlock::text("caption"),
                    ContentBlock::image("a+/=".to_string(), "image/png".to_string()),
                ])),
            )),
        ];

        // Non-vision: images in tool results are omitted from the output
        // string (with a note) instead of being emitted as input_image items.
        let model_config = ModelConfig::new("o3-mini");
        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["type"], "function_call_output");
        let output = input[0]["output"].as_str().unwrap();
        assert!(output.contains("caption"));
        assert!(output.contains("[image omitted: model does not support vision]"));

        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("input_image"));
        assert!(!serialized.contains("data:image"));
    }

    #[test]
    fn test_assistant_text_uses_output_text_with_annotations() {
        use crate::conversation::message::Message;

        let messages = vec![Message::assistant().with_text("hello")];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        assert_eq!(input[0]["role"], "assistant");
        assert_eq!(input[0]["content"][0]["type"], "output_text");
        assert_eq!(input[0]["content"][0]["text"], "hello");
        assert_eq!(input[0]["content"][0]["annotations"], serde_json::json!([]));
    }

