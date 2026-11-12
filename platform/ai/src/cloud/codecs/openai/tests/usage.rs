use super::super::*;
use serde_json::json;

#[test]
fn test_get_usage_preserves_provider_totals_with_cache_fields() {
    let usage = get_usage(&json!({
        "prompt_tokens": 120,
        "completion_tokens": 30,
        "total_tokens": 150,
        "cache_read_input_tokens": 80,
        "cache_creation_input_tokens": 20
    }));

    assert_eq!(usage.input_tokens, Some(120));
    assert_eq!(usage.output_tokens, Some(30));
    assert_eq!(usage.total_tokens, Some(150));
    assert_eq!(usage.cache_read_input_tokens, Some(80));
    assert_eq!(usage.cache_write_input_tokens, Some(20));
}

#[test]
fn test_get_usage_reads_openai_prompt_tokens_details() {
    let usage = get_usage(&json!({
        "prompt_tokens": 120,
        "completion_tokens": 30,
        "total_tokens": 150,
        "prompt_tokens_details": {
            "cached_tokens": 80
        }
    }));

    assert_eq!(usage.input_tokens, Some(120));
    assert_eq!(usage.output_tokens, Some(30));
    assert_eq!(usage.total_tokens, Some(150));
    assert_eq!(usage.cache_read_input_tokens, Some(80));
    assert_eq!(usage.cache_write_input_tokens, None);
}

#[test]
fn test_get_usage_reads_nested_usage_with_cache_fields() {
    let usage = get_usage(&json!({
        "id": "chatcmpl_test",
        "object": "chat.completion",
        "usage": {
            "prompt_tokens": 84,
            "completion_tokens": 21,
            "total_tokens": 105,
            "prompt_tokens_details": {
                "cached_tokens": 60
            },
            "cache_creation_input_tokens": 10
        }
    }));

    assert_eq!(usage.input_tokens, Some(84));
    assert_eq!(usage.output_tokens, Some(21));
    assert_eq!(usage.total_tokens, Some(105));
    assert_eq!(usage.cache_read_input_tokens, Some(60));
    assert_eq!(usage.cache_write_input_tokens, Some(10));
}

#[test]
fn test_get_usage_with_ollama_native_fields() {
    // Ollama-native fields should be picked up as fallback
    let usage = json!({
        "prompt_eval_count": 42,
        "eval_count": 128
    });
    let result = get_usage(&usage);
    assert_eq!(result.input_tokens, Some(42));
    assert_eq!(result.output_tokens, Some(128));
    assert_eq!(result.total_tokens, Some(170));
}

#[test]
fn test_get_usage_prefers_openai_fields_over_ollama() {
    // Standard OpenAI fields should take precedence
    let usage = json!({
        "prompt_tokens": 10,
        "completion_tokens": 20,
        "prompt_eval_count": 42,
        "eval_count": 128
    });
    let result = get_usage(&usage);
    assert_eq!(result.input_tokens, Some(10));
    assert_eq!(result.output_tokens, Some(20));
    assert_eq!(result.total_tokens, Some(30));
}

#[test]
fn test_get_usage_falls_back_when_openai_fields_are_null() {
    // When OpenAI fields exist but are null, should fall back to Ollama-native
    let usage = json!({
        "prompt_tokens": null,
        "completion_tokens": null,
        "prompt_eval_count": 42,
        "eval_count": 128
    });
    let result = get_usage(&usage);
    assert_eq!(result.input_tokens, Some(42));
    assert_eq!(result.output_tokens, Some(128));
    assert_eq!(result.total_tokens, Some(170));
}

