use super::super::*;
use crate::conversation::message::Message;
use serde_json::json;

#[test]
fn user_document_becomes_a_base64_document_block() {
    let messages = vec![Message::user().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    )];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "user");
    let block = &spec[0]["content"][0];
    assert_eq!(block["type"], DOCUMENT_TYPE);
    assert_eq!(block["title"], "q3-report.pdf");
    assert_eq!(
        block[SOURCE_FIELD],
        json!({
            "type": "base64",
            "media_type": "application/pdf",
            "data": "cGRmLWJ5dGVz",
        })
    );
}

#[test]
fn assistant_document_becomes_a_text_block() {
    let messages = vec![Message::assistant().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    )];

    let spec = format_messages(&messages);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    assert_eq!(spec[0]["content"][0]["type"], "text");
    let text = spec[0]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("q3-report.pdf"), "{text}");
    assert!(text.contains("user messages"), "{text}");
    assert!(!text.contains("cGRmLWJ5dGVz"), "{text}");
}

#[test]
fn unsupported_document_media_type_becomes_an_explicit_text_block() {
    let messages = vec![Message::user().with_document(
        "cm93cw==",
        "text/csv",
        Some("rows.csv".to_string()),
    )];

    let spec = format_messages(&messages);

    assert_eq!(spec[0]["content"][0]["type"], "text");
    let text = spec[0]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("rows.csv"), "{text}");
    assert!(text.contains("text/csv"), "{text}");
    assert!(text.contains("application/pdf"), "{text}");
    assert!(!text.contains("cm93cw=="), "{text}");
}

