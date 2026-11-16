use super::super::types::ModelConfig;

#[test]
fn reads_supports_vision_from_config() {
    let config: ModelConfig = serde_json::from_str(
        r#"{"model_name":"gpt-4o","toolshim":false,"supports_vision":true}"#,
    )
    .unwrap();
    assert_eq!(config.supports_vision, Some(true));

    let config: ModelConfig = serde_json::from_str(
        r#"{"model_name":"gpt-4o","toolshim":false,"supports_vision":false}"#,
    )
    .unwrap();
    assert_eq!(config.supports_vision, Some(false));
}

#[test]
fn defaults_supports_vision_to_none_when_absent() {
    let config: ModelConfig =
        serde_json::from_str(r#"{"model_name":"deepseek-v4","toolshim":false}"#).unwrap();
    assert_eq!(config.supports_vision, None);
}

#[test]
fn serializes_supports_vision_only_when_some() {
    let config = ModelConfig::new("gpt-4o").with_vision_support(true);
    let serialized = serde_json::to_value(&config).unwrap();
    assert_eq!(
        serialized.get("supports_vision"),
        Some(&serde_json::Value::Bool(true))
    );

    let config = ModelConfig::new("deepseek-v4");
    let serialized = serde_json::to_value(&config).unwrap();
    assert!(serialized.get("supports_vision").is_none());
}

#[test]
fn sets_limits_from_canonical_model() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("gpt-4o").with_canonical_limits("openai");
    assert_eq!(config.max_tokens, Some(16_384));
    assert_eq!(config.reasoning, Some(false));
}

#[test]
fn does_not_override_existing_max_tokens() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let mut config = ModelConfig::new("gpt-4o");
    config.max_tokens = Some(1_000);
    let config = config.with_canonical_limits("openai");

    assert_eq!(config.max_tokens, Some(1_000));
}

#[test]
fn skips_canonical_output_limit_when_it_equals_context_limit() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("moonshotai/kimi-k2.6").with_canonical_limits("nvidia");
    assert_eq!(config.max_tokens, None);
    assert_eq!(config.max_output_tokens(), 4_096);
}

#[test]
fn resolves_claude_sonnet_5_on_aws_bedrock() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("global.anthropic.claude-sonnet-5")
        .with_canonical_limits("aws_bedrock");
    assert_eq!(config.max_tokens, Some(128_000));
    assert_eq!(config.reasoning, Some(true));
}

#[test]
fn unknown_model_leaves_fields_none() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("totally-unknown-model").with_canonical_limits("openai");

    assert_eq!(config.context_limit, None);
    assert_eq!(config.max_tokens, None);
    assert_eq!(config.reasoning, None);
}

#[test]
fn resolves_after_stripping_reasoning_effort_suffix() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);

    let config = ModelConfig::new("gpt-5.6-sol-xhigh").with_canonical_limits("openai");
    assert_eq!(config.max_tokens, Some(128_000));
    assert_eq!(config.reasoning, Some(true));
    let canonical = crate::catalog::canonical::maybe_get_canonical_model("openai", "gpt-5.6-sol")
        .expect("gpt-5.6-sol should have canonical metadata");
    assert_eq!(canonical.temperature, Some(false));

    let config = ModelConfig::new("gpt-5.6-sol").with_canonical_limits("chatgpt_codex");
    assert_eq!(config.max_tokens, Some(128_000));
    assert_eq!(config.reasoning, Some(true));
}

#[test]
fn resolves_gpt_6_astra_limits_for_databricks_model_service() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("data_workflow_tools.goose.goose-gpt-6-astra")
        .with_canonical_limits("databricks_v2");

    let canonical = crate::catalog::canonical::maybe_get_canonical_model(
        "databricks_v2",
        "data_workflow_tools.goose.goose-gpt-6-astra",
    )
    .expect("GPT-6 Astra should have canonical metadata");
    assert_eq!(canonical.limit.context, 1_050_000);
    assert_eq!(canonical.limit.output, Some(128_000));
    assert_eq!(config.max_tokens, Some(128_000));
    assert_eq!(config.reasoning, Some(true));
    assert_eq!(config.supports_vision, Some(true));
}

#[test]
fn fills_supports_vision_from_canonical_model() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("gpt-4o").with_canonical_limits("openai");
    assert_eq!(config.supports_vision, Some(true));
}

#[test]
fn does_not_override_existing_supports_vision() {
    let _guard = env_lock::lock_env([
        ("GOOSE_MAX_TOKENS", None::<&str>),
        ("GOOSE_CONTEXT_LIMIT", None::<&str>),
    ]);
    let config = ModelConfig::new("gpt-4o")
        .with_vision_support(false)
        .with_canonical_limits("openai");
    assert_eq!(config.supports_vision, Some(false));
}

