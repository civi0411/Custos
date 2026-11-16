use crate::conversation::message::{
    ActionRequiredData, Message, MessageContentBlock, MessageErrorKind, MessageMetadata,
};
use crate::errors::ProviderError;
use rmcp::model::{
    Annotations, CallToolRequestParams, CallToolResult, ContentBlock, ElicitationAction,
    ImageContent, PromptMessage, ResourceContents, Role, TextContent,
};
use rmcp::object;
use serde_json::Value;

#[test]
fn provider_authentication_error_has_authentication_kind() {
    let message = Message::from_provider_error(&ProviderError::Authentication(
        "Authentication required".to_string(),
    ));

    assert_eq!(message.error_kind(), Some(MessageErrorKind::Authentication));
    assert!(message.is_user_visible());
    assert!(!message.is_agent_visible());
}

#[test]
fn test_elicitation_response_defaults_action_to_accept() {
    let action_required: ActionRequiredData = serde_json::from_value(serde_json::json!({
        "actionType": "elicitationResponse",
        "id": "request-123",
        "user_data": { "name": "custos-engine" }
    }))
    .unwrap();

    let ActionRequiredData::ElicitationResponse {
        id,
        user_data,
        action,
    } = action_required
    else {
        panic!("Expected elicitation response");
    };

    assert_eq!(id, "request-123");
    assert_eq!(user_data, serde_json::json!({ "name": "custos-engine" }));
    assert_eq!(action, ElicitationAction::Accept);
}

#[test]
fn test_agent_visible_content_preserves_thinking_for_provider() {
    let message = Message::assistant()
        .with_thinking("internal reasoning", "sig")
        .with_redacted_thinking("redacted")
        .with_text("final answer");

    let provider_message = message.agent_visible_content();
    assert_eq!(provider_message.content.len(), 3);
    assert!(matches!(
        provider_message.content[0],
        MessageContentBlock::Thinking(_)
    ));
    assert!(matches!(
        provider_message.content[1],
        MessageContentBlock::RedactedThinking(_)
    ));
}

#[test]
fn test_user_visible_content_filters_audience_without_dropping_thinking() {
    let assistant_text = TextContent::new("assistant text")
        .with_annotations(Annotations::default().with_audience(vec![Role::Assistant]));
    let assistant_image = ImageContent::new("assistant image", "image/png")
        .with_annotations(Annotations::default().with_audience(vec![Role::Assistant]));
    let assistant_tool_content = ContentBlock::Text(
        TextContent::new("assistant tool result")
            .with_annotations(Annotations::default().with_audience(vec![Role::Assistant])),
    );
    let user_tool_content = ContentBlock::Text(
        TextContent::new("user tool result")
            .with_annotations(Annotations::default().with_audience(vec![Role::User])),
    );

    let message = Message::assistant()
        .with_content(MessageContentBlock::Text(assistant_text))
        .with_text("shared text")
        .with_content(MessageContentBlock::Image(assistant_image))
        .with_tool_response(
            "tool-1",
            Ok(CallToolResult::success(vec![
                assistant_tool_content,
                user_tool_content,
            ])),
        )
        .with_thinking("visible reasoning", "sig");

    let projected = message.user_visible_content();

    assert_eq!(projected.as_concat_text(), "shared text");
    assert!(projected
        .content
        .iter()
        .any(|content| matches!(content, MessageContentBlock::Thinking(_))));
    assert!(!projected
        .content
        .iter()
        .any(|content| matches!(content, MessageContentBlock::Image(_))));
    let tool_response = projected
        .content
        .iter()
        .find_map(|content| match content {
            MessageContentBlock::ToolResponse(response) => Some(response),
            _ => None,
        })
        .expect("tool response should be preserved");
    let result = tool_response
        .tool_result
        .as_ref()
        .expect("tool result should be valid");
    assert_eq!(result.content.len(), 1);
    assert_eq!(
        result.content[0].as_text().unwrap().text,
        "user tool result"
    );
}

#[test]
fn test_user_visible_content_rejoins_text_across_hidden_blocks() {
    let user_text = |text: &str| {
        MessageContentBlock::Text(
            TextContent::new(text)
                .with_annotations(Annotations::default().with_audience(vec![Role::User])),
        )
    };
    let assistant_text = TextContent::new("provider state")
        .with_annotations(Annotations::default().with_audience(vec![Role::Assistant]));
    let message = Message::assistant()
        .with_content(user_text("Hello"))
        .with_content(MessageContentBlock::Text(assistant_text))
        .with_content(user_text(" world"));

    let projected = message.user_visible_content();

    assert_eq!(projected.content.len(), 1);
    assert_eq!(projected.as_concat_text(), "Hello world");
}

#[test]
fn test_from_prompt_message_text() {
    let prompt_content = ContentBlock::text("Hello, world!");

    let prompt_message = PromptMessage::new(Role::User, prompt_content);

    let message = Message::from(prompt_message);

    if let MessageContentBlock::Text(text_content) = &message.content[0] {
        assert_eq!(text_content.text, "Hello, world!");
    } else {
        panic!("Expected MessageContentBlock::Text");
    }
}

#[test]
fn test_from_prompt_message_text_preserves_visible_unicode() {
    let prompt_message = PromptMessage::new(Role::User, ContentBlock::text("Grüße 你好 🪿"));

    let message = Message::from(prompt_message);

    assert_eq!(message.as_concat_text(), "Grüße 你好 🪿");
}

#[test]
fn test_from_prompt_message_text_removes_unicode_tags() {
    let prompt_message = PromptMessage::new(
        Role::User,
        ContentBlock::text("visible\u{E0000}\u{E0041}\u{E007F} text"),
    );

    let message = Message::from(prompt_message);

    assert_eq!(message.as_concat_text(), "visible text");
}

#[test]
fn test_from_prompt_message_image() {
    let prompt_content = ContentBlock::image("base64data", "image/jpeg");

    let prompt_message = PromptMessage::new(Role::User, prompt_content);

    let message = Message::from(prompt_message);

    if let MessageContentBlock::Image(image_content) = &message.content[0] {
        assert_eq!(image_content.data, "base64data");
        assert_eq!(image_content.mime_type, "image/jpeg");
    } else {
        panic!("Expected MessageContentBlock::Image");
    }
}

#[test]
fn test_from_prompt_message_text_resource() {
    let resource = ResourceContents::TextResourceContents {
        uri: "file:///test.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: "Resource content".to_string(),
        meta: None,
    };

    let prompt_content = ContentBlock::resource(resource);

    let prompt_message = PromptMessage::new(Role::User, prompt_content);

    let message = Message::from(prompt_message);

    if let MessageContentBlock::Text(text_content) = &message.content[0] {
        assert_eq!(text_content.text, "Resource content");
    } else {
        panic!("Expected MessageContentBlock::Text");
    }
}

#[test]
fn test_from_prompt_message() {
    let prompt_message = PromptMessage::new(Role::User, ContentBlock::text("Hello, world!"));

    let message = Message::from(prompt_message);
    assert_eq!(message.role, Role::User);
    assert_eq!(message.content.len(), 1);
    assert_eq!(message.as_concat_text(), "Hello, world!");

    let prompt_message =
        PromptMessage::new(Role::Assistant, ContentBlock::text("I can help with that."));

    let message = Message::from(prompt_message);
    assert_eq!(message.role, Role::Assistant);
    assert_eq!(message.content.len(), 1);
    assert_eq!(message.as_concat_text(), "I can help with that.");
}

#[test]
fn test_message_with_text() {
    let message = Message::user().with_text("Hello");
    assert_eq!(message.as_concat_text(), "Hello");
}

#[test]
fn test_message_with_tool_request() {
    let tool_call = Ok(CallToolRequestParams::new("test_tool").with_arguments(object!({})));

    let message = Message::assistant().with_tool_request("req1", tool_call);
    assert!(message.is_tool_call());
    assert!(!message.is_tool_response());

    let ids = message.get_tool_ids();
    assert_eq!(ids.len(), 1);
    assert!(ids.contains("req1"));
}

#[test]
fn test_message_metadata_defaults() {
    let message = Message::user().with_text("Test");

    assert!(message.is_user_visible());
    assert!(message.is_agent_visible());
}

#[test]
fn test_message_visibility_methods() {
    let user_only_msg = Message::user().with_text("User only").user_only();
    assert!(user_only_msg.is_user_visible());
    assert!(!user_only_msg.is_agent_visible());

    let agent_only_msg = Message::assistant().with_text("Agent only").agent_only();
    assert!(!agent_only_msg.is_user_visible());
    assert!(agent_only_msg.is_agent_visible());

    let custom_msg = Message::user()
        .with_text("Custom visibility")
        .with_visibility(false, true);
    assert!(!custom_msg.is_user_visible());
    assert!(custom_msg.is_agent_visible());
}

#[test]
fn test_message_metadata_serialization() {
    let mut message = Message::user()
        .with_text("Test message")
        .with_visibility(false, true);
    message.metadata.output_token_limit_reached = true;

    let json_str = serde_json::to_string(&message).unwrap();
    let value: Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(value["metadata"]["userVisible"], false);
    assert_eq!(value["metadata"]["agentVisible"], true);
    assert_eq!(value["metadata"]["outputTokenLimitReached"], true);
}

#[test]
fn test_message_metadata_deserialization() {
    let json_with_metadata = r#"{
        "role": "user",
        "created": 1640995200,
        "content": [{
            "type": "text",
            "text": "Test"
        }],
        "metadata": {
            "userVisible": false,
            "agentVisible": true
        }
    }"#;

    let message: Message = serde_json::from_str(json_with_metadata).unwrap();
    assert!(!message.is_user_visible());
    assert!(message.is_agent_visible());
    assert!(!message.metadata.output_token_limit_reached);
}

#[test]
fn test_message_metadata_static_methods() {
    let agent_only_metadata = MessageMetadata::agent_only();
    assert!(!agent_only_metadata.user_visible);
    assert!(agent_only_metadata.agent_visible);

    let user_only_metadata = MessageMetadata::user_only();
    assert!(user_only_metadata.user_visible);
    assert!(!user_only_metadata.agent_visible);

    let invisible_metadata = MessageMetadata::invisible();
    assert!(!invisible_metadata.user_visible);
    assert!(!invisible_metadata.agent_visible);

    let agent_msg = Message::assistant()
        .with_text("Agent only message")
        .with_metadata(MessageMetadata::agent_only());
    assert!(!agent_msg.is_user_visible());
    assert!(agent_msg.is_agent_visible());

    let user_msg = Message::user()
        .with_text("User only message")
        .with_metadata(MessageMetadata::user_only());
    assert!(user_msg.is_user_visible());
    assert!(!user_msg.is_agent_visible());

    let invisible_msg = Message::user()
        .with_text("Invisible message")
        .with_metadata(MessageMetadata::invisible());
    assert!(!invisible_msg.is_user_visible());
    assert!(!invisible_msg.is_agent_visible());
}

#[test]
fn test_message_metadata_builder_methods() {
    let metadata = MessageMetadata::default().with_agent_invisible();
    assert!(metadata.user_visible);
    assert!(!metadata.agent_visible);

    let metadata = MessageMetadata::default().with_user_invisible();
    assert!(!metadata.user_visible);
    assert!(metadata.agent_visible);

    let metadata = MessageMetadata::invisible().with_agent_visible();
    assert!(!metadata.user_visible);
    assert!(metadata.agent_visible);

    let metadata = MessageMetadata::invisible().with_user_visible();
    assert!(metadata.user_visible);
    assert!(!metadata.agent_visible);

    let metadata = MessageMetadata::invisible()
        .with_user_visible()
        .with_agent_visible();
    assert!(metadata.user_visible);
    assert!(metadata.agent_visible);
}
