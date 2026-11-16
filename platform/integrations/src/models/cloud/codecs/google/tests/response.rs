use super::super::*;
use super::helpers::*;
use crate::conversation::message::{Message, MessageContentBlock};
use rmcp::model::{ContentBlock, ErrorCode, ErrorData, Role};
use serde_json::json;

    #[test]
    fn test_response_to_message_with_no_candidates() {
        let response = json!({});
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert!(message.content.is_empty());
    }

    #[test]
    fn test_response_to_message_with_text_part() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello, world!"
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        if let MessageContentBlock::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_response_to_message_with_invalid_function_name() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "invalid name!",
                            "args": {}
                        }
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        if let Err(error) = &message.content[0].as_tool_request().unwrap().tool_call {
            assert!(matches!(
                error,
                ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: _,
                    data: None,
                }
            ));
        } else {
            panic!("Expected tool request error");
        }
    }

    #[test]
    fn test_response_to_message_with_valid_function_call() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "id": "call_123",
                            "name": "valid_name",
                            "args": {
                                "param": "value"
                            }
                        }
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        assert_eq!(message.content[0].as_tool_request().unwrap().id, "call_123");
        if let Ok(tool_call) = &message.content[0].as_tool_request().unwrap().tool_call {
            assert_eq!(tool_call.name, "valid_name");
            assert_eq!(
                tool_call
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("param"))
                    .and_then(|v| v.as_str()),
                Some("value")
            );
        } else {
            panic!("Expected valid tool request");
        }
    }

    #[test]
    fn test_response_to_message_with_empty_content() {
        let tool_result: Vec<ContentBlock> = Vec::new();

        let messages = vec![set_up_tool_response_message("response_id", tool_result)];
        let payload = format_messages(&messages, false);

        let expected_payload = vec![json!({
            "role": "model",
            "parts": [
                {
                    "functionResponse": {
                        "id": "response_id",
                        "name": "response_id",
                        "response": {
                            "content": {
                                "text": "Tool call is done."
                            }
                        }
                    }
                }
            ]
        })];

        assert_eq!(payload, expected_payload);
    }

    #[test]

    fn test_thought_signature_roundtrip() {
        const SIG: &str = "thought_sig_abc";

        let response_with_tools = google_response(vec![
            json!({"text": "Let me think...", "thought": true, "thoughtSignature": SIG}),
            json!({"functionCall": {"name": "shell", "args": {"cmd": "ls"}}, "thoughtSignature": SIG}),
            json!({"functionCall": {"name": "read", "args": {}}}),
        ]);

        let native = response_to_message(response_with_tools).unwrap();
        assert_eq!(native.content.len(), 3, "Expected thinking + 2 tool calls");

        let thinking = native.content[0]
            .as_thinking()
            .expect("Text with function calls should be Thinking");
        assert_eq!(thinking.signature, SIG);

        let req1 = native.content[1]
            .as_tool_request()
            .expect("Second part should be ToolRequest");
        let req2 = native.content[2]
            .as_tool_request()
            .expect("Third part should be ToolRequest");
        assert_eq!(get_thought_signature(&req1.metadata), Some(SIG));
        assert_eq!(
            get_thought_signature(&req2.metadata),
            Some(SIG),
            "Should inherit"
        );

        let mut tool_response = Message::user();
        tool_response.add_tool_response_with_metadata(
            req1.id.clone(),
            Ok(tool_result("output")),
            req1.metadata.as_ref(),
        );
        let user_prompt = set_up_text_message("List files", Role::User);
        let google_out = format_messages(
            &[user_prompt.clone(), native.clone(), tool_response.clone()],
            false,
        );
        assert_eq!(google_out[1]["parts"][0]["thoughtSignature"], SIG);
        assert_eq!(google_out[2]["parts"][0]["thoughtSignature"], SIG);

        let second_assistant = response_to_message(google_response(vec![json!({
            "functionCall": {"name": "echo", "args": {}},
            "thoughtSignature": "sig_456"
        })]))
        .unwrap();
        let google_multi = format_messages(
            &[user_prompt, native, tool_response, second_assistant],
            false,
        );
        assert_eq!(google_multi[1]["parts"][0]["thoughtSignature"], SIG);
        assert_eq!(google_multi[2]["parts"][0]["thoughtSignature"], SIG);
        assert_eq!(google_multi[3]["parts"][0]["thoughtSignature"], "sig_456");

        let final_response_with_sig =
            google_response(vec![json!({"text": "Done!", "thoughtSignature": SIG})]);
        let final_native_with_sig = response_to_message(final_response_with_sig).unwrap();
        assert!(
            final_native_with_sig.content[0].as_text().is_some(),
            "Text with signature but no function calls should be regular text (final response)"
        );

        let final_response_no_sig = google_response(vec![json!({"text": "Done!"})]);
        let final_native_no_sig = response_to_message(final_response_no_sig).unwrap();
        assert!(
            final_native_no_sig.content[0].as_text().is_some(),
            "Text without signature is regular text"
        );
    }

    #[test]
    fn test_thought_without_signature_maps_to_thinking() {
        let response = google_response(vec![json!({
            "text": "Working through options...",
            "thought": true
        })]);
        let native = response_to_message(response).unwrap();
        assert_eq!(native.content.len(), 1);
        assert!(native.content[0].as_thinking().is_some());
    }

    #[test]
    fn test_format_messages_omits_messages_with_empty_parts() {
        let user_prompt = set_up_text_message("hello", Role::User);
        let thinking_only =
            Message::assistant().with_thinking("internal".to_string(), "sig_123".to_string());
        let reasoning_only = response_to_message(google_response(vec![json!({
            "text": "deliberating",
            "thought": true
        })]))
        .unwrap();

        let formatted = format_messages(&[user_prompt, thinking_only, reasoning_only], false);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["role"], "user");
        assert_eq!(formatted[0]["parts"][0]["text"], "hello");
    }

    #[test]
    fn test_active_loop_injects_synthetic_signature_for_first_model_tool_call() {
        let user_prompt = set_up_text_message("Find a restaurant", Role::User);
        let assistant_tool = response_to_message(google_response(vec![json!({
            "functionCall": {"name": "find_restaurant", "args": {"cuisine": "italian"}}
        })]))
        .unwrap();

        let formatted = format_messages(&[user_prompt, assistant_tool], false);
        assert_eq!(
            formatted[1]["parts"][0][THOUGHT_SIGNATURE_KEY],
            SYNTHETIC_THOUGHT_SIGNATURE
        );
    }

