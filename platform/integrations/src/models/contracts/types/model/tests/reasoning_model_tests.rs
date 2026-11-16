use super::super::types::ModelConfig;

const ENV_LOCK_KEYS: [(&str, Option<&str>); 5] = [
    ("GOOSE_MAX_TOKENS", None),
    ("GOOSE_TEMPERATURE", None),
    ("GOOSE_CONTEXT_LIMIT", None),
    ("GOOSE_TOOLSHIM", None),
    ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None),
];

#[test]
fn bare_reasoning_models() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    assert!(ModelConfig::new("o1").is_openai_reasoning_model());
    assert!(ModelConfig::new("o1-preview").is_openai_reasoning_model());
    assert!(ModelConfig::new("o3").is_openai_reasoning_model());
    assert!(ModelConfig::new("o3-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("o4-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("gpt-5").is_openai_reasoning_model());
    assert!(ModelConfig::new("gpt-5-3-codex").is_openai_reasoning_model());
}

#[test]
fn goose_prefixed_reasoning_models() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    assert!(ModelConfig::new("goose-o3-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("goose-o4-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("goose-gpt-5").is_openai_reasoning_model());
}

#[test]
fn databricks_prefixed_reasoning_models() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    assert!(ModelConfig::new("databricks-o3-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("databricks-o4-mini").is_openai_reasoning_model());
    assert!(ModelConfig::new("databricks-gpt-5").is_openai_reasoning_model());
}

#[test]
fn non_reasoning_models() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    assert!(!ModelConfig::new("claude-sonnet-4").is_openai_reasoning_model());
    assert!(!ModelConfig::new("gpt-4o").is_openai_reasoning_model());
    assert!(!ModelConfig::new("databricks-claude-sonnet-4").is_openai_reasoning_model());
    assert!(!ModelConfig::new("goose-claude-sonnet-4").is_openai_reasoning_model());
    assert!(!ModelConfig::new("llama-3-70b").is_openai_reasoning_model());
}

#[test]
fn includes_reasoning_model_families() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    assert!(ModelConfig::new("o3-mini").is_reasoning_model());
    assert!(ModelConfig::new("claude-sonnet-4").is_reasoning_model());
    assert!(ModelConfig::new("gemini-3-pro").is_reasoning_model());
    assert!(ModelConfig::new("glm-5.3").is_reasoning_model());
    assert!(
        ModelConfig::new("data_workflow_tools.goose.goose-glm-5-3").is_reasoning_model()
    );
    assert!(!ModelConfig::new("glm-5.30").is_reasoning_model());
    assert!(!ModelConfig::new("glm_5_3_models.prod.llama-3").is_reasoning_model());
    assert!(ModelConfig::new("grok-4.5").is_reasoning_model());
    assert!(ModelConfig::new("grok-4.20-0309-reasoning").is_reasoning_model());
    assert!(!ModelConfig::new("grok-4.20-0309-non-reasoning").is_reasoning_model());
}

#[test]
fn recognizes_kimi_k3_without_matching_other_versions() {
    for model in [
        "kimi-k3",
        "moonshotai/kimi-k3",
        "catalog.schema.goose-kimi-k3",
        "Kimi-K3",
    ] {
        assert!(ModelConfig::new(model).is_reasoning_model(), "{model}");
        let mut config = ModelConfig::new(model);
        config.reasoning = Some(false);
        assert!(!config.is_reasoning_model());
    }
    for model in ["kimi-k30", "kimi-k2.5", "kimi-k2-thinking", "notkimi-k3"] {
        assert!(
            !ModelConfig::new(model).is_kimi_k3_reasoning_model(),
            "{model}"
        );
    }
}

#[test]
fn uses_explicit_metadata_first() {
    let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
    let mut config = ModelConfig::new("provider-alias");
    config.reasoning = Some(true);
    assert!(config.is_reasoning_model());

    let mut config = ModelConfig::new("claude-sonnet-4");
    config.reasoning = Some(false);
    assert!(!config.is_reasoning_model());
}

