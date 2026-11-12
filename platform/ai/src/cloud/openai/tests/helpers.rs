use super::super::builder::from_declarative_config;
use super::super::provider::OpenAiProvider;
use crate::cloud::transport::api_client::{ApiClient, AuthMethod};
use crate::base::ModelInfo;
use crate::cloud::declarative::{DeclarativeProviderConfig, KeyResolver};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) fn make_provider(name: &str) -> OpenAiProvider {
    OpenAiProvider {
        api_client: ApiClient::new_with_tls(
            "http://localhost".to_string(),
            AuthMethod::NoAuth,
            None,
        )
        .unwrap(),
        base_path: "v1/chat/completions".to_string(),
        organization: None,
        project: None,
        custom_headers: None,
        supports_streaming: true,
        name: name.to_string(),
        custom_models: None,
        dynamic_models: None,
        skip_canonical_filtering: false,
        preserve_thinking_context: false,
        n_ctx_cache: Arc::new(Mutex::new(HashMap::new())),
    }
}

pub(crate) fn make_provider_with_custom_models(
    host: &str,
    base_path: &str,
    custom_models: Vec<String>,
) -> OpenAiProvider {
    OpenAiProvider {
        api_client: ApiClient::new_with_tls(host.to_string(), AuthMethod::NoAuth, None).unwrap(),
        base_path: base_path.to_string(),
        organization: None,
        project: None,
        custom_headers: None,
        supports_streaming: true,
        name: "test-provider".to_string(),
        custom_models: Some(
            custom_models
                .into_iter()
                .map(|model| ModelInfo::new(model).with_context_limit(4096))
                .collect(),
        ),
        dynamic_models: Some(true),
        skip_canonical_filtering: false,
        preserve_thinking_context: false,
        n_ctx_cache: Arc::new(Mutex::new(HashMap::new())),
    }
}

pub(crate) fn custom_config(base_url: &str) -> DeclarativeProviderConfig {
    DeclarativeProviderConfig {
        name: "test-openai".to_string(),
        engine: crate::cloud::declarative::ProviderEngine::OpenAI,
        display_name: "Test OpenAI".to_string(),
        description: None,
        api_key_env: String::new(),
        base_url: base_url.to_string(),
        models: vec![ModelInfo::new("test-model").with_context_limit(4096)],
        headers: None,
        session_id_header_override: None,
        timeout_seconds: None,
        supports_streaming: None,
        requires_auth: false,
        catalog_provider_id: None,
        base_path: None,
        env_vars: None,
        auth: None,
        dynamic_models: Some(false),
        skip_canonical_filtering: false,
        model_doc_link: None,
        setup_steps: vec![],
        toolshim: false,
        preserves_thinking: false,
        emit_clear_thinking: false,
        setup: None,
    }
}

pub(crate) fn cerebras_config() -> DeclarativeProviderConfig {
    crate::cloud::declarative::fixed_provider_configs()
        .expect("bundled providers should load")
        .into_iter()
        .find(|config| config.name == "cerebras")
        .expect("cerebras should be bundled")
}

pub(crate) fn cerebras_provider() -> OpenAiProvider {
    struct StaticKeyResolver;
    impl KeyResolver for StaticKeyResolver {
        type Error = std::convert::Infallible;

        fn resolve_key(&self, _key: &str) -> std::result::Result<String, Self::Error> {
            Ok("test-key".to_string())
        }
    }

    from_declarative_config(cerebras_config(), None, StaticKeyResolver)
        .expect("cerebras config should build a provider")
        .build()
}
