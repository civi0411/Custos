use super::helpers::{project_provider, raw_model_config};
use crate::base::Provider;
use crate::cloud::azure_foundry::types::{model_info_for_deployment, ModelPublisher};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn deployment_metadata_enriches_context_without_pricing() {
    let info = model_info_for_deployment("production-chat", "gpt-5");
    assert_eq!(info.name, "production-chat");
    assert_eq!(info.resolved_model.as_deref(), Some("gpt-5"));
    assert_eq!(info.context_limit, Some(400_000));
    assert_eq!(info.input_token_cost, None);
    assert_eq!(info.output_token_cost, None);
}

#[test]
fn gpt_5_6_sol_uses_its_full_context_window() {
    let info = model_info_for_deployment("gpt-5.6-sol", "gpt-5.6-sol");

    assert_eq!(info.context_limit, Some(1_050_000));
    assert!(info.reasoning);
}

#[tokio::test]
async fn deployment_discovery_is_paginated_and_preserves_underlying_model() {
    let server = MockServer::start().await;
    let page_two = format!("{}/api/projects/test/deployments?page=2", server.uri());
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .and(query_param("api-version", "v1"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "partner-prod",
                "modelName": "Mistral-large",
                "modelVersion": "1",
                "modelPublisher": "MistralAI"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .and(query_param("api-version", "v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "openai-prod",
                "modelName": "gpt-5",
                "modelVersion": "1",
                "modelPublisher": "OpenAI"
            }],
            "nextLink": page_two
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    let (names, deployments) = provider.fetch_deployments().await.unwrap();
    assert_eq!(names, vec!["openai-prod", "partner-prod"]);
    assert_eq!(deployments["openai-prod"].model_name, "gpt-5");
    assert_eq!(deployments["openai-prod"].publisher, ModelPublisher::OpenAi);
}

#[tokio::test]
async fn custom_deployment_context_uses_underlying_model() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "production-chat",
                "modelName": "gpt-5",
                "modelVersion": "1",
                "modelPublisher": "OpenAI"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    assert_eq!(
        provider.get_context_limit("production-chat", None).await,
        400_000
    );
}

#[tokio::test]
async fn canonical_like_alias_context_uses_underlying_model() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [{
                "type": "ModelDeployment",
                "name": "gpt-5-high",
                "modelName": "custom-128k-model",
                "modelPublisher": "OpenAI"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    let config = raw_model_config("gpt-5-high");
    assert_eq!(
        provider.get_context_limit(&config.model_name, None).await,
        128_000
    );
}

#[tokio::test]
async fn caller_override_precedes_deployment_metadata() {
    let server = MockServer::start().await;
    let provider = project_provider(&server);
    let config = raw_model_config("gpt-5-high");

    assert_eq!(
        provider
            .get_context_limit(&config.model_name, Some(64_000))
            .await,
        64_000
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn project_inventory_failure_is_propagated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/test/deployments"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {"message": "expired"}
        })))
        .mount(&server)
        .await;

    let provider = project_provider(&server);
    assert!(provider.fetch_supported_models().await.is_err());
    assert!(provider.fetch_supported_model_info().await.is_err());
}
