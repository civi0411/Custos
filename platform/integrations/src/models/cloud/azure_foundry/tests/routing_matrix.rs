use crate::cloud::transport::api_client::AuthMethod;
use crate::cloud::azure_foundry::types::{
    configured_client, endpoint_kind, inference_route, is_project_endpoint, with_api_version,
    DeploymentCache, DeploymentMetadataLookup, EndpointKind, InferenceRoute, ModelPublisher,
};
use std::time::Instant;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn routing_retries_cached_deployment_failures() {
    let cache = DeploymentCache {
        fetched_at: Some(Instant::now()),
        fetch_failed: true,
        ..Default::default()
    };

    assert!(cache.applies_to(DeploymentMetadataLookup::ContextDiscovery));
    assert!(!cache.applies_to(DeploymentMetadataLookup::InferenceRouting));
}

#[test]
fn routing_reuses_successful_empty_deployment_inventory() {
    let cache = DeploymentCache {
        fetched_at: Some(Instant::now()),
        fetch_failed: false,
        ..Default::default()
    };

    assert!(cache.applies_to(DeploymentMetadataLookup::ContextDiscovery));
    assert!(cache.applies_to(DeploymentMetadataLookup::InferenceRouting));
}

#[test]
fn routing_matrix_uses_endpoint_publisher_and_underlying_model() {
    use InferenceRoute::*;
    assert_eq!(
        inference_route(false, ModelPublisher::OpenAi, "gpt-5"),
        MaasChatCompletions
    );
    assert_eq!(
        inference_route(false, ModelPublisher::Anthropic, "claude-sonnet-4-6"),
        MaasChatCompletions
    );
    assert_eq!(
        inference_route(true, ModelPublisher::OpenAi, "gpt-5"),
        ProjectResponses
    );
    assert_eq!(
        inference_route(true, ModelPublisher::OpenAi, "o3-mini"),
        ProjectResponses
    );
    assert_eq!(
        inference_route(true, ModelPublisher::OpenAi, "gpt-4o"),
        ProjectChatCompletions
    );
    assert_eq!(
        inference_route(true, ModelPublisher::Partner, "gpt-5"),
        ProjectChatCompletions
    );
    assert_eq!(
        inference_route(true, ModelPublisher::Anthropic, "gpt-5"),
        AnthropicMessages
    );
}

#[test]
fn resource_endpoint_routes_declared_models_to_native_surfaces() {
    let resource = endpoint_kind("https://hub.services.ai.azure.com");
    let native_inference = resource != EndpointKind::Maas;

    assert_eq!(
        inference_route(
            native_inference,
            ModelPublisher::from_model_name("gpt-5.6-sol"),
            "gpt-5.6-sol",
        ),
        InferenceRoute::ProjectResponses
    );
    assert_eq!(
        inference_route(
            native_inference,
            ModelPublisher::from_model_name("claude-sonnet-4-6"),
            "claude-sonnet-4-6",
        ),
        InferenceRoute::AnthropicMessages
    );
    assert_eq!(
        inference_route(
            native_inference,
            ModelPublisher::from_model_name("Mistral-large"),
            "Mistral-large",
        ),
        InferenceRoute::ProjectChatCompletions
    );
}

#[test]
fn model_fallback_only_routes_known_native_families() {
    assert_eq!(
        ModelPublisher::from_model_name("gpt-5"),
        ModelPublisher::OpenAi
    );
    assert_eq!(
        ModelPublisher::from_model_name("o3-mini"),
        ModelPublisher::OpenAi
    );
    assert_eq!(
        ModelPublisher::from_model_name("claude-sonnet-4-6"),
        ModelPublisher::Anthropic
    );
    assert_eq!(
        ModelPublisher::from_model_name("Mistral-large"),
        ModelPublisher::Partner
    );
    assert_eq!(
        ModelPublisher::from_model_name("glm-4.7"),
        ModelPublisher::Partner
    );
    assert_eq!(
        ModelPublisher::from_model_name("Kimi-K2"),
        ModelPublisher::Partner
    );
}

#[test]
fn endpoint_type_is_detected() {
    assert_eq!(
        endpoint_kind("https://hub.services.ai.azure.com/api/projects/project"),
        EndpointKind::Project
    );
    assert_eq!(
        endpoint_kind("https://hub.services.ai.azure.com"),
        EndpointKind::Resource
    );
    assert_eq!(
        endpoint_kind("https://deployment.eastus.models.ai.azure.com"),
        EndpointKind::Maas
    );
    assert!(is_project_endpoint(
        "https://hub.services.ai.azure.com/api/projects/project"
    ));
    assert!(!is_project_endpoint("https://hub.services.ai.azure.com"));
}

#[test]
fn pagination_link_keeps_api_version() {
    assert_eq!(
        with_api_version("https://example.test/deployments?page=2", Some("v1")),
        "https://example.test/deployments?page=2&api-version=v1"
    );
    assert_eq!(
        with_api_version(
            "https://example.test/deployments?page=2&api-version=2025-05-01",
            Some("v1")
        ),
        "https://example.test/deployments?page=2&api-version=2025-05-01"
    );
}

#[tokio::test]
async fn foundry_api_keys_do_not_follow_cross_origin_redirects() {
    for header_name in ["api-key", "x-api-key"] {
        let destination = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/capture"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&destination)
            .await;

        let source = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/redirect"))
            .and(header(header_name, "foundry-secret"))
            .respond_with(
                ResponseTemplate::new(307)
                    .append_header("location", format!("{}/capture", destination.uri())),
            )
            .expect(1)
            .mount(&source)
            .await;

        let client = configured_client(
            source.uri(),
            AuthMethod::ApiKey {
                header_name: header_name.to_string(),
                key: "foundry-secret".to_string(),
            },
            None,
            None,
        )
        .unwrap();

        assert!(client.response_get("redirect").await.is_err());
        assert!(destination.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn foundry_api_keys_follow_same_origin_redirects() {
    for header_name in ["api-key", "x-api-key"] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/redirect"))
            .respond_with(ResponseTemplate::new(307).append_header("location", "/final"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/final"))
            .and(header(header_name, "foundry-secret"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let client = configured_client(
            server.uri(),
            AuthMethod::ApiKey {
                header_name: header_name.to_string(),
                key: "foundry-secret".to_string(),
            },
            None,
            None,
        )
        .unwrap();

        assert!(client
            .response_get("redirect")
            .await
            .unwrap()
            .status()
            .is_success());
    }
}

#[tokio::test]
async fn foundry_bearer_redirects_keep_reqwest_authorization_behavior() {
    let destination = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/capture"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&destination)
        .await;

    let source = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/redirect"))
        .and(header("authorization", "Bearer foundry-token"))
        .respond_with(
            ResponseTemplate::new(307)
                .append_header("location", format!("{}/capture", destination.uri())),
        )
        .expect(1)
        .mount(&source)
        .await;

    let client = configured_client(
        source.uri(),
        AuthMethod::BearerToken("foundry-token".to_string()),
        None,
        None,
    )
    .unwrap();

    assert!(client
        .response_get("redirect")
        .await
        .unwrap()
        .status()
        .is_success());
    let requests = destination.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.get("authorization").is_none());
}
