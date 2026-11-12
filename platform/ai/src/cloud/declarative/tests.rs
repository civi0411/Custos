use super::*;
use anyhow::Result;
use serde_json::json;
use std::collections::HashSet;

fn model_json() -> serde_json::Value {
    json!({
        "name": "test-model",
        "context_limit": 4096,
        "input_token_cost": null,
        "output_token_cost": null,
        "currency": null,
        "supports_cache_control": null,
        "reasoning": false
    })
}

#[test]
fn provider_engine_deserializes_compatible_aliases() {
    let openai: DeclarativeProviderConfig = serde_json::from_value(json!({
        "name": "test-openai",
        "engine": "openai_compatible",
        "display_name": "Test OpenAI",
        "base_url": "http://localhost:1234",
        "models": [model_json()]
    }))
    .unwrap();
    assert_eq!(openai.engine, ProviderEngine::OpenAI);

    let anthropic: DeclarativeProviderConfig = serde_json::from_value(json!({
        "name": "test-anthropic",
        "engine": "anthropic_compatible",
        "display_name": "Test Anthropic",
        "base_url": "http://localhost:1234",
        "models": [model_json()]
    }))
    .unwrap();
    assert_eq!(anthropic.engine, ProviderEngine::Anthropic);

    let ollama: DeclarativeProviderConfig = serde_json::from_value(json!({
        "name": "test-ollama",
        "engine": "ollama_compatible",
        "display_name": "Test Ollama",
        "base_url": "http://localhost:11434",
        "models": [model_json()]
    }))
    .unwrap();
    assert_eq!(ollama.engine, ProviderEngine::Ollama);
}

#[test]
fn groq_json_disables_thinking_preservation() {
    let config =
        deserialize_provider_config(crate::cloud::groq::JSON).expect("groq.json should parse");

    assert!(!config.preserves_thinking);
}

#[test]
fn setup_metadata_rejects_unknown_fields() {
    let mut definition: serde_json::Value = serde_json::from_str(crate::cloud::groq::JSON).unwrap();
    definition["setup"]["description"] = json!("This field would be ignored");

    let error = deserialize_provider_config(&definition.to_string()).unwrap_err();

    assert!(error.to_string().contains("unknown field `description`"));
}

fn placeholder_var_names(template: &str) -> Vec<String> {
    template
        .split("${")
        .skip(1)
        .filter_map(|chunk| chunk.split_once('}'))
        .map(|(name, _)| name.to_string())
        .collect()
}

fn validate_provider_id(id: &str) -> Result<()> {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("Invalid provider id: provider id cannot be empty");
    };

    if !(first.is_ascii_lowercase() || first.is_ascii_digit() || first == '_') {
        anyhow::bail!("Invalid provider id: {id}");
    }

    if chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        Ok(())
    } else {
        anyhow::bail!("Invalid provider id: {id}")
    }
}

#[test]
fn expose_declarative_providers_enumerates_all_bundled_json_files() {
    let enumerated: HashSet<_> = fixed_provider_config_entries()
        .into_iter()
        .map(|(path, _)| path.to_string())
        .collect();
    let bundled: HashSet<_> = FIXED_PROVIDERS
        .files()
        .filter(|file| file.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .map(|file| {
            file.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    assert_eq!(enumerated, bundled);
}

#[test]
fn all_bundled_providers_are_valid() {
    let mut seen_ids = HashSet::new();

    for (path, json) in fixed_provider_config_entries() {
        let config = deserialize_provider_config(json)
            .unwrap_or_else(|e| panic!("{path} failed to parse: {e}"));

        validate_provider_id(config.id())
            .unwrap_or_else(|e| panic!("{path} has an invalid provider id: {e}"));
        assert!(
            seen_ids.insert(config.id().to_string()),
            "{path} has a duplicate provider id: {}",
            config.id()
        );
        assert!(!config.base_url.is_empty(), "{path} has an empty base_url");

        if config.dynamic_models == Some(false) {
            assert!(
                !config.models.is_empty(),
                "{path} disables dynamic_models but lists no static models"
            );
        }

        let declared: HashSet<&str> = config
            .env_vars
            .iter()
            .flatten()
            .map(|v| v.name.as_str())
            .collect();
        let templates = std::iter::once(config.base_url.as_str())
            .chain(config.base_path.as_deref())
            .chain(
                config
                    .headers
                    .iter()
                    .flat_map(|h| h.values())
                    .map(String::as_str),
            );
        for template in templates {
            for var in placeholder_var_names(template) {
                assert!(
                    declared.contains(var.as_str()),
                    "{path} references ${{{var}}} but declares no matching env_var"
                );
            }
        }
    }

    assert!(!seen_ids.is_empty(), "no bundled providers were found");
}

#[test]
fn opencode_go_overrides_session_id_header() {
    let config = fixed_provider_configs()
        .expect("bundled providers should load")
        .into_iter()
        .find(|config| config.name == "opencode_go")
        .expect("opencode_go should be bundled");

    assert_eq!(
        config.session_id_header_override.as_deref(),
        Some("x-opencode-session")
    );
}

#[test]
fn fixed_provider_configs_are_unresolved() {
    let configs = fixed_provider_configs().expect("bundled providers should load");
    let config = configs
        .iter()
        .find(|config| config.env_vars.is_some())
        .expect("at least one bundled provider should declare env_vars");

    assert!(
        config.base_url.contains("${"),
        "{} should keep base_url placeholders unresolved",
        config.id()
    );
}

#[test]
fn from_json_defaults_openai_preserves_thinking_to_true() {
    let json = json!({
        "name": "test-provider",
        "engine": "openai",
        "display_name": "Test Provider",
        "base_url": "http://localhost:1234/v1/chat/completions",
        "models": [model_json()],
        "requires_auth": false,
        "dynamic_models": false
    })
    .to_string();

    let config = config_from_json(&json).unwrap();

    assert!(config.preserves_thinking);
}

#[test]
fn from_json_preserves_explicit_openai_preserves_thinking_false() {
    let json = json!({
        "name": "test-provider",
        "engine": "openai",
        "display_name": "Test Provider",
        "base_url": "http://localhost:1234/v1/chat/completions",
        "models": [model_json()],
        "requires_auth": false,
        "dynamic_models": false,
        "preserves_thinking": false
    })
    .to_string();

    let config = config_from_json(&json).unwrap();

    assert!(!config.preserves_thinking);
}

#[test]
fn from_json_expands_base_url_from_env_var_default() {
    let _guard = env_lock::lock_env([("TEST_PROVIDER_HOST", None::<&str>)]);
    let json = json!({
        "name": "test-provider",
        "engine": "openai",
        "display_name": "Test Provider",
        "base_url": "${TEST_PROVIDER_HOST}/v1/chat/completions",
        "models": [model_json()],
        "requires_auth": false,
        "dynamic_models": false,
        "env_vars": [{
            "name": "TEST_PROVIDER_HOST",
            "default": "http://localhost:1234"
        }]
    })
    .to_string();

    let provider = from_json(&json, None, EnvKeyResolver).unwrap();

    assert_eq!(provider.get_name(), "test-provider");
}

#[tokio::test]
async fn from_json_ollama_returns_static_models_when_dynamic_models_false() {
    let json = json!({
        "name": "test-ollama",
        "engine": "ollama",
        "display_name": "Test Ollama",
        "base_url": "http://localhost:11434",
        "models": [model_json()],
        "requires_auth": false,
        "dynamic_models": false
    })
    .to_string();

    let provider = from_json(&json, None, EnvKeyResolver).unwrap();

    assert_eq!(
        provider.fetch_supported_models().await.unwrap(),
        vec!["test-model".to_string()]
    );
}

#[test]
fn from_json_errors_when_required_env_var_is_missing() {
    let _guard = env_lock::lock_env([("TEST_PROVIDER_REQUIRED_HOST", None::<&str>)]);
    let json = json!({
        "name": "test-provider",
        "engine": "openai",
        "display_name": "Test Provider",
        "base_url": "${TEST_PROVIDER_REQUIRED_HOST}/v1/chat/completions",
        "models": [model_json()],
        "requires_auth": false,
        "dynamic_models": false,
        "env_vars": [{
            "name": "TEST_PROVIDER_REQUIRED_HOST",
            "required": true
        }]
    })
    .to_string();

    let err = match from_json(&json, None, EnvKeyResolver) {
        Ok(_) => panic!("expected missing required env var error"),
        Err(err) => err,
    };

    assert!(err
        .to_string()
        .contains("Required environment variable TEST_PROVIDER_REQUIRED_HOST is not set"));
}
