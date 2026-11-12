use super::super::*;
use crate::conversation::message::MessageContentBlock;
use futures::StreamExt;

    #[test]
    fn test_responses_api_to_message_sanitizes_unicode_tags() {
        let response: ResponsesApiResponse = serde_json::from_value(serde_json::json!({
            "id": "resp_1",
            "object": "response",
            "created_at": 0,
            "status": "completed",
            "model": "gpt-5.5",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "visible\u{E0041}text"},
                    {"type": "refusal", "refusal": "cannot\u{E0042}help"}
                ]
            }]
        }))
        .unwrap();

        let message = responses_api_to_message(&response).unwrap();
        let text = message
            .content
            .iter()
            .filter_map(MessageContentBlock::as_text)
            .collect::<Vec<_>>();

        assert_eq!(text, vec!["visibletext", "cannothelp"]);
    }

    #[test]
    fn test_streaming_output_items_sanitize_unicode_tags() {
        let item: ResponseOutputItemInfo = serde_json::from_value(serde_json::json!({
            "type": "message",
            "id": "msg_1",
            "status": "completed",
            "role": "assistant",
            "content": [
                {"type": "output_text", "text": "visible\u{E0041}text"},
                {"type": "refusal", "refusal": "cannot\u{E0042}help"}
            ]
        }))
        .unwrap();

        let content = process_streaming_output_items(vec![item], false).unwrap();
        let text = content
            .iter()
            .filter_map(MessageContentBlock::as_text)
            .collect::<Vec<_>>();

        assert_eq!(text, vec!["visibletext", "cannothelp"]);
    }

    #[tokio::test]
    async fn test_streaming_deltas_sanitize_unicode_tags() -> anyhow::Result<()> {
        let lines = vec![
            format!(
                "data: {}",
                serde_json::json!({
                    "type": "response.created",
                    "sequence_number": 1,
                    "response": {
                        "id": "resp_1",
                        "object": "response",
                        "created_at": 0,
                        "status": "in_progress",
                        "model": "gpt-5.5",
                        "output": []
                    }
                })
            ),
            format!(
                "data: {}",
                serde_json::json!({
                    "type": "response.output_text.delta",
                    "sequence_number": 2,
                    "item_id": "msg_1",
                    "output_index": 0,
                    "content_index": 0,
                    "delta": "visible\u{E0041}te"
                })
            ),
            format!(
                "data: {}",
                serde_json::json!({
                    "type": "response.output_text.delta",
                    "sequence_number": 3,
                    "item_id": "msg_1",
                    "output_index": 0,
                    "content_index": 0,
                    "delta": "xt e"
                })
            ),
            format!(
                "data: {}",
                serde_json::json!({
                    "type": "response.output_text.delta",
                    "sequence_number": 4,
                    "item_id": "msg_1",
                    "output_index": 0,
                    "content_index": 0,
                    "delta": "\u{301}"
                })
            ),
            format!(
                "data: {}",
                serde_json::json!({
                    "type": "response.refusal.delta",
                    "sequence_number": 5,
                    "item_id": "msg_1",
                    "output_index": 0,
                    "content_index": 1,
                    "delta": "cannot\u{E0042}help"
                })
            ),
            "data: [DONE]".to_string(),
        ];
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text = Vec::new();
        while let Some(item) = messages.next().await {
            if let Some(message) = item?.0 {
                text.extend(
                    message
                        .content
                        .iter()
                        .filter_map(MessageContentBlock::as_text)
                        .map(str::to_owned),
                );
            }
        }

        assert_eq!(text.concat(), "visibletext e\u{301}cannothelp");
        Ok(())
    }

    #[test]
    fn test_refusal_content_block_deserializes_in_non_streaming_response() {
        let json = r#"{
            "id": "resp_1",
            "object": "response",
            "created_at": 0,
            "status": "completed",
            "model": "gpt-5.5",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "status": "completed",
                "role": "assistant",
                "content": [{"type": "refusal", "refusal": "I cannot help with that request."}]
            }]
        }"#;

        let response: ResponsesApiResponse = serde_json::from_str(json).unwrap();
        let message = responses_api_to_message(&response).unwrap();
        assert_eq!(message.content.len(), 1);
        if let MessageContentBlock::Text(t) = &message.content[0] {
            assert_eq!(t.text, "I cannot help with that request.");
        } else {
            panic!("expected text content from refusal");
        }
    }

    #[test]
    fn test_refusal_content_part_deserializes_in_streaming_output() -> anyhow::Result<()> {
        let json = r#"{
            "type": "message",
            "id": "msg_1",
            "status": "completed",
            "role": "assistant",
            "content": [{"type": "refusal", "refusal": "I'm unable to assist."}]
        }"#;

        let item: ResponseOutputItemInfo = serde_json::from_str(json).unwrap();
        let content = process_streaming_output_items(vec![item], false)?;
        assert_eq!(content.len(), 1);
        if let MessageContentBlock::Text(t) = &content[0] {
            assert_eq!(t.text, "I'm unable to assist.");
        } else {
            panic!("expected text content from refusal");
        }

        Ok(())
    }

    #[test]
    fn test_refusal_delta_stream_event_deserializes() {
        let json = r#"{"type":"response.refusal.delta","sequence_number":5,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"I cannot"}"#;

        let event: ResponsesStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            ResponsesStreamEvent::RefusalDelta { delta, .. } => {
                assert_eq!(delta, "I cannot");
            }
            _ => panic!("expected RefusalDelta event"),
        }
    }

    #[test]
    fn test_streamed_refusal_not_duplicated_in_output_items() -> anyhow::Result<()> {
        let output_items = vec![ResponseOutputItemInfo::Message {
            id: Some("msg_1".to_string()),
            status: Some("completed".to_string()),
            role: "assistant".to_string(),
            content: vec![ContentBlockPart::Refusal {
                refusal: "I cannot help with that.".to_string(),
            }],
        }];

        let content = process_streaming_output_items(output_items.clone(), true)?;
        assert!(
            content.is_empty(),
            "refusal should be suppressed when already streamed"
        );

        let content = process_streaming_output_items(output_items, false)?;
        assert_eq!(
            content.len(),
            1,
            "refusal should appear in non-streaming path"
        );

        Ok(())
    }

    #[test]
    fn test_reasoning_text_content_part_deserializes() {
        let json = r#"{"type":"response.content_part.added","content_index":0,"item_id":"64cda83e-03b0-4e61-ab77-30d06ffe7f23","output_index":0,"part":{"type":"reasoning_text","text":"thinking..."},"sequence_number":3}"#;

        let event: ResponsesStreamEvent = serde_json::from_str(json).unwrap();
        match event {
            ResponsesStreamEvent::ContentBlockPartAdded { part, .. } => match part {
                ContentBlockPart::ReasoningText { text } => {
                    assert_eq!(text, "thinking...");
                }
                _ => panic!("expected ReasoningText part"),
            },
            _ => panic!("expected ContentBlockPartAdded event"),
        }
    }

    #[test]
    fn test_reasoning_text_surfaces_as_thinking_block() -> anyhow::Result<()> {
        let output_items = vec![ResponseOutputItemInfo::Message {
            id: Some("msg_1".to_string()),
            status: Some("completed".to_string()),
            role: "assistant".to_string(),
            content: vec![ContentBlockPart::ReasoningText {
                text: "thinking...".to_string(),
            }],
        }];

        let content = process_streaming_output_items(output_items, true)?;
        assert_eq!(content.len(), 1, "reasoning text should be kept");
        assert!(
            matches!(content[0], MessageContentBlock::Thinking(_)),
            "reasoning text should become a thinking block"
        );

        Ok(())
    }

