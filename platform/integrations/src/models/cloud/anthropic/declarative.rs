use anyhow::Result;

use super::types::{
    AnthropicProviderBuilder, ANTHROPIC_API_VERSION, DEFAULT_ANTHROPIC_TIMEOUT_SECONDS,
};
use crate::cloud::transport::api_client::{ApiClient, AuthMethod, TlsConfig};
use crate::cloud::declarative::{DeclarativeProviderConfig, KeyResolver};
use crate::cloud::codecs::anthropic::AnthropicFormatOptions;

pub(crate) fn format_options_for_provider(
    preserves_thinking: bool,
    emit_clear_thinking: bool,
) -> AnthropicFormatOptions {
    AnthropicFormatOptions {
        preserve_unsigned_thinking: preserves_thinking,
        preserve_thinking_context: preserves_thinking,
        emit_clear_thinking,
        ..Default::default()
    }
}

pub fn from_declarative_config(
    config: DeclarativeProviderConfig,
    tls_config: Option<TlsConfig>,
    key_resolver: impl KeyResolver,
) -> Result<AnthropicProviderBuilder> {
    let custom_models = if !config.models.is_empty() {
        Some(config.models.clone())
    } else {
        None
    };

    if config.dynamic_models == Some(false) && custom_models.is_none() {
        return Err(anyhow::anyhow!(
            "Provider '{}' has dynamic_models: false but no static models listed; \
             at least one entry in `models` is required.",
            config.name
        ));
    }

    config.validate_auth()?;

    let api_key = if config.api_key_env.is_empty() {
        None
    } else {
        match key_resolver.resolve_key(config.api_key_env.as_str()) {
            Ok(key) => Some(key),
            Err(err) => {
                if config.requires_auth {
                    anyhow::bail!("missing required key {}: {}", config.api_key_env, err);
                }
                None
            }
        }
    };

    let auth = match api_key {
        Some(key) if !key.is_empty() => AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key,
        },
        _ => AuthMethod::NoAuth,
    };

    let format_options =
        format_options_for_provider(config.preserves_thinking, config.emit_clear_thinking);

    let timeout_secs = config
        .timeout_seconds
        .unwrap_or(DEFAULT_ANTHROPIC_TIMEOUT_SECONDS);
    let mut api_client = ApiClient::with_timeout_and_tls(
        config.base_url,
        auth,
        std::time::Duration::from_secs(timeout_secs),
        tls_config,
    )?;

    if let Some(headers) = &config.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            reqwest::header::HeaderName::from_static("anthropic-version"),
            reqwest::header::HeaderValue::from_static(ANTHROPIC_API_VERSION),
        );
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }
        api_client = api_client.with_headers(header_map)?;
    } else {
        api_client = api_client.with_header("anthropic-version", ANTHROPIC_API_VERSION)?;
    }

    let supports_streaming = config.supports_streaming.unwrap_or(true);

    if !supports_streaming {
        return Err(anyhow::anyhow!(
            "Anthropic provider does not support non-streaming mode. All Claude models support streaming. \
            Please remove 'supports_streaming: false' from your provider configuration."
        ));
    }

    Ok(AnthropicProviderBuilder::new(api_client)
        .supports_streaming(supports_streaming)
        .name(config.name.clone())
        .custom_models(custom_models)
        .dynamic_models(config.dynamic_models)
        .skip_canonical_filtering(config.skip_canonical_filtering)
        .format_options(format_options))
}
