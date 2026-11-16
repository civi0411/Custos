use crate::conversation::message::{
    Message, MessageContentBlock, ProviderMetadata, ToolResponse,
};
use base64::Engine;
use rmcp::model::{
    Annotations, CallToolResult, ContentBlock, EmbeddedResource, ErrorCode, ErrorData,
    ImageContent, ResourceContents, Role, TextContent,
};
use rmcp::object;

#[test]
fn test_sanitize_with_text() {
    let malicious = "Hello\u{E0041}\u{E0042}\u{E0043}world"; // Invisible "ABC"
    let message = Message::user().with_text(malicious);
    assert_eq!(message.as_concat_text(), "Helloworld");
}

#[test]
fn test_no_sanitize_with_text() {
    let clean_text = "Hello world 世界 🌍";
    let message = Message::user().with_text(clean_text);
    assert_eq!(message.as_concat_text(), clean_text);
}

#[test]
fn test_tool_response_sanitizes_unicode_tags() {
    let content = MessageContentBlock::tool_response(
        "tool-1",
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "visible\u{E0041}\u{E0042}text",
        )])),
    );

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    let result = response.tool_result.unwrap();
    let ContentBlock::Text(text) = &result.content[0] else {
        panic!("expected text content");
    };
    assert_eq!(text.text, "visibletext");
}

#[test]
fn test_tool_response_with_metadata_sanitizes_unicode_tags() {
    let mut metadata = ProviderMetadata::new();
    metadata.insert("provider".to_string(), serde_json::json!("test"));
    let tagged = ContentBlock::Text(
        TextContent::new("result\u{E0041}")
            .with_annotations(Annotations::default().with_audience(vec![Role::Assistant])),
    );
    let mut message = Message::user();

    message.add_tool_response_with_metadata(
        "tool-1",
        Ok(CallToolResult::success(vec![tagged])),
        Some(&metadata),
    );

    let MessageContentBlock::ToolResponse(response) = &message.content[0] else {
        panic!("expected tool response");
    };
    assert_eq!(response.metadata.as_ref(), Some(&metadata));
    let result = response.tool_result.as_ref().unwrap();
    let text = &result.content[0];
    let ContentBlock::Text(text) = text else {
        panic!("expected text content");
    };
    assert_eq!(
        text.annotations
            .as_ref()
            .and_then(|value| value.audience.as_ref()),
        Some(&vec![Role::Assistant])
    );
    assert_eq!(text.text, "result");
}

#[test]
fn test_tool_response_sanitizes_error_message() {
    let data = serde_json::json!({"retry": false});
    let content = MessageContentBlock::tool_response(
        "tool-1",
        Err(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            "error\u{E0041}text",
            Some(data.clone()),
        )),
    );

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    let error = response.tool_result.unwrap_err();
    assert_eq!(error.message, "errortext");
    assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
    assert_eq!(error.data, Some(data));
}

#[test]
fn test_tool_response_sanitizes_text_resource() {
    let resource = ResourceContents::TextResourceContents {
        uri: "file:///result.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: "resource\u{E0041}text".to_string(),
        meta: None,
    };
    let content = MessageContentBlock::tool_response(
        "tool-1",
        Ok(CallToolResult::success(vec![ContentBlock::Resource(
            EmbeddedResource::new(resource),
        )])),
    );

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    let result = response.tool_result.unwrap();
    let ContentBlock::Resource(resource) = &result.content[0] else {
        panic!("expected resource content");
    };
    let ResourceContents::TextResourceContents {
        uri,
        mime_type,
        text,
        meta,
    } = &resource.resource
    else {
        panic!("expected text resource");
    };
    assert_eq!(uri, "file:///result.txt");
    assert_eq!(mime_type.as_deref(), Some("text/plain"));
    assert_eq!(text, "resourcetext");
    assert!(meta.is_none());
}

#[test]
fn test_tool_response_sanitizes_utf8_blob_resource() {
    let blob =
        base64::engine::general_purpose::STANDARD.encode("resource\u{E0041}text".as_bytes());
    let resource = ResourceContents::BlobResourceContents {
        uri: "file:///result.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        blob,
        meta: None,
    };
    let content = MessageContentBlock::tool_response(
        "tool-1",
        Ok(CallToolResult::success(vec![ContentBlock::Resource(
            EmbeddedResource::new(resource),
        )])),
    );

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    let result = response.tool_result.unwrap();
    let ContentBlock::Resource(resource) = &result.content[0] else {
        panic!("expected resource content");
    };
    let ResourceContents::BlobResourceContents { blob, .. } = &resource.resource else {
        panic!("expected blob resource");
    };
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(blob)
            .unwrap(),
        b"resourcetext"
    );
}

#[test]
fn test_tool_response_sanitizes_malformed_blob_resource() {
    let resource = ResourceContents::BlobResourceContents {
        uri: "file:///result.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        blob: "malformed\u{E0041}text".to_string(),
        meta: None,
    };
    let content = MessageContentBlock::tool_response(
        "tool-1",
        Ok(CallToolResult::success(vec![ContentBlock::Resource(
            EmbeddedResource::new(resource),
        )])),
    );

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    let result = response.tool_result.unwrap();
    let ContentBlock::Resource(resource) = &result.content[0] else {
        panic!("expected resource content");
    };
    let ResourceContents::BlobResourceContents { blob, .. } = &resource.resource else {
        panic!("expected blob resource");
    };
    assert_eq!(blob, "malformedtext");
}

#[test]
fn test_deserialization_sanitizes_persisted_tool_response() {
    let message = Message::new(
        Role::User,
        1,
        vec![MessageContentBlock::ToolResponse(ToolResponse {
            id: "tool-1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![ContentBlock::text(
                "persisted\u{E0041}text",
            )])),
            metadata: None,
        })],
    );

    let json = serde_json::to_string(&message).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    let MessageContentBlock::ToolResponse(response) = &deserialized.content[0] else {
        panic!("expected tool response");
    };
    let result = response.tool_result.as_ref().unwrap();
    let ContentBlock::Text(text) = &result.content[0] else {
        panic!("expected text content");
    };
    assert_eq!(text.text, "persistedtext");
}

#[test]
fn test_content_deserialization_sanitizes_persisted_tool_response() {
    let content = vec![MessageContentBlock::ToolResponse(ToolResponse {
        id: "tool-1".to_string(),
        tool_result: Ok(CallToolResult::success(vec![ContentBlock::text(
            "persisted\u{E0041}text",
        )])),
        metadata: None,
    })];

    let json = serde_json::to_string(&content).unwrap();
    let deserialized: Vec<MessageContentBlock> = serde_json::from_str(&json).unwrap();
    let MessageContentBlock::ToolResponse(response) = &deserialized[0] else {
        panic!("expected tool response");
    };
    let result = response.tool_result.as_ref().unwrap();
    let ContentBlock::Text(text) = &result.content[0] else {
        panic!("expected text content");
    };
    assert_eq!(text.text, "persistedtext");
}

#[test]
fn test_tool_response_sanitization_preserves_legitimate_content() {
    let text = ContentBlock::Text(
        TextContent::new("世界 🌍 café")
            .with_annotations(Annotations::default().with_audience(vec![Role::Assistant])),
    );
    let image = ContentBlock::Image(
        ImageContent::new("image-data", "image/png")
            .with_annotations(Annotations::default().with_audience(vec![Role::User])),
    );
    let mut result = CallToolResult::success(vec![text, image]);
    result.structured_content = Some(serde_json::json!({"safe": "世界"}));
    result.meta = Some(rmcp::model::MetaObject(object!({"source": "test"})));
    let expected = result.clone();

    let content = MessageContentBlock::tool_response("tool-1", Ok(result));

    let MessageContentBlock::ToolResponse(response) = content else {
        panic!("expected tool response");
    };
    assert_eq!(response.tool_result.unwrap(), expected);
}
