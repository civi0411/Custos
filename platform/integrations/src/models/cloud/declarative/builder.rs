use super::types::{
    should_preserve_thinking_by_default, DeclarativeProviderConfig, EnvVarConfig, KeyResolver,
    ProviderEngine,
};
use crate::base::Provider;
use crate::cloud::{
    anthropic, ollama, openai,
    transport::api_client::TlsConfig,
};
use anyhow::Result;
use std::path::Path;

pub fn expand_env_vars(template: &str, env_vars: &[EnvVarConfig]) -> Result<String> {
    let mut result = template.to_string();

    for var in env_vars {
        let placeholder = format!("${{{}}}", var.name);
        if !result.contains(&placeholder) {
            continue;
        }

        let value = match std::env::var(&var.name) {
            Ok(value) => value,
            Err(_) => match &var.default {
                Some(default) => default.clone(),
                None if var.required => {
                    anyhow::bail!("Required environment variable {} is not set", var.name)
                }
                None => continue,
            },
        };

        result = result.replace(&placeholder, &value);
    }

    Ok(result)
}

pub fn resolve_config(config: &mut DeclarativeProviderConfig) -> Result<()> {
    if let Some(env_vars) = &config.env_vars {
        config.base_url = expand_env_vars(&config.base_url, env_vars)?;

        for var in env_vars {
            if var.name.ends_with("_STREAMING") {
                let value = std::env::var(&var.name)
                    .ok()
                    .or_else(|| var.default.clone())
                    .map(|value| value.eq_ignore_ascii_case("true"));
                if let Some(value) = value {
                    config.supports_streaming = Some(value);
                }
            }
        }
    }

    Ok(())
}

pub fn deserialize_provider_config(json: &str) -> Result<DeclarativeProviderConfig> {
    let raw: serde_json::Value = serde_json::from_str(json)?;
    let preserves_thinking_was_set = raw.get("preserves_thinking").is_some();
    let mut config: DeclarativeProviderConfig = serde_json::from_value(raw)?;

    if !preserves_thinking_was_set {
        config.preserves_thinking = should_preserve_thinking_by_default(&config.engine);
    }

    Ok(config)
}

pub fn config_from_json(json: &str) -> Result<DeclarativeProviderConfig> {
    let mut config = deserialize_provider_config(json)?;
    resolve_config(&mut config)?;
    Ok(config)
}

pub fn load_custom_providers(dir: &Path) -> Result<Vec<DeclarativeProviderConfig>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .map(|path| {
            let content = std::fs::read_to_string(&path)?;
            deserialize_provider_config(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))
        })
        .collect()
}

pub fn from_json(
    json: &str,
    tls_config: Option<TlsConfig>,
    key_resolver: impl KeyResolver,
) -> Result<Box<dyn Provider>> {
    let config = config_from_json(json)?;

    match config.engine {
        ProviderEngine::OpenAI => openai::from_declarative_config(config, tls_config, key_resolver)
            .map(|provider| Box::new(provider.build()) as Box<dyn Provider>),
        ProviderEngine::Ollama => ollama::from_declarative_config(config, tls_config, key_resolver)
            .map(|provider| Box::new(provider.build()) as Box<dyn Provider>),
        ProviderEngine::Anthropic => {
            anthropic::from_declarative_config(config, tls_config, key_resolver)
                .map(|provider| Box::new(provider.build()) as Box<dyn Provider>)
        }
    }
}
