//! Provider Conformance Test Suite
//!
//! Provides reusable conformance verification for any ModelProvider implementation.

use crate::port::ModelProvider;
use crate::request::ProviderRequest;

/// Asserts that a provider implementation adheres to the required ModelProvider contracts.
pub async fn assert_provider_conformance<P: ModelProvider>(provider: &P) {
    // 1. Identity assertion
    let id = provider.provider_id();
    assert!(!id.trim().is_empty(), "Provider ID must not be empty");

    // 2. Unary generation assertion
    let req = ProviderRequest::simple("Conformance test prompt");
    let resp = provider
        .generate(&req)
        .await
        .expect("Conformance: generate() must succeed");

    assert!(
        !resp.content.is_empty(),
        "Response content must not be empty"
    );
    assert!(!resp.model_id.is_empty(), "Model ID must not be empty");

    // 3. Streaming generation assertion
    let mut stream = provider
        .stream(&req)
        .await
        .expect("Conformance: stream() must succeed");

    let mut event_count = 0;
    let mut saw_completed = false;

    while let Some(event) = stream.recv().await {
        event_count += 1;
        assert_eq!(
            event.request_id, req.request_id,
            "Event request_id must match request"
        );

        if event.event_type == crate::events::ProviderEventType::Completed {
            saw_completed = true;
        }
    }

    assert!(event_count > 0, "Stream must emit at least one event");
    assert!(saw_completed, "Stream must emit a Completed event");
}
