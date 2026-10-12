//! Provider Conformance Test Suite
//!
//! Provides reusable conformance verification for any ModelProvider implementation.

use crate::port::ModelProvider;
use crate::request::ProviderRequest;
use crate::turn::ModelTurnRequest;
use crate::types::conversation::message::{Message, MessageContentBlock};
use uuid::Uuid;

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

/// Asserts the compatibility bridge never strips structured tool or multimodal
/// input into the legacy flat prompt. Providers that support structured turns
/// should override `execute_turn`; providers that do not must fail closed.
pub async fn assert_legacy_bridge_rejects_lossy_turn<P: ModelProvider>(provider: &P) {
    let request = ModelTurnRequest {
        task_id: Uuid::new_v4(),
        run_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        connection_id: provider.provider_id().to_string(),
        requested_model: "conformance-model".to_string(),
        structured_messages: vec![Message::new(
            rmcp::model::Role::User,
            chrono::Utc::now().timestamp(),
            vec![MessageContentBlock::text("Use the declared tool")],
        )],
        tool_schemas: vec![crate::tool::ToolDefinition {
            name: "conformance_tool".to_string(),
            description: "Conformance-only tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }],
        required_capabilities: vec!["tools".to_string()],
        source_scope_digest: None,
        privacy_class: "test".to_string(),
        budget_reservation: None,
        deadline: None,
        cancellation_token: None,
        decoding_options: None,
    };

    let error = provider
        .execute_turn(request)
        .await
        .expect_err("legacy bridge must reject a structured turn with tools");
    assert!(
        error
            .to_string()
            .contains("does not implement structured ModelTurn"),
        "unexpected compatibility error: {error}"
    );
}
