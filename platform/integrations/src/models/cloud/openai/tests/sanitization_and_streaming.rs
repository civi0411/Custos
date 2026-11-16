use super::helpers::{cerebras_config, cerebras_provider, make_provider, make_provider_with_custom_models};
use crate::base::{Provider, ThinkingPreservationFormat};
use crate::cloud::openai::compat::apply_declared_request_params;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn sanitize_remaps_max_completion_tokens_for_compat_provider() {
    let provider = make_provider("mistral");
    let payload = json!({
        "model": "mistral-medium-latest",
        "messages": [],
        "max_completion_tokens": 16384
    });

    let result = provider
        .sanitize_request_for_compat(payload, &ModelConfig::new("mistral-medium-latest"));
    let obj = result.as_object().unwrap();

    assert!(!obj.contains_key("max_completion_tokens"));
    assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
}

#[test]
fn sanitize_preserves_existing_max_tokens_for_compat_provider() {
    let provider = make_provider("mistral");
    let payload = json!({
        "model": "mistral-medium-latest",
        "messages": [],
        "max_tokens": 4096,
        "max_completion_tokens": 16384
    });

    let result = provider
        .sanitize_request_for_compat(payload, &ModelConfig::new("mistral-medium-latest"));
    let obj = result.as_object().unwrap();

    assert!(!obj.contains_key("max_completion_tokens"));
    assert_eq!(obj.get("max_tokens").unwrap(), &json!(4096));
}

#[test]
fn sanitize_noop_for_native_openai_provider() {
    let provider = make_provider("openai");
    let payload = json!({
        "model": "o3",
        "messages": [],
        "max_completion_tokens": 16384
    });

    let result = provider.sanitize_request_for_compat(payload, &ModelConfig::new("o3"));
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("max_completion_tokens"));
    assert!(!obj.contains_key("max_tokens"));
}

#[test]
fn sanitize_noop_for_unknown_provider() {
    let provider = make_provider("some_future_provider");
    let payload = json!({
        "model": "future-model",
        "messages": [],
        "max_completion_tokens": 16384
    });

    let result =
        provider.sanitize_request_for_compat(payload, &ModelConfig::new("future-model"));
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("max_completion_tokens"));
    assert!(!obj.contains_key("max_tokens"));
}

#[test]
fn sanitize_no_token_params() {
    let provider = make_provider("groq");
    let payload = json!({
        "model": "llama-3.3-70b-versatile",
        "messages": []
    });

    let result = provider.sanitize_request_for_compat(
        payload.clone(),
        &ModelConfig::new("llama-3.3-70b-versatile"),
    );
    assert_eq!(result, payload);
}

#[test]
fn sanitize_nearai_reasoning_chat_params() {
    let provider = make_provider("nearai");
    let payload = json!({
        "model": "Qwen/Qwen3.6-35B-A3B-FP8",
        "messages": [
            {
                "role": "developer",
                "content": "system instructions"
            },
            {
                "role": "user",
                "content": "hello"
            }
        ],
        "reasoning_effort": "medium",
        "max_completion_tokens": 16384
    });

    let result = provider
        .sanitize_request_for_compat(payload, &ModelConfig::new("Qwen/Qwen3.6-35B-A3B-FP8"));
    let obj = result.as_object().unwrap();

    assert!(!obj.contains_key("reasoning_effort"));
    assert!(!obj.contains_key("max_completion_tokens"));
    assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
    assert_eq!(obj["messages"][0]["role"], "system");
    assert_eq!(obj["messages"][1]["role"], "user");
}

#[test]
fn sanitize_nearai_preserves_openai_reasoning_effort() {
    let provider = make_provider("nearai");
    let payload = json!({
        "model": "openai/gpt-5",
        "messages": [],
        "reasoning_effort": "medium",
        "max_completion_tokens": 16384
    });

    let result =
        provider.sanitize_request_for_compat(payload, &ModelConfig::new("openai/gpt-5"));
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("medium")));
    assert!(!obj.contains_key("max_completion_tokens"));
    assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
}

#[test]
fn sanitize_meta_applies_reasoning_effort_from_thinking_effort() {
    let provider = make_provider("meta");
    let payload = json!({
        "model": "muse-spark-1.1",
        "messages": []
    });
    let model_config =
        ModelConfig::new("muse-spark-1.1").with_thinking_effort(ThinkingEffort::High);

    let result = provider.sanitize_request_for_compat(payload, &model_config);
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
}

#[test]
fn sanitize_meta_maps_max_thinking_effort_to_xhigh() {
    let provider = make_provider("meta");
    let payload = json!({
        "model": "muse-spark-1.1",
        "messages": []
    });
    let model_config =
        ModelConfig::new("muse-spark-1.1").with_thinking_effort(ThinkingEffort::Max);

    let result = provider.sanitize_request_for_compat(payload, &model_config);
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("xhigh")));
}

#[test]
fn sanitize_meta_clamps_off_thinking_effort_to_low() {
    // Muse Spark always reasons and has no "disable reasoning" level,
    // so an explicit `Off` must be clamped to the lightest supported
    // level rather than omitted or sent as-is.
    let provider = make_provider("meta");
    let payload = json!({
        "model": "muse-spark-1.1",
        "messages": [],
        "reasoning_effort": "high"
    });
    let model_config =
        ModelConfig::new("muse-spark-1.1").with_thinking_effort(ThinkingEffort::Off);

    let result = provider.sanitize_request_for_compat(payload, &model_config);
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("low")));
}

#[test]
fn sanitize_meta_omits_reasoning_effort_when_unset() {
    let provider = make_provider("meta");
    let payload = json!({
        "model": "muse-spark-1.1",
        "messages": []
    });
    let model_config = ModelConfig::new("muse-spark-1.1");

    let result = provider.sanitize_request_for_compat(payload, &model_config);
    let obj = result.as_object().unwrap();

    assert!(!obj.contains_key("reasoning_effort"));
}

#[tokio::test]
async fn nonstreaming_chat_accepts_legitimate_response() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hello"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })))
        .mount(&server)
        .await;

    let mut provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["test-model".to_string()],
    );
    provider.supports_streaming = false;

    let _stream = provider
        .stream(&ModelConfig::new("test-model"), "", &[], &[])
        .await
        .expect("legitimate non-streaming response should be accepted");
}

#[tokio::test]
async fn nonstreaming_chat_rejects_oversized_response_body() {
    use crate::cloud::transport::http_status::MAX_PROVIDER_JSON_RESPONSE_BYTES;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "a".repeat(MAX_PROVIDER_JSON_RESPONSE_BYTES + 1)
                },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let mut provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["test-model".to_string()],
    );
    provider.supports_streaming = false;

    let err = match provider
        .stream(&ModelConfig::new("test-model"), "", &[], &[])
        .await
    {
        Ok(_) => panic!("oversized response should be rejected"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains("response body exceeds"),
        "got: {err}"
    );
}

#[test]
fn cerebras_models_declare_thinking_preservation_and_reasoning_format() {
    let provider = cerebras_provider();

    for (model, expected_format) in [
        ("gpt-oss-120b", ThinkingPreservationFormat::ContentPrepend),
        ("zai-glm-4.7", ThinkingPreservationFormat::ContentXml),
        ("gemma-4-31b", ThinkingPreservationFormat::ContentPrepend),
    ] {
        let declared = provider
            .declared_model(model)
            .unwrap_or_else(|| panic!("{model} should be declared"));

        assert_eq!(declared.thinking_preservation_format, Some(expected_format));

        let reasoning_format = declared
            .request_params
            .as_ref()
            .and_then(|params| params.get("reasoning_format"));
        assert_eq!(
            reasoning_format,
            Some(&json!("parsed")),
            "{model} must request parsed reasoning"
        );
    }

    assert!(provider.declared_model("not-a-cerebras-model").is_none());
}

#[test]
fn cerebras_preserves_thinking_by_default() {
    assert!(cerebras_config().preserves_thinking);
}

#[test]
fn apply_declared_request_params_skips_reserved_keys() {
    let mut payload = json!({
        "model": "zai-glm-4.7",
        "stream": true,
        "stream_options": {"include_usage": true},
        "messages": [{"role": "user", "content": "hi"}]
    });

    let params = HashMap::from([
        ("reasoning_format".to_string(), json!("parsed")),
        ("model".to_string(), json!("hijacked")),
        ("stream".to_string(), json!(false)),
        ("stream_options".to_string(), json!(null)),
        ("messages".to_string(), json!([])),
    ]);

    apply_declared_request_params(&mut payload, &params);

    assert_eq!(payload["reasoning_format"], json!("parsed"));
    assert_eq!(payload["model"], json!("zai-glm-4.7"));
    assert_eq!(payload["stream"], json!(true));
    assert_eq!(payload["stream_options"], json!({"include_usage": true}));
    assert_eq!(payload["messages"].as_array().unwrap().len(), 1);
}
