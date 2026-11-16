use super::helpers::{
    anthropic_stream, chat_stream, project_provider, raw_model_config, responses_stream,
};
use crate::cloud::transport::api_client::AuthMethod;
use crate::base::Provider;
use crate::cloud::azure_foundry::provider::AzureFoundryProvider;
use crate::model::ModelConfig;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn custom_openai_deployment_uses_alias_and_underlying_capabilities() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "production-chat",
                "modelName": "gpt-5",
                "modelPublisher": "OpenAI"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/projects/test/openai/v1/responses"))
        .and(body_partial_json(json!({"model": "production-chat"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    let config = ModelConfig::new("production-chat").with_temperature(Some(0.7));
    provider
        .complete(&config, "system", &[], &[])
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    let payload: serde_json::Value = requests
        .iter()
        .find(|request| request.url.path().ends_with("/responses"))
        .unwrap()
        .body_json()
        .unwrap();
    assert_eq!(payload["model"], "production-chat");
    assert!(payload.get("temperature").is_none());
    assert!(payload.get("capability_model").is_none());
    assert!(config.request_params.is_none());
    assert!(serde_json::to_value(&config)
        .unwrap()
        .get("capability_model")
        .is_none());
}

#[tokio::test]
async fn suffixed_deployment_without_metadata_is_preserved_on_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value": []})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/projects/test/openai/v1/responses"))
        .and(body_partial_json(json!({"model": "gpt-5-high"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    assert_eq!(
        provider.get_context_limit("gpt-5-high", None).await,
        400_000
    );
    provider
        .complete(&raw_model_config("gpt-5-high"), "system", &[], &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn suffixed_deployment_alias_is_preserved_on_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "gpt-5-high",
                "modelName": "gpt-5",
                "modelPublisher": "OpenAI"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/projects/test/openai/v1/responses"))
        .and(body_partial_json(json!({"model": "gpt-5-high"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    let config = raw_model_config("gpt-5-high")
        .with_thinking_effort(crate::thinking::ThinkingEffort::Off);
    assert_eq!(config.model_name, "gpt-5-high");
    provider
        .complete(&config, "system", &[], &[])
        .await
        .unwrap();
}

#[tokio::test]
async fn anthropic_alias_uses_underlying_output_limit_and_preserves_override() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "claude-prod",
                "modelName": "claude-sonnet-4-6",
                "modelPublisher": "Anthropic"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    provider
        .complete(&ModelConfig::new("claude-prod"), "system", &[], &[])
        .await
        .unwrap();
    provider
        .complete(
            &ModelConfig::new("claude-prod").with_max_tokens(Some(12_345)),
            "system",
            &[],
            &[],
        )
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let max_tokens: Vec<i64> = requests
        .iter()
        .filter(|request| request.url.path().ends_with("/messages"))
        .map(|request| {
            request.body_json::<serde_json::Value>().unwrap()["max_tokens"]
                .as_i64()
                .unwrap()
        })
        .collect();
    assert_eq!(max_tokens, vec![128_000, 12_345]);
}

#[tokio::test]
async fn canonical_claude_deployment_uses_canonical_output_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "claude-sonnet-4-6",
                "modelName": "claude-sonnet-4-6",
                "modelPublisher": "Anthropic"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    provider
        .complete(&raw_model_config("claude-sonnet-4-6"), "system", &[], &[])
        .await
        .unwrap();

    let request = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| request.url.path().ends_with("/messages"))
        .unwrap();
    let payload = request.body_json::<serde_json::Value>().unwrap();
    assert_eq!(payload["max_tokens"], 128_000);
}

#[tokio::test]
async fn maas_uses_v1_chat_completions_path_and_bound_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = AzureFoundryProvider::create(
        server.uri(),
        None,
        Some("bound-model".to_string()),
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        AuthMethod::NoAuth,
        None,
        None,
    )
    .unwrap();
    let config = raw_model_config("wrong-model");
    provider
        .complete(&config, "system", &[], &[])
        .await
        .unwrap();
    let request = server.received_requests().await.unwrap().pop().unwrap();
    let payload = request.body_json::<serde_json::Value>().unwrap();
    assert_eq!(payload["model"], "bound-model");
    assert!(payload.get("azure_foundry_deployment").is_none());
    assert_eq!(
        provider.fetch_supported_models().await.unwrap(),
        vec!["bound-model"]
    );
}

#[tokio::test]
async fn custom_deployment_names_route_to_all_three_surfaces() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [
                {"type":"ModelDeployment","name":"openai-prod","modelName":"gpt-5","modelVersion":"1","modelPublisher":"OpenAI"},
                {"type":"ModelDeployment","name":"claude-prod","modelName":"claude-sonnet-4-6","modelVersion":"1","modelPublisher":"Anthropic"},
                {"type":"ModelDeployment","name":"partner-prod","modelName":"Mistral-large","modelVersion":"1","modelPublisher":"MistralAI"}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/projects/test/openai/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/projects/test/openai/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_stream())
                .append_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    for deployment in ["openai-prod", "claude-prod", "partner-prod"] {
        provider
            .complete(&ModelConfig::new(deployment), "system", &[], &[])
            .await
            .unwrap();
    }
}
