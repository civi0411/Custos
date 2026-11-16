use crate::routing::streaming::{ModelTurnStreamDecoder, Wire};
use custos_provider::turn::TurnDelta;
use uuid::Uuid;

#[test]
fn model_turn_stream_decoder_separates_reasoning_and_text() {
    let mut decoder = ModelTurnStreamDecoder::default();
    let attempt = Uuid::new_v4();

    // 1. Claude thinking delta followed by text delta
    let events = decoder
        .decode(
            attempt,
            "anthropic",
            "claude-3-7-sonnet",
            Wire::Claude,
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"let me think"}}"#,
        )
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0].delta {
        TurnDelta::Reasoning { text } => assert_eq!(text, "let me think"),
        other => panic!("expected Reasoning delta, got {:?}", other),
    }

    let events = decoder
        .decode(
            attempt,
            "anthropic",
            "claude-3-7-sonnet",
            Wire::Claude,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello world"}}"#,
        )
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0].delta {
        TurnDelta::Text { text } => assert_eq!(text, "hello world"),
        other => panic!("expected Text delta, got {:?}", other),
    }

    // 2. OpenAI reasoning_content followed by content
    let mut oai_decoder = ModelTurnStreamDecoder::default();
    let events = oai_decoder
        .decode(
            attempt,
            "openai",
            "o3-mini",
            Wire::OpenAi,
            r#"{"choices":[{"delta":{"reasoning_content":"step 1 reasoning"}}]}"#,
        )
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0].delta {
        TurnDelta::Reasoning { text } => assert_eq!(text, "step 1 reasoning"),
        other => panic!("expected Reasoning delta, got {:?}", other),
    }

    let events = oai_decoder
        .decode(
            attempt,
            "openai",
            "o3-mini",
            Wire::OpenAi,
            r#"{"choices":[{"delta":{"content":"final answer"}}]}"#,
        )
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0].delta {
        TurnDelta::Text { text } => assert_eq!(text, "final answer"),
        other => panic!("expected Text delta, got {:?}", other),
    }

    // 3. Google Gemini thought part
    let mut gemini_decoder = ModelTurnStreamDecoder::default();
    let events = gemini_decoder
        .decode(
            attempt,
            "google",
            "gemini-2.5-flash",
            Wire::Google,
            r#"{"candidates":[{"content":{"parts":[{"text":"pondering deep questions","thought":true},{"text":"clear answer"}]}}]}"#,
        )
        .unwrap();
    assert_eq!(events.len(), 2);
    match &events[0].delta {
        TurnDelta::Reasoning { text } => assert_eq!(text, "pondering deep questions"),
        other => panic!("expected Reasoning delta, got {:?}", other),
    }
    match &events[1].delta {
        TurnDelta::Text { text } => assert_eq!(text, "clear answer"),
        other => panic!("expected Text delta, got {:?}", other),
    }
}
