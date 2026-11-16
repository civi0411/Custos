use super::super::*;
use crate::conversation::message::Message;
use crate::images::ImageFormat;
use serde_json::json;

    fn format(messages: &[Message]) -> Vec<DatabricksMessage> {
        format_messages(messages, &ImageFormat::OpenAi, None, true)
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
            spec[0].content[0],
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
        assert_eq!(spec[0].role, "assistant");
        let text = spec[0].content.as_str().unwrap();
        assert!(text.contains("q3-report.pdf"), "{text}");
        assert!(text.contains("user messages"), "{text}");
        assert!(!text.contains("cGRmLWJ5dGVz"), "{text}");
    }
