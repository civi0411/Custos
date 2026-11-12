use crate::cloud::transport::api_client::{ApiClient, AuthMethod};
use crate::base::{Provider, ProviderDescriptor};
use crate::cloud::openrouter::provider::{
    merge_openrouter_parameters, OpenRouterProvider, OPENROUTER_PARAMETERS_CONFIG_KEY,
};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use serde_json::json;
use std::collections::HashMap;

fn model_config(model_name: &str) -> ModelConfig {
    ModelConfig {
        model_name: model_name.to_string(),
        context_limit: None,
        temperature: None,
        max_tokens: None,
        toolshim: false,
        toolshim_model: None,
        request_params: None,
        reasoning: None,
        supports_vision: None,
        request_headers: None,
    }
}

#[test]
fn metadata_includes_openrouter_parameters_config_key() {
    let metadata = OpenRouterProvider::metadata();

    assert!(metadata
        .config_keys
        .iter()
        .any(|key| key.name == OPENROUTER_PARAMETERS_CONFIG_KEY));
}

#[test]
fn merge_openrouter_parameters_updates_model_request_params() {
    let mut model = model_config("anthropic/claude-sonnet-4");
    model.request_params = Some(HashMap::from([("verbosity".to_string(), json!("low"))]));
    let params = HashMap::from([
        ("plugins".to_string(), json!([{ "id": "web" }])),
        ("verbosity".to_string(), json!("xhigh")),
    ]);

    merge_openrouter_parameters(&mut model, params);

    let request_params = model.request_params.as_ref().unwrap();
    assert_eq!(request_params["plugins"], json!([{ "id": "web" }]));
    assert_eq!(request_params["verbosity"], json!("xhigh"));
}

#[tokio::test]
async fn stream_downgrades_reasoning_disable_on_mandatory_endpoint() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .and(body_partial_json(
            json!({ "reasoning": { "enabled": false } }),
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": { "message": "Reasoning is mandatory for this endpoint and cannot be disabled." }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .and(body_partial_json(
            json!({ "reasoning": { "effort": "low" } }),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: [DONE]\n\n"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenRouterProvider::new(
        ApiClient::new_with_tls(
            server.uri(),
            AuthMethod::BearerToken("test-key".to_string()),
            None,
        )
        .unwrap(),
        None,
        None,
    );

    let mut config = model_config("google/gemini-3.5-flash");
    config.reasoning = Some(true);
    config.request_params = Some(HashMap::from([(
        "thinking_effort".to_string(),
        json!("off"),
    )]));

    let _stream = provider
        .stream(&config, "system", &[Message::user().with_text("hi")], &[])
        .await
        .unwrap();
}
