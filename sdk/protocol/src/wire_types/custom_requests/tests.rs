use super::*;

#[test]
fn export_session_request_defaults_to_json_without_format() {
    let req: ExportSessionRequest = serde_json::from_str(r#"{"sessionId":"abc"}"#).unwrap();

    assert_eq!(req.format, SessionExportFormat::Json);
}

#[test]
fn export_session_request_accepts_markdown_format() {
    let req: ExportSessionRequest =
        serde_json::from_str(r#"{"sessionId":"abc","format":"markdown"}"#).unwrap();

    assert_eq!(req.format, SessionExportFormat::Markdown);
}
