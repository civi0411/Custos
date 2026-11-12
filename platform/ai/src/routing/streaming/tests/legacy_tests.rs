use super::helpers::response;
use crate::routing::streaming::{
    stream_response, stream_response_with_idle_timeout, unary_completion, Decoder, Wire,
};
use custos_provider::ProviderEventType;
use std::time::Duration;

#[tokio::test]
async fn utf8_split_delta_precedes_completion_with_usage() {
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"Xin chào\"}}]}\n\ndata: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3}}\n\ndata: [DONE]\n\n".as_bytes();
    let split = body.iter().position(|b| *b == 0xc3).unwrap() + 1;
    let mut receiver = stream_response(
        response(vec![body[..split].to_vec(), body[split..].to_vec()]).await,
        "req".into(),
        Wire::OpenAi,
    );
    let first = receiver.recv().await.unwrap();
    assert_eq!(first.event_type, ProviderEventType::Chunk);
    assert_eq!(first.text.as_deref(), Some("Xin chào"));
    let usage = receiver.recv().await.unwrap();
    assert_eq!(usage.event_type, ProviderEventType::Usage);
    let done = receiver.recv().await.unwrap();
    assert_eq!(done.event_type, ProviderEventType::Completed);
    assert_eq!(done.usage.unwrap().output_tokens, Some(3));
}

#[tokio::test]
async fn midstream_failure_keeps_prior_delta() {
    let mut receiver = stream_response(
        response(vec![
            b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec(),
            b"data: {\"error\":{\"message\":\"secret\"}}\n\n".to_vec(),
        ])
        .await,
        "req".into(),
        Wire::OpenAi,
    );
    assert_eq!(
        receiver.recv().await.unwrap().text.as_deref(),
        Some("partial")
    );
    assert_eq!(
        receiver.recv().await.unwrap().event_type,
        ProviderEventType::Failed
    );
}

#[tokio::test]
async fn idle_timeout_reports_failure_after_partial_output() {
    let response = response(vec![
        b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec(),
    ])
    .await;
    let mut receiver = stream_response_with_idle_timeout(
        response,
        "req".into(),
        Wire::OpenAi,
        Duration::from_millis(10),
    );
    assert_eq!(
        receiver.recv().await.unwrap().event_type,
        ProviderEventType::Chunk
    );
    let failure = receiver.recv().await.unwrap();
    assert_eq!(failure.event_type, ProviderEventType::Failed);
    assert!(failure.error.unwrap().contains("timed out"));
}

#[test]
fn canonical_response_excludes_private_reasoning_and_keeps_public_refusal() {
    let mut decoder = Decoder::default();
    decoder.decode("req",Wire::Codex,r#"{"type":"response.completed","response":{"output":[{"content":[{"type":"reasoning_text","text":"private chain"},{"type":"output_text","text":"public answer"}]}]}}"#).unwrap();
    assert_eq!(decoder.text, "public answer");
    let mut decoder = Decoder::default();
    let events = decoder.decode("req",Wire::Claude,r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","text":"private chain","thinking":"private chain"}}"#).unwrap();
    assert!(events
        .iter()
        .all(|e| e.text.as_deref() != Some("private chain")));
    let events = decoder
        .decode(
            "req",
            Wire::Codex,
            r#"{"type":"response.refusal.delta","delta":"public refusal"}"#,
        )
        .unwrap();
    assert_eq!(events[0].text.as_deref(), Some("public refusal"));
}

#[test]
fn unary_fallback_emits_only_canonical_completion() {
    let event = unary_completion("req",Wire::OpenAi,&serde_json::json!({"choices":[{"message":{"content":"whole answer"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}})).unwrap();
    assert_eq!(event.event_type, ProviderEventType::Completed);
    assert_eq!(event.text.as_deref(), Some("whole answer"));
    assert_eq!(event.usage.unwrap().input_tokens, Some(3));
}

#[test]
fn ollama_claude_google_and_public_summary_usage() {
    for (wire, frame) in [
        (
            Wire::Ollama,
            r#"{"message":{"content":"ok"},"done":true,"prompt_eval_count":2,"eval_count":1}"#,
        ),
        (
            Wire::Google,
            r#"{"candidates":[{"content":{"parts":[{"text":"ok"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":1}}"#,
        ),
    ] {
        let mut decoder = Decoder::default();
        assert_eq!(
            decoder.decode("req", wire, frame).unwrap()[0]
                .text
                .as_deref(),
            Some("ok")
        );
        assert!(decoder.completed);
        assert_eq!(decoder.output, Some(1));
    }
    let mut decoder = Decoder::default();
    let events = decoder.decode("req",Wire::Claude,r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"private"}}"#).unwrap();
    assert_eq!(events[0].event_type, ProviderEventType::Thinking);
    assert_eq!(events[0].text.as_deref(), Some(""));
    let events = decoder
        .decode(
            "req",
            Wire::Codex,
            r#"{"type":"response.reasoning_summary_text.delta","delta":"public"}"#,
        )
        .unwrap();
    assert_eq!(events[0].event_type, ProviderEventType::ReasoningSummary);
}
