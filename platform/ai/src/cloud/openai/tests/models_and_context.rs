use super::helpers::make_provider_with_custom_models;
use crate::base::Provider;
use crate::cloud::openai::context_limit::{
    parse_model_ids, parse_n_ctx_from_models, CachedContextLimit, N_CTX_FAILURE_TTL,
};
use crate::errors::ProviderError;
use serde_json::json;
use std::time::Instant;

#[test]
fn parse_model_ids_accepts_openai_response() {
    let response = json!({"data": [{"id": "model-b"}, {"id": "model-a"}]});

    assert_eq!(parse_model_ids(&response).unwrap(), ["model-a", "model-b"]);
}

#[test]
fn parse_model_ids_accepts_together_response() {
    let response = json!([
        {"id": "meta-llama/Llama-3.3-70B-Instruct-Turbo", "type": "chat"},
        {"id": "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8", "type": "code"}
    ]);

    assert_eq!(
        parse_model_ids(&response).unwrap(),
        [
            "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8",
            "meta-llama/Llama-3.3-70B-Instruct-Turbo"
        ]
    );
}

#[test]
fn parse_model_ids_rejects_unknown_response() {
    let response = json!({"models": []});

    assert!(parse_model_ids(&response).is_err());
}

#[test]
fn parse_n_ctx_falls_back_to_sole_entry_when_id_differs() {
    let body = json!({
        "data": [
            { "id": "/models/qwen3.gguf", "meta": { "n_ctx": 32768 } }
        ]
    });
    assert_eq!(parse_n_ctx_from_models(&body, "qwen3"), Some(32768));
}

#[test]
fn parse_n_ctx_no_fallback_with_multiple_unmatched_entries() {
    let body = json!({
        "data": [
            { "id": "model-a", "meta": { "n_ctx": 4096 } },
            { "id": "model-b", "meta": { "n_ctx": 8192 } }
        ]
    });
    assert_eq!(parse_n_ctx_from_models(&body, "model-c"), None);
}

#[tokio::test]
async fn context_limit_caches_failed_models_probe() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let mut provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["other-model".to_string()],
    );
    provider.custom_models = None;

    for _ in 0..2 {
        assert_eq!(
            provider.get_context_limit("unknown-model", None).await,
            crate::model::DEFAULT_CONTEXT_LIMIT
        );
    }
}

#[tokio::test]
async fn context_limit_retries_expired_models_probe_failure() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let mut provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["other-model".to_string()],
    );
    provider.custom_models = None;
    provider.n_ctx_cache.lock().unwrap().insert(
        "unknown-model".to_string(),
        CachedContextLimit::Failure(Instant::now() - N_CTX_FAILURE_TTL),
    );

    assert_eq!(
        provider.get_context_limit("unknown-model", None).await,
        crate::model::DEFAULT_CONTEXT_LIMIT
    );
}

#[tokio::test]
async fn context_limit_caches_unsupported_models_endpoint() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;

    let mut provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["other-model".to_string()],
    );
    provider.custom_models = None;

    assert_eq!(
        provider.get_context_limit("unknown-model", None).await,
        crate::model::DEFAULT_CONTEXT_LIMIT
    );
    assert_eq!(
        provider.get_context_limit("unknown-model", None).await,
        crate::model::DEFAULT_CONTEXT_LIMIT
    );
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

    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["static-model".to_string()],
    );

    let err = provider.fetch_models_from_api().await.unwrap_err();
    assert!(err.is_endpoint_not_found(), "got: {:?}", err);
}

#[tokio::test]
async fn fetch_models_returns_request_failed_for_missing_data_field() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["static-model".to_string()],
    );

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

    let predefined = vec!["glm-4.5".to_string(), "glm-5".to_string()];
    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        predefined.clone(),
    );

    let models = provider
        .fetch_supported_models()
        .await
        .expect("should fall back");
    assert_eq!(models, predefined);
}

#[tokio::test]
async fn fetch_supported_models_propagates_auth_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": {"message": "invalid api key"}
        })))
        .mount(&server)
        .await;

    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["static-model".to_string()],
    );

    let err = provider.fetch_supported_models().await.unwrap_err();
    assert!(
        matches!(err, ProviderError::Authentication(_)),
        "got: {:?}",
        err
    );
}

#[tokio::test]
async fn fetch_supported_models_does_not_reclassify_400_as_endpoint_not_found() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": {"message": "request is not valid JSON"}
        })))
        .mount(&server)
        .await;

    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["static-model".to_string()],
    );

    let err = provider.fetch_supported_models().await.unwrap_err();
    assert!(!err.is_endpoint_not_found(), "got: {:?}", err);
}

#[tokio::test]
async fn fetch_supported_models_accepts_payload_with_extra_fields() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "model-a"}, {"id": "model-b"}],
            "message": "ok",
            "error": null
        })))
        .mount(&server)
        .await;

    let provider = make_provider_with_custom_models(
        &server.uri(),
        "v1/chat/completions",
        vec!["static-model".to_string()],
    );

    let models = provider.fetch_supported_models().await.unwrap();
    assert_eq!(models, vec!["model-a".to_string(), "model-b".to_string()]);
}
