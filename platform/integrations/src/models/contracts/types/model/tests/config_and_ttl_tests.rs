use std::collections::HashMap;

use super::super::types::ModelConfig;

#[test]
fn cache_ttl_round_trips_through_request_params() {
    let config = ModelConfig::new("claude-sonnet-4-5").with_cache_ttl("1h");
    assert_eq!(config.cache_ttl().as_deref(), Some("1h"));
    assert!(ModelConfig::new("claude-sonnet-4-5").cache_ttl().is_none());
}

#[test]
fn cache_ttl_clamp_resets_one_hour_to_default() {
    let config = ModelConfig::new("claude-sonnet-4-5")
        .with_cache_ttl("1h")
        .with_cache_ttl_clamped();
    assert_eq!(config.cache_ttl().as_deref(), Some("5m"));
}

#[test]
fn without_cache_ttl_removes_the_param_and_empty_map() {
    let config = ModelConfig::new("claude-sonnet-4-5")
        .with_cache_ttl("1h")
        .without_cache_ttl();
    assert!(config.cache_ttl().is_none());
    assert!(config.request_params.is_none());
}

#[test]
fn without_cache_ttl_preserves_other_request_params() {
    let config = ModelConfig::new("claude-sonnet-4-5")
        .with_merged_request_params(HashMap::from([(
            "thinking_effort".to_string(),
            serde_json::json!("high"),
        )]))
        .with_cache_ttl("1h")
        .without_cache_ttl();
    assert!(config.cache_ttl().is_none());
    assert_eq!(
        config.request_param::<String>("thinking_effort").as_deref(),
        Some("high")
    );
}

#[test]
fn cache_ttl_clamp_leaves_unset_ttl_absent() {
    let config = ModelConfig::new("claude-sonnet-4-5").with_cache_ttl_clamped();
    assert!(config.cache_ttl().is_none());
}

#[test]
fn request_headers_never_serialize_into_bodies() {
    let config = ModelConfig::new("test-model").with_request_headers(Some(HashMap::from([(
        "queue_threshold".to_string(),
        "500".to_string(),
    )])));

    let serialized = serde_json::to_value(&config).unwrap();
    assert!(serialized.get("request_headers").is_none());
    assert_eq!(
        config
            .request_headers
            .as_ref()
            .unwrap()
            .get("queue_threshold"),
        Some(&"500".to_string())
    );
}

