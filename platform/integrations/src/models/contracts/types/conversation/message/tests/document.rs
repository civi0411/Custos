use crate::conversation::message::{Message, MessageContent};

#[test]
fn document_content_carries_media_type_and_name() {
    let message = Message::user().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    );

    let MessageContent::Document(document) = &message.content[0] else {
        panic!("expected document content");
    };
    assert_eq!(document.data, "cGRmLWJ5dGVz");
    assert_eq!(document.mime_type, "application/pdf");
    assert_eq!(document.name.as_deref(), Some("q3-report.pdf"));
    assert_eq!(
        message.content[0].to_string(),
        "[Document: q3-report.pdf (application/pdf)]"
    );
}

#[test]
fn document_content_without_name_is_allowed() {
    let content = MessageContent::document("cGRmLWJ5dGVz", "application/pdf", None);

    assert!(matches!(&content, MessageContent::Document(document) if document.name.is_none()));
    assert_eq!(content.to_string(), "[Document: application/pdf]");
}

#[test]
fn document_content_round_trips_through_serde() {
    let message = Message::user().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    );

    let json = serde_json::to_value(&message).unwrap();
    assert_eq!(json["content"][0]["type"], "document");
    assert_eq!(json["content"][0]["mimeType"], "application/pdf");
    assert_eq!(json["content"][0]["name"], "q3-report.pdf");

    let restored: Message = serde_json::from_value(json).unwrap();
    assert_eq!(restored, message);
}
