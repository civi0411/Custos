use std::collections::HashMap;

use serde_json::Value;

use crate::thinking::ThinkingEffort;

use super::super::types::ModelConfig;

fn config_with_params(model_name: &str, params: HashMap<String, Value>) -> ModelConfig {
    ModelConfig::new(model_name).with_merged_request_params(params)
}

#[test]
fn from_request_params() {
    let mut params = HashMap::new();
    params.insert("thinking_effort".to_string(), serde_json::json!("medium"));
    let config = config_with_params("test", params);
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Medium));
}

#[test]
fn with_thinking_effort_sets_request_param() {
    let config = ModelConfig::new("test").with_thinking_effort(ThinkingEffort::High);

    assert_eq!(
        config
            .request_params
            .as_ref()
            .and_then(|params| params.get("thinking_effort")),
        Some(&serde_json::json!("high"))
    );
}

#[test]
fn with_default_thinking_effort_preserves_unparseable_raw_param() {
    let config = config_with_params(
        "test",
        HashMap::from([("thinking_effort".to_string(), serde_json::json!("default"))]),
    )
    .with_default_thinking_effort(Some(ThinkingEffort::High));

    assert_eq!(
        config
            .request_params
            .as_ref()
            .and_then(|params| params.get("thinking_effort")),
        Some(&serde_json::json!("default"))
    );
}

#[test]
fn with_default_thinking_effort_applies_when_absent() {
    let config =
        ModelConfig::new("test").with_default_thinking_effort(Some(ThinkingEffort::High));

    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
}

#[test]
fn preserves_explicit_thinking_effort() {
    let previous = config_with_params(
        "previous",
        HashMap::from([("thinking_effort".to_string(), serde_json::json!("high"))]),
    );
    let config = ModelConfig::new("next")
        .with_inherited_session_settings_from(Some(&previous), None);

    assert_eq!(
        config
            .request_params
            .as_ref()
            .and_then(|params| params.get("thinking_effort")),
        Some(&serde_json::json!("high"))
    );
}

#[test]
fn does_not_override_existing_thinking_effort() {
    let previous = config_with_params(
        "previous",
        HashMap::from([("thinking_effort".to_string(), serde_json::json!("high"))]),
    );
    let config = config_with_params(
        "next",
        HashMap::from([("thinking_effort".to_string(), serde_json::json!("low"))]),
    )
    .with_inherited_session_settings_from(Some(&previous), None);

    assert_eq!(
        config
            .request_params
            .as_ref()
            .and_then(|params| params.get("thinking_effort")),
        Some(&serde_json::json!("low"))
    );
}

#[test]
fn inherits_reasoning_controls_but_not_provider_specific_params() {
    let previous = config_with_params(
        "previous",
        HashMap::from([
            ("budget_tokens".to_string(), serde_json::json!(8192)),
            (
                "preserve_thinking_context".to_string(),
                serde_json::json!(true),
            ),
            ("anthropic_beta".to_string(), serde_json::json!("beta")),
        ]),
    );
    let config = ModelConfig::new("next")
        .with_inherited_session_settings_from(Some(&previous), None);

    let params = config.request_params.expect("reasoning controls inherited");
    assert_eq!(params.get("budget_tokens"), Some(&serde_json::json!(8192)));
    assert_eq!(
        params.get("preserve_thinking_context"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(params.get("anthropic_beta"), None);
}

#[test]
fn explicit_request_params_override_preserved_session_settings() {
    let previous = config_with_params(
        "previous",
        HashMap::from([("thinking_effort".to_string(), serde_json::json!("high"))]),
    );
    let config = ModelConfig::new("next").with_inherited_session_settings_from(
        Some(&previous),
        Some(HashMap::from([(
            "thinking_effort".to_string(),
            serde_json::json!("low"),
        )])),
    );

    assert_eq!(
        config
            .request_params
            .as_ref()
            .and_then(|params| params.get("thinking_effort")),
        Some(&serde_json::json!("low"))
    );
}

#[test]
fn effort_suffix_stripped_from_model_name() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("o3-mini-high");
    assert_eq!(config.model_name, "o3-mini");
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
}

#[test]
fn none_suffix_stripped_from_model_name() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", Some("high")),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("o3-mini-none");
    assert_eq!(config.model_name, "o3-mini");
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Off));
}

#[test]
fn xhigh_suffix_stripped_from_model_name() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", Some("low")),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("gpt-5.4-xhigh");
    assert_eq!(config.model_name, "gpt-5.4");
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Max));
}

#[test]
fn effort_suffix_not_stripped_when_thinking_effort_set() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let mut params = HashMap::new();
    params.insert("thinking_effort".to_string(), serde_json::json!("low"));
    let mut config = ModelConfig::new("o3-mini-high");
    assert_eq!(config.model_name, "o3-mini");

    config.request_params = Some(params);
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Low));
}

#[test]
fn no_suffix_no_change() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("o3-mini");
    assert_eq!(config.model_name, "o3-mini");
}

#[test]
fn non_reasoning_model_suffix_not_stripped() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("claude-sonnet-4-high");
    assert_eq!(config.model_name, "claude-sonnet-4-high");
}

#[test]
fn xai_reasoning_effort_suffix_is_normalized() {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_TEMPERATURE", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
        ("GOOSE_TOOLSHIM", None::<&str>),
        ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
    ]);
    let config = ModelConfig::new("grok-4.5-high");
    assert_eq!(config.model_name, "grok-4.5");
    assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
}

#[test]
fn parse_aliases() {
    assert_eq!("off".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Off));
    assert_eq!(
        "disabled".parse::<ThinkingEffort>(),
        Ok(ThinkingEffort::Off)
    );
    assert_eq!("med".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Medium));
    assert_eq!("max".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
    assert_eq!("xhigh".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
    assert!("invalid".parse::<ThinkingEffort>().is_err());
}

