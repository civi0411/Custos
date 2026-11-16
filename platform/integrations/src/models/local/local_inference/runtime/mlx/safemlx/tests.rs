use super::backend::token_id_or_ids;
use super::generation::{final_stream_suffix, mlx_max_tokens, split_generated_thinking};
use crate::local_inference::model::ModelSettings;
use serde_json::json;

#[test]
fn extracts_thinking_started_by_generation_prompt() {
    let (content, thinking) = split_generated_thinking(
        "hidden reasoning</think>visible answer",
        "<|im_start|>assistant\n<think>\n",
        true,
    );
    assert_eq!(thinking, "hidden reasoning");
    assert_eq!(content, "visible answer");
}

#[test]
fn leaves_think_tags_as_content_when_thinking_disabled() {
    let (content, thinking) = split_generated_thinking(
        "<think>plain text</think>answer",
        "<|im_start|>assistant\n",
        false,
    );
    assert!(thinking.is_empty());
    assert_eq!(content, "<think>plain text</think>answer");
}

#[test]
fn parses_single_and_multiple_eos_token_ids() {
    assert_eq!(token_id_or_ids(&json!(106)), vec![106]);
    assert_eq!(
        token_id_or_ids(&json!([1, 2, "ignored", 3])),
        vec![1, 2, 3]
    );
    assert!(token_id_or_ids(&json!({"not": "an id"})).is_empty());
}

#[test]
fn final_stream_suffix_flushes_append_only_suffix() {
    assert_eq!(
        final_stream_suffix("hello world", "hello")
            .unwrap()
            .as_deref(),
        Some(" world")
    );
}

#[test]
fn final_stream_suffix_does_not_replay_fully_streamed_text() {
    assert_eq!(
        final_stream_suffix("hello world", "hello world")
            .unwrap()
            .as_deref(),
        Some("")
    );
}

#[test]
fn final_stream_suffix_allows_unstreamed_fallback() {
    assert_eq!(final_stream_suffix("hello world", "").unwrap(), None);
}

#[test]
fn final_stream_suffix_rejects_rewritten_streamed_prefix() {
    assert!(final_stream_suffix("hello world", "different prefix").is_err());
}

#[test]
fn max_tokens_defaults_to_context_headroom() {
    let settings = ModelSettings::default();
    assert_eq!(mlx_max_tokens(&settings, None, 128_000, 1_752), 126_248);
}

#[test]
fn max_tokens_respects_configured_caps() {
    let mut settings = ModelSettings::default();
    settings.max_output_tokens = Some(256);
    assert_eq!(mlx_max_tokens(&settings, Some(1024), 128_000, 1_752), 256);

    let settings = ModelSettings::default();
    assert_eq!(mlx_max_tokens(&settings, Some(1024), 128_000, 1_752), 1024);
}
