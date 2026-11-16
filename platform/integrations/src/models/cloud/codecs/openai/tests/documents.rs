use super::super::*;
use crate::conversation::message::Message;
use crate::images::ImageFormat;
use serde_json::{json, Value};

fn format(messages: &[Message]) -> Vec<Value> {
    format_messages_with_options(
        messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            supports_vision: true,
            ..Default::default()
        },
    )
}

#[test]
fn user_document_becomes_a_file_content_part() {
    let spec = format(&[Message::user().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    )]);

    assert_eq!(spec.len(), 1);
    assert_eq!(
        spec[0]["content"][0],
        json!({
            "type": "file",
            "file": {
                "filename": "q3-report.pdf",
                "file_data": "data:application/pdf;base64,cGRmLWJ5dGVz",
            }
        })
    );
}

#[test]
fn assistant_document_becomes_a_text_part() {
    let spec = format(&[Message::assistant().with_document(
        "cGRmLWJ5dGVz",
        "application/pdf",
        Some("q3-report.pdf".to_string()),
    )]);

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["role"], "assistant");
    let text = spec[0]["content"].as_str().unwrap();
    assert!(text.contains("q3-report.pdf"), "{text}");
    assert!(text.contains("user messages"), "{text}");
    assert!(!text.contains("cGRmLWJ5dGVz"), "{text}");
}

#[test]
fn unsupported_document_media_type_becomes_an_explicit_text_part() {
    let spec = format(&[Message::user().with_document(
        "cm93cw==",
        "text/csv",
        Some("rows.csv".to_string()),
    )]);

    let text = spec[0]["content"].as_str().unwrap();
    assert!(text.contains("rows.csv"), "{text}");
    assert!(text.contains("application/pdf"), "{text}");
    assert!(!text.contains("cm93cw=="), "{text}");
}

