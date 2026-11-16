use super::super::*;
use crate::conversation::message::{Message, MessageContentBlock};
use crate::model::ModelConfig;
use rmcp::model::CallToolRequestParams;
use rmcp::object;
use serde_json::json;

    #[test]
    fn test_history_preserves_chronological_order() {
        let model_config = ModelConfig {
            model_name: "gpt-5.2-codex".to_string(),
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

        let messages = vec![
            Message::assistant()
                .with_text("I'll create that file.")
                .with_tool_request(
                    "call_1",
                    Ok(CallToolRequestParams::new("shell")
                        .with_arguments(object!({"command": "echo hello"}))),
                ),
            Message::assistant()
                .with_text("Now let me verify.")
                .with_tool_request(
                    "call_2",
                    Ok(CallToolRequestParams::new("shell")
                        .with_arguments(object!({"command": "cat file.txt"}))),
                ),
        ];

        let result = create_responses_request(&model_config, "", &messages, &[]).unwrap();
        let input = result["input"].as_array().unwrap();

        let types: Vec<&str> = input
            .iter()
            .map(|item| item["type"].as_str().unwrap())
            .collect();

        assert_eq!(
            types,
            vec!["message", "function_call", "message", "function_call"]
        );
    }

    #[test]
    fn test_responses_api_to_message_uses_call_id_for_tool_request_id() {
        let response = ResponsesApiResponse {
            id: "resp_1".to_string(),
            object: "response".to_string(),
            created_at: 0,
            status: "completed".to_string(),
            model: "gpt-5.3-codex".to_string(),
            output: vec![ResponseOutputItem::FunctionCall {
                id: Some("fc_123".to_string()),
                status: Some("completed".to_string()),
                call_id: Some("call_abc".to_string()),
                name: "test__get_person_zip_code".to_string(),
                arguments: r#"{"name":"Alice Burns"}"#.to_string(),
            }],
            reasoning: None,
            usage: None,
        };

        let message = responses_api_to_message(&response).unwrap();
        assert_eq!(message.content.len(), 1);
        let MessageContentBlock::ToolRequest(tool_request) = &message.content[0] else {
            panic!("expected tool request content");
        };
        assert_eq!(tool_request.id, "call_abc");
    }

    #[test]
    fn test_responses_api_to_message_sanitizes_tool_arguments() {
        let response = ResponsesApiResponse {
            id: "resp_1".to_string(),
            object: "response".to_string(),
            created_at: 0,
            status: "completed".to_string(),
            model: "gpt-5.3-codex".to_string(),
            output: vec![
                ResponseOutputItem::Message {
                    id: Some("msg_1".to_string()),
                    status: Some("completed".to_string()),
                    role: "assistant".to_string(),
                    content: vec![ResponseContentBlock::ToolCall {
                        id: "call_\u{E0041}1".to_string(),
                        name: "sh\u{E0041}ell".to_string(),
                        input: json!({"prompt": "visible e\u{301}\u{E0041}text"}),
                    }],
                },
                ResponseOutputItem::FunctionCall {
                    id: None,
                    status: Some("completed".to_string()),
                    call_id: Some("call_\u{E0042}2".to_string()),
                    name: "sh\u{E0042}ell".to_string(),
                    arguments: serde_json::to_string(
                        &json!({"prompt": "visible e\u{301}\u{E0042}text"}),
                    )
                    .unwrap(),
                },
            ],
            reasoning: None,
            usage: None,
        };

        let message = responses_api_to_message(&response).unwrap();
        for (content, expected_id) in message.content.into_iter().zip(["call_1", "call_2"]) {
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
    }

    #[test]
    fn test_responses_api_to_message_rejects_sanitized_tool_request_id_collisions() {
        let response = ResponsesApiResponse {
            id: "resp_1".to_string(),
            object: "response".to_string(),
            created_at: 0,
            status: "completed".to_string(),
            model: "gpt-5.3-codex".to_string(),
            output: vec![
                ResponseOutputItem::Message {
                    id: Some("msg_1".to_string()),
                    status: Some("completed".to_string()),
                    role: "assistant".to_string(),
                    content: vec![ResponseContentBlock::ToolCall {
                        id: "call_1".to_string(),
                        name: "shell".to_string(),
                        input: json!({}),
                    }],
                },
                ResponseOutputItem::FunctionCall {
                    id: None,
                    status: Some("completed".to_string()),
                    call_id: Some("call_\u{E0041}1".to_string()),
                    name: "shell".to_string(),
                    arguments: "{}".to_string(),
                },
            ],
            reasoning: None,
            usage: None,
        };

        let error = responses_api_to_message(&response).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate ID after Unicode tag sanitization"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_responses_api_to_message_rejects_sanitized_tool_argument_key_collisions() {
        let colliding_arguments = json!({
            "command": "visible",
            "comm\u{E0041}and": "hidden",
        });
        let responses = [
            ResponsesApiResponse {
                id: "resp_1".to_string(),
                object: "response".to_string(),
                created_at: 0,
                status: "completed".to_string(),
                model: "gpt-5.3-codex".to_string(),
                output: vec![ResponseOutputItem::Message {
                    id: Some("msg_1".to_string()),
                    status: Some("completed".to_string()),
                    role: "assistant".to_string(),
                    content: vec![ResponseContentBlock::ToolCall {
                        id: "call_1".to_string(),
                        name: "shell".to_string(),
                        input: colliding_arguments.clone(),
                    }],
                }],
                reasoning: None,
                usage: None,
            },
            ResponsesApiResponse {
                id: "resp_2".to_string(),
                object: "response".to_string(),
                created_at: 0,
                status: "completed".to_string(),
                model: "gpt-5.3-codex".to_string(),
                output: vec![ResponseOutputItem::FunctionCall {
                    id: None,
                    status: Some("completed".to_string()),
                    call_id: Some("call_2".to_string()),
                    name: "shell".to_string(),
                    arguments: serde_json::to_string(&colliding_arguments).unwrap(),
                }],
                reasoning: None,
                usage: None,
            },
        ];

        for response in responses {
            let error = responses_api_to_message(&response).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("duplicate key after Unicode tag sanitization"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn test_deserialize_reasoning_info_with_null_effort() {
        let json = r#"{"effort": null}"#;
        let info: ResponseReasoningInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.effort, None);
        assert_eq!(info.summary, None);
    }

    #[test]
    fn test_deserialize_reasoning_info_with_effort() {
        let json = r#"{"effort": "high", "summary": "Thought deeply"}"#;
        let info: ResponseReasoningInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.effort.as_deref(), Some("high"));
        assert_eq!(info.summary.as_deref(), Some("Thought deeply"));
    }

