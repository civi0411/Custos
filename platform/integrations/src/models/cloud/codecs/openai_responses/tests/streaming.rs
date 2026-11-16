use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::token_usage::ProviderUsage;
use futures::StreamExt;

    #[tokio::test]
    async fn test_responses_stream_ignores_keepalive_event() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"keepalive"}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hello"}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":3,"item_id":"msg_1","output_index":0,"content_index":0,"delta":" world"}"#.to_string(),
            r#"data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"gpt-5.2-pro","output":[],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14,"input_tokens_details":{"cached_tokens":6}}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text_parts = Vec::new();
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    if let MessageContentBlock::Text(text) = content {
                        text_parts.push(text.text.clone());
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(text_parts.concat(), "Hello world");
        let usage = usage.expect("usage should be present at completion");
        assert_eq!(usage.model, "gpt-5.2-pro");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(4));
        assert_eq!(usage.usage.total_tokens, Some(14));
        assert_eq!(usage.usage.cache_read_input_tokens, Some(6));
        assert_eq!(usage.usage.cache_write_input_tokens, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_ignores_sse_field_lines() -> anyhow::Result<()> {
        let lines = vec![
            "id:1".to_string(),
            "id".to_string(),
            "x-trace: abc123".to_string(),
            "retry: 100".to_string(),
            ": keepalive comment".to_string(),
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"qwen3.8-max","output":[]}}"#.to_string(),
            "event: response.output_text.delta".to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hello"}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":3,"item_id":"msg_1","output_index":0,"content_index":0,"delta":" world"}"#.to_string(),
            r#"data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"qwen3.8-max","output":[],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text_parts = Vec::new();
        while let Some(item) = messages.next().await {
            let (message, _) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    if let MessageContentBlock::Text(text) = content {
                        text_parts.push(text.text.clone());
                    }
                }
            }
        }

        assert_eq!(text_parts.concat(), "Hello world");
        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_completed_allows_missing_output() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hello"}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":3,"item_id":"msg_1","output_index":0,"content_index":0,"delta":" world"}"#.to_string(),
            r#"data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"gpt-5.2-pro","usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text_parts = Vec::new();
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    if let MessageContentBlock::Text(text) = content {
                        text_parts.push(text.text.clone());
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(text_parts.concat(), "Hello world");
        let usage = usage.expect("usage should be present at completion");
        assert_eq!(usage.model, "gpt-5.2-pro");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(4));
        assert_eq!(usage.usage.total_tokens, Some(14));

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_marks_output_token_limit_when_incomplete() -> anyhow::Result<()>
    {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Partial response"}"#.to_string(),
            r#"data: {"type":"response.incomplete","sequence_number":3,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"incomplete","model":"gpt-5.2-pro","output":[],"incomplete_details":{"reason":"max_output_tokens"},"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text_parts = Vec::new();
        let mut output_token_limit_markers = Vec::new();
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                if msg.metadata.output_token_limit_reached {
                    output_token_limit_markers.push((msg.id.clone(), msg.content.is_empty()));
                }
                for content in msg.content {
                    if let MessageContentBlock::Text(text) = content {
                        text_parts.push(text.text);
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(text_parts.concat(), "Partial response");
        assert_eq!(
            output_token_limit_markers,
            vec![(Some("resp_1".to_string()), true)]
        );
        let usage = usage.expect("usage should be present when the response is incomplete");
        assert_eq!(usage.model, "gpt-5.2-pro");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(5));
        assert_eq!(usage.usage.total_tokens, Some(15));
        assert_eq!(
            usage.finish_reasons.as_deref(),
            Some(&["max_output_tokens".to_string()][..])
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_drops_incomplete_function_calls() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"response.incomplete","sequence_number":2,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"incomplete","model":"gpt-5.2-pro","output":[{"type":"function_call","id":"fc_complete","status":"completed","call_id":"call_complete","name":"read_file","arguments":"{\"path\":\"README.md\"}"},{"type":"function_call","id":"fc_partial","status":"in_progress","call_id":"call_partial","name":"shell","arguments":"{\"command\":\"echo"}],"incomplete_details":{"reason":"max_output_tokens"},"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut tool_request_ids = Vec::new();
        let mut output_token_limit_reached = false;
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                output_token_limit_reached |= msg.metadata.output_token_limit_reached;
                for content in msg.content {
                    if let MessageContentBlock::ToolRequest(request) = content {
                        tool_request_ids.push(request.id);
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(tool_request_ids, vec!["call_complete"]);
        assert!(output_token_limit_reached);
        let usage = usage.expect("usage should be present when the response is incomplete");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(5));
        assert_eq!(usage.usage.total_tokens, Some(15));

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_allows_message_output_without_id_status() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hello"}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":3,"item_id":"msg_1","output_index":0,"content_index":0,"delta":" world"}"#.to_string(),
            r#"data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"gpt-5.2-pro","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hello world"}]}],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut text_parts = Vec::new();
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    if let MessageContentBlock::Text(text) = content {
                        text_parts.push(text.text.clone());
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(text_parts.concat(), "Hello world");
        let usage = usage.expect("usage should be present at completion");
        assert_eq!(usage.model, "gpt-5.2-pro");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(4));
        assert_eq!(usage.usage.total_tokens, Some(14));

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_allows_function_call_without_id_status() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5.2-pro","output":[]}}"#.to_string(),
            r#"data: {"type":"response.completed","sequence_number":2,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"gpt-5.2-pro","output":[{"type":"reasoning","summary":[]},{"type":"function_call","call_id":"call_abc","name":"shell","arguments":"{\"command\":\"pwd\"}"}],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut tool_request_id = None;
        let mut usage: Option<ProviderUsage> = None;

        while let Some(item) = messages.next().await {
            let (message, maybe_usage) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    if let MessageContentBlock::ToolRequest(request) = content {
                        tool_request_id = Some(request.id);
                    }
                }
            }
            if let Some(final_usage) = maybe_usage {
                usage = Some(final_usage);
            }
        }

        assert_eq!(tool_request_id.as_deref(), Some("call_abc"));
        let usage = usage.expect("usage should be present at completion");
        assert_eq!(usage.model, "gpt-5.2-pro");
        assert_eq!(usage.usage.total_tokens, Some(14));

        Ok(())
    }

    #[test]
    fn test_responses_api_to_message_captures_reasoning_summary() -> anyhow::Result<()> {
        let response: ResponsesApiResponse = serde_json::from_value(serde_json::json!({
            "id": "resp_1",
            "object": "response",
            "created_at": 1737368310,
            "status": "completed",
            "model": "gpt-5",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_1",
                    "summary": [
                        { "type": "summary_text", "text": "Thinking\u{E0041} about the question..." },
                        { "type": "summary_text", "text": "The answer is\u{E0042} straightforward." }
                    ]
                },
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        { "type": "output_text", "text": "The capital of France is Paris." }
                    ]
                }
            ]
        }))?;

        let message = responses_api_to_message(&response)?;

        let thinking = message.content.iter().find_map(|c| c.as_thinking());
        assert!(thinking.is_some(), "should contain thinking content");
        assert_eq!(
            thinking.unwrap().thinking,
            "Thinking about the question...\nThe answer is straightforward."
        );

        let text = message.content.iter().find_map(|c| c.as_text());
        assert_eq!(text, Some("The capital of France is Paris."));

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_captures_reasoning_summary() -> anyhow::Result<()> {
        let reasoning_item = serde_json::json!({
            "type": "reasoning",
            "id": "rs_1",
            "summary": [
                { "type": "summary_text", "text": "Let me\u{E0041} think step by step." }
            ]
        });
        let message_item = serde_json::json!({
            "type": "message",
            "id": "msg_1",
            "status": "completed",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "Paris." }]
        });

        let lines = vec![
            r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":1737368310,"status":"in_progress","model":"gpt-5","output":[]}}"#.to_string(),
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":1,"content_index":0,"delta":"Paris."}"#.to_string(),
            format!(
                r#"data: {{"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{}}}"#,
                serde_json::to_string(&reasoning_item)?
            ),
            format!(
                r#"data: {{"type":"response.output_item.done","sequence_number":4,"output_index":1,"item":{}}}"#,
                serde_json::to_string(&message_item)?
            ),
            format!(
                r#"data: {{"type":"response.completed","sequence_number":5,"response":{{"id":"resp_1","object":"response","created_at":1737368310,"status":"completed","model":"gpt-5","output":[{},{}],"usage":{{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}}}}"#,
                serde_json::to_string(&reasoning_item)?,
                serde_json::to_string(&message_item)?
            ),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let mut thinking_parts = Vec::new();
        let mut text_parts = Vec::new();

        while let Some(item) = messages.next().await {
            let (message, _) = item?;
            if let Some(msg) = message {
                for content in msg.content {
                    match &content {
                        MessageContentBlock::Thinking(t) => thinking_parts.push(t.thinking.clone()),
                        MessageContentBlock::Text(t) => text_parts.push(t.text.clone()),
                        _ => {}
                    }
                }
            }
        }

        assert!(
            !thinking_parts.is_empty(),
            "should capture thinking from stream"
        );
        assert_eq!(thinking_parts.join(""), "Let me think step by step.");
        assert!(text_parts.concat().contains("Paris."));

        Ok(())
    }

    #[tokio::test]
    async fn test_responses_stream_error_event_still_returns_error() -> anyhow::Result<()> {
        let lines = vec![
            r#"data: {"type":"error","error":{"message":"boom"}}"#.to_string(),
            "data: [DONE]".to_string(),
        ];

        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = responses_api_to_streaming_message(response_stream);
        futures::pin_mut!(messages);

        let first = messages
            .next()
            .await
            .expect("stream should emit an error item");

        assert!(first.is_err());
        assert!(first
            .expect_err("expected error")
            .to_string()
            .contains("Responses API error"));

        Ok(())
    }

