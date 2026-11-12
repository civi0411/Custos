use super::super::*;
use crate::conversation::message::Message;
use serde_json::{json, Value};

    fn format(messages: &[Message]) -> Vec<Value> {
        let mut items = Vec::new();
        add_message_items(&mut items, messages, true);
        items
    }

    #[test]
    fn user_document_becomes_an_input_file_item() {
        let items = format(&[Message::user().with_document(
            "cGRmLWJ5dGVz",
            "application/pdf",
            Some("q3-report.pdf".to_string()),
        )]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["role"], "user");
        assert_eq!(
            items[0]["content"][0],
            json!({
                "type": "input_file",
                "filename": "q3-report.pdf",
                "file_data": "data:application/pdf;base64,cGRmLWJ5dGVz",
            })
        );
    }

    #[test]
    fn assistant_document_becomes_an_output_text_item() {
        let items = format(&[Message::assistant().with_document(
            "cGRmLWJ5dGVz",
            "application/pdf",
            Some("q3-report.pdf".to_string()),
        )]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["role"], "assistant");
        let part = &items[0]["content"][0];
        assert_eq!(part["type"], "output_text");
        let text = part["text"].as_str().unwrap();
        assert!(text.contains("q3-report.pdf"), "{text}");
        assert!(text.contains("user messages"), "{text}");
        assert!(!text.contains("cGRmLWJ5dGVz"), "{text}");
    }
