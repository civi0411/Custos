use super::adapter::LocalInferenceProvider;
use super::runtime::messages::{convert_text_media_markers, extract_text_content};
use super::runtime::preload::preload_model;
use super::runtime::runtime::evict_model;
use custos_provider::base::Provider;
use custos_provider::conversation::message::Message;
use custos_provider::model::ModelConfig;
use serde_json::json;

#[tokio::test]
async fn real_chat_gguf_loads_and_generates_when_fixture_is_configured() {
    let Ok(model_path) = std::env::var("CUSTOS_TEST_CHAT_GGUF") else {
        eprintln!("skipping native generation fixture: set CUSTOS_TEST_CHAT_GGUF");
        return;
    };
    assert!(
        std::path::Path::new(&model_path).is_file(),
        "CUSTOS_TEST_CHAT_GGUF must point to a real chat GGUF file"
    );

    let loaded = preload_model(&model_path, Some(2048), Some(0))
        .await
        .expect("GGUF weights must load through the Custos backend");
    assert_eq!(loaded.model_path, model_path);

    let provider = LocalInferenceProvider::from_env()
        .await
        .expect("native inference runtime must initialize");
    let mut config = ModelConfig::new(&model_path);
    config.max_tokens = Some(16);
    let (answer, _) = provider
        .complete(
            &config,
            "Answer briefly.",
            &[Message::user().with_text("Say hello")],
            &[],
        )
        .await
        .expect("loaded GGUF must generate a chat response");
    assert!(
        !extract_text_content(&answer).trim().is_empty(),
        "chat model generated no text"
    );
    evict_model(&model_path)
        .await
        .expect("test model must unload cleanly");
}

#[test]
fn converts_marker_in_string_content_to_media_marker_part() {
    let mut messages = vec![json!({
        "role": "user",
        "content": "look\n<__media__>\nclosely",
    })];

    convert_text_media_markers(&mut messages, "<__media__>");

    assert_eq!(
        messages[0]["content"],
        json!([
            {"type": "text", "text": "look"},
            {"type": "media_marker", "text": "<__media__>"},
            {"type": "text", "text": "closely"},
        ])
    );
}

#[test]
fn converts_marker_inside_text_content_parts() {
    let mut messages = vec![json!({
        "role": "user",
        "content": [
            {"type": "text", "text": "<__media__>describe"},
            {"type": "text", "text": "next"},
            {"type": "media_marker", "text": "<__media__>"},
        ],
    })];

    convert_text_media_markers(&mut messages, "<__media__>");

    assert_eq!(
        messages[0]["content"],
        json!([
            {"type": "media_marker", "text": "<__media__>"},
            {"type": "text", "text": "describe"},
            {"type": "text", "text": "next"},
            {"type": "media_marker", "text": "<__media__>"},
        ])
    );
}

#[test]
fn preserves_balanced_info_tags_in_text_content() {
    let message = Message::user().with_text("before <info-msg>executed payload</info-msg> after");

    assert_eq!(
        extract_text_content(&message),
        "before <info-msg>executed payload</info-msg> after"
    );
}

#[test]
fn preserves_unterminated_info_tags_in_text_content() {
    let message = Message::user().with_text("before <info-msg>executed payload");

    assert_eq!(
        extract_text_content(&message),
        "before <info-msg>executed payload"
    );
}

#[test]
fn preserves_legitimate_text_content() {
    let message = Message::user().with_text("ordinary user content");

    assert_eq!(extract_text_content(&message), "ordinary user content");
}
