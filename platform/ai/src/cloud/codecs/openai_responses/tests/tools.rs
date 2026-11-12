use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::model::ModelConfig;
use rmcp::model::CallToolRequestParams;
use rmcp::object;
use serde_json::json;

    #[test]
    fn test_function_call_output_requires_call_id_or_id() {
        let output_items = vec![ResponseOutputItemInfo::FunctionCall {
            id: None,
            status: None,
            call_id: None,
            name: "shell".to_string(),
            arguments: "{}".to_string(),
        }];

        let error = process_streaming_output_items(output_items, false).unwrap_err();
        assert!(
            error.to_string().contains("missing call_id and id"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_streaming_output_items_sanitize_tool_arguments() -> anyhow::Result<()> {
        let output_items = vec![
            ResponseOutputItemInfo::Message {
                id: Some("msg_1".to_string()),
                status: Some("completed".to_string()),
                role: "assistant".to_string(),
                content: vec![ContentBlockPart::ToolCall {
                    id: "call_\u{E0041}1".to_string(),
                    name: "sh\u{E0041}ell".to_string(),
                    arguments: serde_json::to_string(
                        &json!({"prompt": "visible e\u{301}\u{E0041}text"}),
                    )?,
                }],
            },
            ResponseOutputItemInfo::FunctionCall {
                id: None,
                status: Some("completed".to_string()),
                call_id: Some("call_\u{E0042}2".to_string()),
                name: "sh\u{E0042}ell".to_string(),
                arguments: serde_json::to_string(
                    &json!({"prompt": "visible e\u{301}\u{E0042}text"}),
                )?,
            },
        ];

        let content = process_streaming_output_items(output_items, false)?;
        for (content, expected_id) in content.into_iter().zip(["call_1", "call_2"]) {
            let MessageContentBlock::ToolRequest(tool_request) = content else {
                panic!("expected tool request content");
            };
            assert_eq!(tool_request.id, expected_id);
            let tool_call = tool_request.tool_call.expect("expected valid tool call");
            assert_eq!(tool_call.name, "shell");
            assert_eq!(
                tool_call
                    .arguments
                    .expect("expected arguments")
                    .get("prompt"),
                Some(&json!("visible e\u{301}text"))
            );
        }

        Ok(())
    }

    #[test]
    fn test_streaming_output_items_reject_sanitized_tool_request_id_collisions(
    ) -> anyhow::Result<()> {
        let output_items = vec![
            ResponseOutputItemInfo::Message {
                id: Some("msg_1".to_string()),
                status: Some("completed".to_string()),
                role: "assistant".to_string(),
                content: vec![ContentBlockPart::ToolCall {
                    id: "call_1".to_string(),
                    name: "shell".to_string(),
                    arguments: "{}".to_string(),
                }],
            },
            ResponseOutputItemInfo::FunctionCall {
                id: None,
                status: Some("completed".to_string()),
                call_id: Some("call_\u{E0041}1".to_string()),
                name: "shell".to_string(),
                arguments: "{}".to_string(),
            },
        ];

        let error = process_streaming_output_items(output_items, false).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate ID after Unicode tag sanitization"),
            "unexpected error: {error}"
        );

        Ok(())
    }

    #[test]
    fn test_streaming_output_items_reject_sanitized_tool_argument_key_collisions(
    ) -> anyhow::Result<()> {
        let colliding_arguments = json!({
            "command": "visible",
            "comm\u{E0041}and": "hidden",
        });
        let output_items = [
            ResponseOutputItemInfo::Message {
                id: Some("msg_1".to_string()),
                status: Some("completed".to_string()),
                role: "assistant".to_string(),
                content: vec![ContentBlockPart::ToolCall {
                    id: "call_1".to_string(),
                    name: "shell".to_string(),
                    arguments: serde_json::to_string(&colliding_arguments)?,
                }],
            },
            ResponseOutputItemInfo::FunctionCall {
                id: None,
                status: Some("completed".to_string()),
                call_id: Some("call_2".to_string()),
                name: "shell".to_string(),
                arguments: serde_json::to_string(&colliding_arguments)?,
            },
        ];

        for output_item in output_items {
            let error = process_streaming_output_items(vec![output_item], false).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("duplicate key after Unicode tag sanitization"),
                "unexpected error: {error}"
            );
        }

        Ok(())
    }

    #[test]
    fn test_responses_request_sanitizes_replayed_function_call_names() {
        use crate::conversation::message::Message;

        let messages = vec![
            Message::assistant().with_tool_request(
                "call_agent",
                Ok(CallToolRequestParams::new("Crack Catcher")
                    .with_arguments(object!({"prompt": "verify the work"}))),
            ),
            Message::assistant().with_tool_request(
                "call_review_agent",
                Ok(CallToolRequestParams::new("@Review Agent")
                    .with_arguments(object!({"prompt": "check it"}))),
            ),
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

        assert_eq!(input[0]["type"], "function_call");
        assert_eq!(input[0]["call_id"], "call_agent");
        assert_eq!(input[0]["name"], "Crack_Catcher");

        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[1]["call_id"], "call_review_agent");
        assert_eq!(input[1]["name"], "_Review_Agent");
    }

    #[test]
    fn test_responses_request_limits_replayed_function_call_names() {
        use crate::conversation::message::Message;

        let messages = vec![Message::assistant().with_tool_request(
            "call_long_name",
            Ok(CallToolRequestParams::new("a".repeat(160))),
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
        let name = result["input"][0]["name"].as_str().unwrap();

        assert_eq!(name.len(), 128);
    }

    #[test]
    fn test_tool_request_error_emits_function_call_output() {
        use crate::conversation::message::Message;
        use rmcp::model::{ErrorCode, ErrorData};

        let messages = vec![Message::assistant().with_tool_request(
            "call_err1",
            Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: "invalid arguments".into(),
                data: None,
            }),
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

        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["type"], "function_call_output");
        assert_eq!(input[0]["call_id"], "call_err1");
        assert!(input[0]["output"]
            .as_str()
            .unwrap()
            .contains("invalid arguments"));

}
