use super::super::*;
use super::helpers::*;
use crate::conversation::message::{Message, MessageContentBlock};
use rmcp::model::{object, CallToolRequestParams, ContentBlock, ResourceContents, Role};
use serde_json::json;

    #[test]
    fn test_message_to_google_spec_text_message() {
        let messages = vec![
            set_up_text_message("Hello", Role::User),
            set_up_text_message("World", Role::Assistant),
        ];
        let payload = format_messages(&messages, false);
        assert_eq!(payload.len(), 2);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["text"], "Hello");
        assert_eq!(payload[1]["role"], "model");
        assert_eq!(payload[1]["parts"][0]["text"], "World");
    }

    #[test]
    fn test_message_to_google_spec_image_message() {
        use rmcp::model::ImageContent;

        let image = ImageContent::new("base64encodeddata", "image/png");
        let messages = vec![Message::new(
            Role::User,
            0,
            vec![
                MessageContentBlock::text("What is in this image?".to_string()),
                MessageContentBlock::Image(image),
            ],
        )];
        let payload = format_messages(&messages, false);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["text"], "What is in this image?");
        assert_eq!(
            payload[0]["parts"][1]["inline_data"]["mime_type"],
            "image/png"
        );
        assert_eq!(
            payload[0]["parts"][1]["inline_data"]["data"],
            "base64encodeddata"
        );
    }

    #[test]
    fn test_message_to_google_spec_document_only_message() {
        let messages = vec![Message::new(
            Role::User,
            0,
            vec![MessageContentBlock::document(
                "base64pdfdata".to_string(),
                "application/pdf".to_string(),
                Some("report.pdf".to_string()),
            )],
        )];
        let payload = format_messages(&messages, false);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(
            payload[0]["parts"][0]["inline_data"]["mime_type"],
            "application/pdf"
        );
        assert_eq!(
            payload[0]["parts"][0]["inline_data"]["data"],
            "base64pdfdata"
        );
    }

    #[test]
    fn test_message_to_google_spec_tool_request_message() {
        let arguments = json!({
            "param1": "value1"
        });
        let messages = vec![
            set_up_tool_request_message(
                "id",
                CallToolRequestParams::new("tool_name").with_arguments(object(arguments.clone())),
            ),
            set_up_action_required_message(
                "id2",
                CallToolRequestParams::new("tool_name_2").with_arguments(object(arguments.clone())),
            ),
        ];
        let payload = format_messages(&messages, false);
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["functionCall"]["id"], "id");
        assert_eq!(payload[0]["parts"][0]["functionCall"]["args"], arguments);
    }

    #[test]
    fn test_message_to_google_spec_tool_result_message() {
        let tool_result: Vec<ContentBlock> = vec![ContentBlock::text("Hello")];
        let messages = vec![set_up_tool_response_message("response_id", tool_result)];
        let payload = format_messages(&messages, false);
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "model");
        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["name"],
            "response_id"
        );
        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["id"],
            "response_id"
        );
        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["response"]["content"]["text"],
            "Hello"
        );
    }

    #[test]
    fn test_message_to_google_spec_sanitizes_resource_tool_response() {
        let messages = vec![set_up_tool_response_message(
            "response_id",
            vec![ContentBlock::embedded_text(
                "file:///result.txt",
                "visible\u{E0041}text",
            )],
        )];

        let payload = format_messages(&messages, false);

        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["response"]["content"]["text"],
            "visibletext"
        );
    }

    #[test]
    fn test_function_response_matches_function_call() {
        let messages = vec![
            set_up_tool_request_message("call_123", CallToolRequestParams::new("read_file")),
            set_up_tool_response_message("call_123", vec![ContentBlock::text("contents")]),
        ];

        let payload = format_messages(&messages, false);

        assert_eq!(
            payload[1]["parts"][0]["functionResponse"],
            json!({
                "id": "call_123",
                "name": "read_file",
                "response": {"content": {"text": "contents"}}
            })
        );
    }

    #[test]
    fn test_reused_tool_call_id_keeps_each_response_name() {
        let messages = vec![
            set_up_tool_request_message(
                "call_3407",
                CallToolRequestParams::new("download_document"),
            ),
            set_up_tool_response_message("call_3407", vec![ContentBlock::text("the document")]),
            set_up_tool_request_message("call_3407", CallToolRequestParams::new("shell")),
            set_up_tool_response_message("call_3407", vec![ContentBlock::text("shell output")]),
        ];

        let payload = format_messages(&messages, false);

        assert_eq!(
            payload[1]["parts"][0]["functionResponse"],
            json!({
                "id": "call_3407",
                "name": "download_document",
                "response": {"content": {"text": "the document"}}
            })
        );
        assert_eq!(
            payload[3]["parts"][0]["functionResponse"],
            json!({
                "id": "call_3407",
                "name": "shell",
                "response": {"content": {"text": "shell output"}}
            })
        );
    }

    #[test]
    fn test_image_tool_result_is_nested_in_function_response() {
        let messages = vec![
            set_up_tool_request_message("call_123", CallToolRequestParams::new("screenshot")),
            set_up_tool_response_message(
                "call_123",
                vec![
                    ContentBlock::text("Screenshot captured"),
                    ContentBlock::image("base64encodeddata", "image/png"),
                ],
            ),
        ];

        let payload = format_messages(&messages, true);

        assert_eq!(payload[1]["parts"].as_array().unwrap().len(), 1);
        assert_eq!(
            payload[1]["parts"][0]["functionResponse"],
            json!({
                "id": "call_123",
                "name": "screenshot",
                "response": {"content": {"text": "Screenshot captured"}},
                "parts": [{
                    "inlineData": {
                        "mimeType": "image/png",
                        "data": "base64encodeddata"
                    }
                }]
            })
        );
    }

    #[test]
    fn test_blob_resource_tool_result_is_forwarded_as_media() {
        let blob = ContentBlock::resource(ResourceContents::BlobResourceContents {
            uri: "file:///shot.png".to_string(),
            mime_type: Some("image/png".to_string()),
            blob: "aGVsbG8=".to_string(),
            meta: None,
        });
        let messages = vec![
            set_up_tool_request_message("call_123", CallToolRequestParams::new("screenshot")),
            set_up_tool_response_message("call_123", vec![blob]),
        ];

        let payload = format_messages(&messages, true);

        assert_eq!(
            payload[1]["parts"][0]["functionResponse"]["parts"],
            json!([{"inlineData": {"mimeType": "image/png", "data": "aGVsbG8="}}])
        );
    }

    #[test]
    fn test_message_to_google_spec_tool_result_multiple_texts() {
        let tool_result: Vec<ContentBlock> = vec![
            ContentBlock::text("Hello"),
            ContentBlock::text("World"),
            ContentBlock::embedded_text("test_uri", "This is a test."),
        ];

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
                                "text": "Hello\nWorld\nThis is a test."
                            }
                        }
                    }
                }
            ]
        })];

        assert_eq!(payload, expected_payload);
    }
