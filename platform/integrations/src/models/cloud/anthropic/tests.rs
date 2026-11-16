use super::declarative::from_declarative_config;
use super::stream::beta_header_value;
use super::types::AnthropicProvider;
use crate::cloud::transport::api_client::{ApiClient, AuthMethod};
use crate::base::{ModelInfo, Provider, ProviderDescriptor};
use crate::conversation::message::{Message, MessageContent};
use crate::errors::ProviderError;
use crate::cloud::codecs::anthropic::{create_request_for_model, AnthropicFormatOptions};
use crate::model::ModelConfig;
use serde_json::json;

struct StubKeyResolver;

impl crate::cloud::declarative::KeyResolver for StubKeyResolver {
    type Error = std::convert::Infallible;

    fn resolve_key(&self, _key: &str) -> Result<String, Self::Error> {
        Ok("test-key".to_string())
    }
}

#[test]
fn zai_provider_config_emits_clear_thinking() {
    let configs = crate::cloud::declarative::fixed_provider_configs().unwrap();
    let zai = configs.iter().find(|c| c.name == "zai").cloned().unwrap();
    let builder = from_declarative_config(zai, None, StubKeyResolver).unwrap();

    let mut model = ModelConfig::new("glm-4.7");
    model.max_tokens = Some(64_000);
    let messages = vec![
        Message::assistant().with_content(MessageContent::thinking("internal", "")),
        Message::user().with_text("Continue"),
    ];

    let payload = create_request_for_model(
        "zai",
        &model,
        "glm-4.7",
        "system",
        &messages,
        &[],
        builder.format_options,
    )
    .unwrap();

    assert_eq!(payload["thinking"]["clear_thinking"], false);
}

fn make_provider_with_custom_models(
    host: &str,
    custom_models: Vec<String>,
) -> AnthropicProvider {
    AnthropicProvider {
        api_client: ApiClient::new_with_tls(host.to_string(), AuthMethod::NoAuth, None)
            .unwrap(),
        supports_streaming: true,
        name: "test-provider".to_string(),
        custom_models: Some(custom_models.into_iter().map(ModelInfo::new).collect()),
        dynamic_models: Some(true),
        skip_canonical_filtering: false,
        format_options: AnthropicFormatOptions::default(),
    }
}

#[tokio::test]
async fn fetch_models_treats_invalid_json_as_endpoint_not_found() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html>not a models endpoint</html>"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    let err = provider.fetch_models_from_api().await.unwrap_err();
    assert!(
        err.is_endpoint_not_found(),
        "expected EndpointNotFound, got: {:?}",
        err
    );
}

#[tokio::test]
async fn fetch_models_treats_missing_data_field_as_request_failed() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .expect(1)
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    let err = provider.fetch_models_from_api().await.unwrap_err();
    assert!(
        matches!(err, ProviderError::RequestFailed(_)),
        "expected RequestFailed, got: {:?}",
        err
    );
}

#[tokio::test]
async fn fetch_supported_models_falls_back_on_invalid_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>error page</html>"))
        .mount(&server)
        .await;

    let predefined = vec![
        "claude-sonnet-4-5".to_string(),
        "claude-haiku-4-5".to_string(),
    ];
    let provider = make_provider_with_custom_models(&server.uri(), predefined.clone());

    let models = provider
        .fetch_supported_models()
        .await
        .expect("should fall back to predefined list on invalid payload");
    assert_eq!(models, predefined);
}

#[tokio::test]
async fn fetch_supported_models_does_not_fall_back_on_missing_data() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    let err = provider.fetch_supported_models().await.unwrap_err();
    assert!(
        matches!(err, ProviderError::RequestFailed(_)),
        "expected RequestFailed to propagate, got: {:?}",
        err
    );
}

#[tokio::test]
async fn fetch_supported_models_propagates_auth_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "type": "error",
            "error": {
                "type": "authentication_error",
                "message": "invalid api key"
            }
        })))
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    let err = provider.fetch_supported_models().await.unwrap_err();
    assert!(
        matches!(err, ProviderError::Authentication(_)),
        "expected Authentication error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn fetch_supported_models_accepts_null_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "model-a"}],
            "error": null
        })))
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    assert_eq!(
        provider.fetch_supported_models().await.unwrap(),
        vec!["model-a".to_string()]
    );
}

#[tokio::test]
async fn fetch_supported_models_preserves_200_error_type() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "quota exceeded"
            }
        })))
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    assert!(matches!(
        provider.fetch_supported_models().await.unwrap_err(),
        ProviderError::RateLimitExceeded { .. }
    ));
}

#[tokio::test]
async fn fetch_supported_models_propagates_auth_error_from_200_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "error",
            "error": {
                "type": "authentication_error",
                "message": "invalid api key"
            }
        })))
        .mount(&server)
        .await;

    let provider =
        make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

    let err = provider.fetch_supported_models().await.unwrap_err();
    assert!(
        matches!(err, ProviderError::Authentication(_)),
        "expected Authentication error, got: {:?}",
        err
    );
}

#[test]
fn beta_header_merges_client_default_with_binding_beta() {
    let client =
        ApiClient::new_with_tls("http://localhost".to_string(), AuthMethod::NoAuth, None)
            .unwrap()
            .with_header("anthropic-beta", "context-1m-2025-08-07")
            .unwrap();
    let payload = json!({
        "thinking": {"type": "adaptive", "block_binding": {"prefix_mismatch_behavior": "drop_block"}}
    });
    assert_eq!(
        beta_header_value(&client, &ModelConfig::new("claude-opus-5"), &payload).as_deref(),
        Some("context-1m-2025-08-07,thinking-binding-controls-2026-08-01")
    );
}

#[test]
fn metadata_comes_from_the_registry() {
    let metadata = AnthropicProvider::metadata();
    let sonnet = metadata
        .known_models
        .iter()
        .find(|model| model.name == "claude-sonnet-4-5")
        .expect("claude-sonnet-4-5 should come from the catalog");
    assert_eq!(sonnet.context_limit, Some(1_000_000));
    assert!(
        metadata
            .known_models
            .iter()
            .all(|model| !model.name.contains('.')),
        "Anthropic picker ids must be dashed wire names, not dotted catalog names"
    );
}
