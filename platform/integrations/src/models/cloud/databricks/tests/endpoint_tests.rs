use std::time::Instant;

use serde_json::json;

use crate::cloud::databricks::types::{
    CachedDatabricksEndpointInfo, DatabricksEndpointInfo, DatabricksProvider,
    EndpointMetadataLookup,
};
use crate::cloud::codecs::databricks::DATABRICKS_PROVIDER_NAME;
use crate::model::ModelConfig;

#[test]
fn routing_retries_cached_metadata_failures() {
    let cached = CachedDatabricksEndpointInfo {
        info: None,
        fetched_at: Instant::now(),
    };

    assert!(cached.applies_to(EndpointMetadataLookup::ContextDiscovery));
    assert!(!cached.applies_to(EndpointMetadataLookup::InferenceRouting));
}

#[test]
fn routing_reuses_cached_metadata_successes() {
    let cached = CachedDatabricksEndpointInfo {
        info: Some(DatabricksEndpointInfo {
            name: "production-chat".to_string(),
            upstream_model_name: Some("gpt-5".to_string()),
            upstream_model_provider: Some("openai".to_string()),
            reasoning: Some(true),
            supports_responses_api: true,
        }),
        fetched_at: Instant::now(),
    };

    assert!(cached.applies_to(EndpointMetadataLookup::ContextDiscovery));
    assert!(cached.applies_to(EndpointMetadataLookup::InferenceRouting));
}

#[test]
fn endpoint_metadata_marks_reasoning_alias_from_external_model() {
    let endpoint = json!({
        "name": "custos-engine",
        "config": {
            "served_entities": [{
                "name": "current",
                "external_model": {
                    "name": "claude-opus-4.6",
                    "provider": "anthropic",
                    "task": "llm/v1/chat"
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert_eq!(info.name, "custos-engine");
    assert_eq!(info.upstream_model_name.as_deref(), Some("claude-opus-4.6"));
    assert_eq!(info.reasoning, Some(true));
    assert!(!info.supports_responses_api);

    let model_info = DatabricksProvider::model_info_from_endpoint(info);
    assert_eq!(model_info.name, "custos-engine");
    assert_eq!(
        model_info.resolved_model.as_deref(),
        Some("claude-opus-4.6")
    );
    assert!(model_info.reasoning);
}

#[test]
fn endpoint_metadata_captures_databricks_model_serving_hop() {
    let endpoint = json!({
        "name": "custos-engine",
        "config": {
            "served_entities": [{
                "external_model": {
                    "name": "databricks-claude-opus-4-6",
                    "provider": "databricks-model-serving",
                    "task": "llm/v1/chat"
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert_eq!(info.name, "custos-engine");
    assert_eq!(
        info.upstream_model_name.as_deref(),
        Some("databricks-claude-opus-4-6")
    );
    assert_eq!(
        info.upstream_model_provider.as_deref(),
        Some("databricks-model-serving")
    );
    assert_eq!(info.reasoning, Some(true));
}

#[test]
fn endpoint_metadata_marks_reasoning_alias_from_pending_gpt_model() {
    let endpoint = json!({
        "name": "custos-engine",
        "pending_config": {
            "served_entities": [{
                "external_model": {
                    "name": "gpt-5.5",
                    "provider": "openai",
                    "task": "llm/v1/chat"
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert_eq!(info.name, "custos-engine");
    assert_eq!(info.upstream_model_name.as_deref(), Some("gpt-5.5"));
    assert_eq!(info.reasoning, Some(true));
}

#[test]
fn endpoint_metadata_uses_endpoint_name_when_no_upstream_model_exists() {
    let endpoint = json!({
        "name": "goose-gpt-5-5"
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert_eq!(info.name, "goose-gpt-5-5");
    assert_eq!(info.upstream_model_name, None);
    assert_eq!(info.reasoning, Some(true));
    assert!(!info.supports_responses_api);
}

#[test]
fn endpoint_metadata_detects_responses_api_from_foundation_model_api_types() {
    let endpoint = json!({
        "name": "databricks-gpt-5-4",
        "config": {
            "served_entities": [{
                "name": "databricks-gpt-5-4",
                "entity_name": "system.ai.databricks-gpt-5-4",
                "type": "FOUNDATION_MODEL",
                "foundation_model": {
                    "name": "system.ai.databricks-gpt-5-4",
                    "display_name": "GPT-5.4",
                    "api_types": [
                        "mlflow/v1/chat/completions",
                        "openai/v1/responses",
                        "cursor/v1/chat/completions"
                    ]
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert_eq!(info.name, "databricks-gpt-5-4");
    assert_eq!(
        info.upstream_model_name.as_deref(),
        Some("system.ai.databricks-gpt-5-4")
    );
    assert!(info.supports_responses_api);
}

#[test]
fn endpoint_metadata_detects_responses_api_from_served_models() {
    let endpoint = json!({
        "name": "databricks-gpt-5-4",
        "config": {
            "served_models": [{
                "foundation_model": {
                    "api_types": ["openai/v1/responses"]
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert!(info.supports_responses_api);
}

#[test]
fn endpoint_metadata_ignores_pending_config_for_responses_routing() {
    let endpoint = json!({
        "name": "databricks-gpt-5-4",
        "config": {
            "served_entities": [{
                "foundation_model": {
                    "api_types": ["mlflow/v1/chat/completions"]
                }
            }]
        },
        "pending_config": {
            "served_entities": [{
                "foundation_model": {
                    "api_types": ["openai/v1/responses"]
                }
            }]
        }
    });

    let info = DatabricksProvider::endpoint_info_from_value(&endpoint).unwrap();

    assert!(!info.supports_responses_api);
}

#[test]
fn vision_support_resolves_from_upstream_model_behind_endpoint_alias() {
    let aliased = ModelConfig::new("production-chat");
    assert_eq!(aliased.supports_vision, None);

    let resolved = DatabricksProvider::resolve_vision_support(&aliased, "gpt-4o")
        .expect("upstream model should resolve vision support");
    assert_eq!(resolved.supports_vision, Some(true));
    assert_eq!(resolved.model_name, "production-chat");

    let catalog_alias = ModelConfig::new("gpt-3.5-turbo")
        .with_canonical_vision_support(DATABRICKS_PROVIDER_NAME);
    assert_eq!(catalog_alias.supports_vision, Some(false));

    let corrected = DatabricksProvider::resolve_vision_support(&catalog_alias, "gpt-4o")
        .expect("upstream model should override alias-derived capability");
    assert_eq!(corrected.supports_vision, Some(true));

    let downgraded =
        DatabricksProvider::resolve_vision_support(&resolved, "gpt-3.5-turbo").unwrap();
    assert_eq!(downgraded.supports_vision, Some(false));
}

#[test]
fn responses_routing_prefers_metadata_over_model_name() {
    let responses_info = DatabricksEndpointInfo {
        name: "custom".into(),
        upstream_model_name: None,
        upstream_model_provider: None,
        reasoning: None,
        supports_responses_api: true,
    };
    assert!(DatabricksProvider::uses_responses_api(
        Some(&responses_info),
        &["databricks-claude-sonnet-4"]
    ));

    let chat_info = DatabricksEndpointInfo {
        supports_responses_api: false,
        ..responses_info
    };
    assert!(!DatabricksProvider::uses_responses_api(
        Some(&chat_info),
        &["gpt-5.4"]
    ));
}

#[test]
fn responses_routing_falls_back_to_model_name_without_metadata() {
    assert!(DatabricksProvider::uses_responses_api(None, &["gpt-5.4"]));
    assert!(DatabricksProvider::uses_responses_api(
        None,
        &["databricks-claude-sonnet-4", "gpt-5.4"]
    ));
    assert!(!DatabricksProvider::uses_responses_api(
        None,
        &["databricks-claude-sonnet-4"]
    ));
}

