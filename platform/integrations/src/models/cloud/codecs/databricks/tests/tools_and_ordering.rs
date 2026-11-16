use super::super::*;
use crate::conversation::message::Message;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, Tool};
use rmcp::object;
use serde_json::{json, Value};

    #[test]
    fn test_format_messages_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that tool calls with None arguments are formatted as "{}" string
        let message = Message::assistant()
            .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool")));

        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["role"], "assistant");
        assert!(spec_array[0]["tool_calls"].is_array());

        let tool_call = &spec_array[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        // This should be the string "{}", not null
        assert_eq!(tool_call["function"]["arguments"], "{}");

        Ok(())
    }

    #[test]
    fn format_messages_post_parse_error_history_is_wellformed() -> anyhow::Result<()> {
        // An unparseable tool call (ToolRequest(Err)) paired with its error tool
        // response must not serialize as an orphan role:"tool" message.
        use rmcp::model::{ErrorCode, ErrorData};
        let err = ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "Tool arguments for id call_bad must be a JSON object".to_string(),
            None,
        );
        let request_msg = Message::assistant().with_tool_request("call_bad", Err(err.clone()));
        let mut final_resp = Message::user();
        final_resp.add_tool_response_with_metadata("call_bad", Err(err), None);
        let messages = vec![
            Message::user().with_text("do the thing"),
            request_msg,
            final_resp,
        ];

        let spec = serde_json::to_value(format_messages(
            &messages,
            &ImageFormat::OpenAi,
            None,
            false,
        ))?;
        let mut open = std::collections::HashSet::new();
        for m in spec.as_array().unwrap() {
            match m.get("role").and_then(|v| v.as_str()) {
                Some("assistant") => {
                    for tc in m
                        .get("tool_calls")
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                    {
                        if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                            open.insert(id.to_string());
                        }
                    }
                }
                Some("tool") => {
                    let id = m.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                    assert!(open.contains(id), "orphan role:tool message for id {id:?}");
                }
                _ => {}
            }
        }
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

        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["role"], "assistant");
        assert!(spec_array[0]["tool_calls"].is_array());

        let tool_call = &spec_array[0]["tool_calls"][0];
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
    fn test_is_claude_model() {
        assert!(is_claude_model("databricks-claude-sonnet-4"));
        assert!(is_claude_model("databricks-claude-sonnet-4.5"));
        assert!(is_claude_model("claude-sonnet-4"));
        assert!(is_claude_model("goose-claude-sonnet"));
        assert!(!is_claude_model("gpt-4o"));
        assert!(!is_claude_model("gemini-2-5-flash"));
        assert!(!is_claude_model("databricks-meta-llama-3-3-70b"));
    }

    #[test]
    fn test_format_messages_with_thought_signature_metadata() -> anyhow::Result<()> {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "thoughtSignature".to_string(),
            json!("sig_abc123_test_signature"),
        );

        let message = Message::assistant().with_tool_request_with_metadata(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool").with_arguments(object!({"param": "value"}))),
            Some(&metadata),
            None,
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        let tool_call = &spec_array[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        assert_eq!(tool_call["thoughtSignature"], "sig_abc123_test_signature");

        Ok(())
    }

    #[test]
    fn test_create_request_claude_has_cache_control() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "databricks-claude-sonnet-4".to_string(),
            context_limit: Some(200000),
            temperature: None,
            max_tokens: Some(8192),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
            Message::user().with_text("How are you?"),
        ];

        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let request = create_request(
            &model_config,
            "You are helpful",
            &messages,
            &[tool],
            &ImageFormat::OpenAi,
        )?;

        // Verify system message has cache_control
        let messages_arr = request["messages"].as_array().unwrap();
        let system_msg = &messages_arr[0];
        assert!(system_msg["content"].is_array());
        assert_eq!(
            system_msg["content"][0]["cache_control"]["type"],
            "ephemeral"
        );

        // Verify last tool has cache_control
        let tools = request["tools"].as_array().unwrap();
        assert_eq!(tools[0]["function"]["cache_control"]["type"], "ephemeral");

        Ok(())
    }

    #[test]
    fn test_create_request_non_claude_no_cache_control() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(128000),
            temperature: None,
            max_tokens: Some(4096),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let messages = vec![Message::user().with_text("Hello")];

        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let request = create_request(
            &model_config,
            "You are helpful",
            &messages,
            &[tool],
            &ImageFormat::OpenAi,
        )?;

        // Verify system message does NOT have cache_control (it's a plain string)
        let messages_arr = request["messages"].as_array().unwrap();
        let system_msg = &messages_arr[0];
        assert!(system_msg["content"].is_string());

        // Verify tool does NOT have cache_control
        let tools = request["tools"].as_array().unwrap();
        assert!(tools[0]["function"].get("cache_control").is_none());

        Ok(())
    }

    #[test]
    fn test_format_messages_with_multiple_metadata_fields() -> anyhow::Result<()> {
        let mut metadata = serde_json::Map::new();
        metadata.insert("thoughtSignature".to_string(), json!("sig_top_level"));
        metadata.insert(
            "extra_content".to_string(),
            json!({
                "google": {
                    "thought_signature": "sig_nested"
                }
            }),
        );
        metadata.insert("custom_field".to_string(), json!("custom_value"));

        let message = Message::assistant().with_tool_request_with_metadata(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool")),
            Some(&metadata),
            None,
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi, None, false);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        let tool_call = &spec_array[0]["tool_calls"][0];
        assert_eq!(tool_call["thoughtSignature"], "sig_top_level");
        assert_eq!(
            tool_call["extra_content"]["google"]["thought_signature"],
            "sig_nested"
        );
        assert_eq!(tool_call["custom_field"], "custom_value");

        Ok(())
    }

    #[test]
    fn test_parallel_tool_responses_with_images_are_consecutive() -> anyhow::Result<()> {
        // Regression: #7449 — parallel tool calls with images must keep tool messages consecutive.
        let messages = vec![
            Message::assistant()
                .with_tool_request("id1", Ok(CallToolRequestParams::new("tool_a")))
                .with_tool_request("id2", Ok(CallToolRequestParams::new("tool_b"))),
            Message::user()
                .with_tool_response(
                    "id1",
                    Ok(CallToolResult::success(vec![ContentBlock::image(
                        "base64data1".to_string(),
                        "image/png".to_string(),
                    )])),
                )
                .with_tool_response(
                    "id2",
                    Ok(CallToolResult::success(vec![ContentBlock::image(
                        "base64data2".to_string(),
                        "image/png".to_string(),
                    )])),
                ),
        ];

        let as_value =
            serde_json::to_value(format_messages(&messages, &ImageFormat::OpenAi, None, true))
                .unwrap();
        let spec = as_value.as_array().unwrap();
        let roles: Vec<&str> = spec.iter().map(|m| m["role"].as_str().unwrap()).collect();

        // Without the fix this was ["assistant", "tool", "user", "tool", "user"].
        assert_eq!(roles, vec!["assistant", "tool", "tool", "user", "user"]);

        Ok(())
    }

    #[test]
    fn test_mixed_tool_responses_image_and_text_ordering() -> anyhow::Result<()> {
        // Mixed case: only one tool response has an image.
        let messages = vec![
            Message::assistant()
                .with_tool_request("id1", Ok(CallToolRequestParams::new("tool_a")))
                .with_tool_request("id2", Ok(CallToolRequestParams::new("tool_b"))),
            Message::user()
                .with_tool_response(
                    "id1",
                    Ok(CallToolResult::success(vec![ContentBlock::text(
                        "text result",
                    )])),
                )
                .with_tool_response(
                    "id2",
                    Ok(CallToolResult::success(vec![ContentBlock::image(
                        "base64data".to_string(),
                        "image/png".to_string(),
                    )])),
                ),
        ];

        let as_value =
            serde_json::to_value(format_messages(&messages, &ImageFormat::OpenAi, None, true))
                .unwrap();
        let spec = as_value.as_array().unwrap();
        let roles: Vec<&str> = spec.iter().map(|m| m["role"].as_str().unwrap()).collect();

        assert_eq!(roles, vec!["assistant", "tool", "tool", "user"]);

        Ok(())

}
