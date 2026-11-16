use serde_json::json;

use super::builder::from_declarative_config;
use super::options::{apply_ollama_options, OllamaOptions};
use crate::base::{ModelInfo, Provider};
use crate::cloud::declarative::DeclarativeProviderConfig;
use crate::images::ImageFormat;
use crate::model::ModelConfig;

fn ollama_config(
    dynamic_models: Option<bool>,
    models: Vec<ModelInfo>,
) -> DeclarativeProviderConfig {
    ollama_config_with_base_url(dynamic_models, models, "http://localhost:11434")
}

fn ollama_config_with_base_url(
    dynamic_models: Option<bool>,
    models: Vec<ModelInfo>,
    base_url: &str,
) -> DeclarativeProviderConfig {
    DeclarativeProviderConfig {
        name: "test-ollama".to_string(),
        engine: crate::cloud::declarative::ProviderEngine::Ollama,
        display_name: "Test Ollama".to_string(),
        description: None,
        api_key_env: String::new(),
        base_url: base_url.to_string(),
        models,
        headers: None,
        session_id_header_override: None,
        timeout_seconds: None,
        supports_streaming: None,
        requires_auth: false,
        catalog_provider_id: None,
        base_path: None,
        env_vars: None,
        auth: None,
        dynamic_models,
        skip_canonical_filtering: false,
        model_doc_link: None,
        setup_steps: vec![],
        toolshim: false,
        preserves_thinking: false,
        emit_clear_thinking: false,
        setup: None,
    }
}

#[tokio::test]
async fn fetch_supported_models_uses_static_models_when_dynamic_models_false() {
    let provider = from_declarative_config(
        ollama_config(
            Some(false),
            vec![ModelInfo::new("static-model").with_context_limit(4096)],
        ),
        None,
        crate::cloud::declarative::EnvKeyResolver,
    )
    .unwrap()
    .build();

    assert_eq!(
        provider.fetch_supported_models().await.unwrap(),
        vec!["static-model".to_string()]
    );
}

#[test]
fn from_custom_config_requires_static_models_when_dynamic_models_false() {
    let err = from_declarative_config(
        ollama_config(Some(false), vec![]),
        None,
        crate::cloud::declarative::EnvKeyResolver,
    )
    .err()
    .expect("expected static models validation error");

    assert!(err
        .to_string()
        .contains("dynamic_models: false but no static models listed"));
}

#[tokio::test]
async fn fetch_supported_models_falls_back_to_static_models_on_404() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/tags"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let provider = from_declarative_config(
        ollama_config_with_base_url(
            None,
            vec![ModelInfo::new("static-model").with_context_limit(4096)],
            &server.uri(),
        ),
        None,
        crate::cloud::declarative::EnvKeyResolver,
    )
    .unwrap()
    .build();

    assert_eq!(
        provider.fetch_supported_models().await.unwrap(),
        vec!["static-model".to_string()]
    );
}

#[test]
fn test_apply_ollama_options_uses_input_limit() {
    let options = OllamaOptions {
        input_limit: Some(8192),
        ..Default::default()
    };
    let model_config = ModelConfig::new("qwen3").with_context_limit(Some(16_000));
    let mut payload = json!({});
    apply_ollama_options(&mut payload, &options, &model_config);
    assert_eq!(payload["options"]["num_ctx"], 8192);
}

#[test]
fn test_apply_ollama_options_ignores_context_management_limit() {
    let options = OllamaOptions::default();
    let model_config = ModelConfig::new("qwen3").with_context_limit(Some(12_000));
    let mut payload = json!({});
    apply_ollama_options(&mut payload, &options, &model_config);
    assert!(payload.get("options").is_none());
}

#[test]
fn test_raw_create_request_contains_unsupported_ollama_fields() {
    use crate::cloud::codecs::ollama::create_request;

    let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
    let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

    let payload = create_request(
        &model_config,
        "You are a helpful assistant.",
        &messages,
        &[],
        &ImageFormat::OpenAi,
        true,
    )
    .unwrap();

    assert!(
        payload.get("stream_options").is_some(),
        "create_request should produce stream_options for usage tracking"
    );
    assert!(
        payload.get("max_tokens").is_some(),
        "create_request should produce max_tokens (unsupported by Ollama)"
    );
}

#[test]
fn test_apply_ollama_options_preserves_stream_options_by_default() {
    use crate::cloud::codecs::ollama::create_request;

    let options = OllamaOptions::default();
    let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
    let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

    let mut payload = create_request(
        &model_config,
        "You are a helpful assistant.",
        &messages,
        &[],
        &ImageFormat::OpenAi,
        true,
    )
    .unwrap();

    apply_ollama_options(&mut payload, &options, &model_config);

    assert!(
        payload.get("stream_options").is_some(),
        "stream_options should be preserved by default for usage tracking"
    );
    assert!(
        payload.get("max_tokens").is_none(),
        "max_tokens should be removed for Ollama"
    );
    assert!(
        payload.get("max_completion_tokens").is_none(),
        "max_completion_tokens should be removed for Ollama"
    );
    assert_eq!(
        payload["options"]["num_predict"], 4096,
        "max_tokens should be moved to options.num_predict"
    );
    assert_eq!(payload["stream"], true, "stream field should be preserved");
}

#[test]
fn test_apply_ollama_options_strips_stream_options_when_disabled() {
    use crate::cloud::codecs::ollama::create_request;

    let options = OllamaOptions {
        input_limit: None,
        stream_usage: false,
        chunk_timeout_secs: 120,
    };
    let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
    let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

    let mut payload = create_request(
        &model_config,
        "You are a helpful assistant.",
        &messages,
        &[],
        &ImageFormat::OpenAi,
        true,
    )
    .unwrap();

    apply_ollama_options(&mut payload, &options, &model_config);

    assert!(
        payload.get("stream_options").is_none(),
        "stream_options should be removed when OLLAMA_STREAM_USAGE=false"
    );
}
