use anyhow::Result;
use std::time::Duration;
use url::Url;

use super::options::{
    OllamaOptions, OLLAMA_DEFAULT_PORT, OLLAMA_PROVIDER_NAME, OLLAMA_TIMEOUT,
};
use super::provider::OllamaProvider;
use crate::cloud::transport::api_client::{ApiClient, AuthMethod, TlsConfig};
use crate::base::ModelInfo;
use crate::cloud::declarative::{DeclarativeProviderConfig, KeyResolver};

pub struct OllamaProviderBuilder {
    api_client: ApiClient,
    name: String,
    custom_models: Option<Vec<ModelInfo>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    options: OllamaOptions,
}

impl OllamaProviderBuilder {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            name: OLLAMA_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            options: OllamaOptions::default(),
        }
    }

    pub fn api_client(mut self, api_client: ApiClient) -> Self {
        self.api_client = api_client;
        self
    }

    pub fn map_api_client(mut self, f: impl FnOnce(ApiClient) -> ApiClient) -> Self {
        self.api_client = f(self.api_client);
        self
    }

    pub fn try_map_api_client(
        mut self,
        f: impl FnOnce(ApiClient) -> Result<ApiClient>,
    ) -> Result<Self> {
        self.api_client = f(self.api_client)?;
        Ok(self)
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn custom_models(mut self, custom_models: Option<Vec<ModelInfo>>) -> Self {
        self.custom_models = custom_models;
        self
    }

    pub fn dynamic_models(mut self, dynamic_models: Option<bool>) -> Self {
        self.dynamic_models = dynamic_models;
        self
    }

    pub fn skip_canonical_filtering(mut self, skip_canonical_filtering: bool) -> Self {
        self.skip_canonical_filtering = skip_canonical_filtering;
        self
    }

    pub fn options(mut self, options: OllamaOptions) -> Self {
        self.options = options;
        self
    }

    pub fn build(self) -> OllamaProvider {
        OllamaProvider::from_builder(
            self.api_client,
            self.name,
            self.custom_models,
            self.dynamic_models,
            self.skip_canonical_filtering,
            self.options,
        )
    }
}

pub fn from_declarative_config(
    config: DeclarativeProviderConfig,
    tls_config: Option<TlsConfig>,
    key_resolver: impl KeyResolver,
) -> Result<OllamaProviderBuilder> {
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

    let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(OLLAMA_TIMEOUT));

    let base_has_scheme =
        config.base_url.starts_with("http://") || config.base_url.starts_with("https://");
    let base = if base_has_scheme {
        config.base_url.clone()
    } else {
        format!("http://{}", config.base_url)
    };

    let mut base_url = Url::parse(&base)
        .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

    let is_localhost = matches!(base_url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));

    if base_url.port().is_none() && !base_has_scheme && is_localhost {
        base_url
            .set_port(Some(OLLAMA_DEFAULT_PORT))
            .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
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
        Some(key) if !key.is_empty() => AuthMethod::BearerToken(key),
        _ => AuthMethod::NoAuth,
    };

    let mut api_client =
        ApiClient::with_timeout_and_tls(base_url.to_string(), auth, timeout, tls_config)?;

    if let Some(headers) = &config.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }
        api_client = api_client.with_headers(header_map)?;
    }

    let supports_streaming = config.supports_streaming.unwrap_or(true);

    if !supports_streaming {
        return Err(anyhow::anyhow!(
            "Ollama provider does not support non-streaming mode. All Ollama models support streaming. \
            Please remove 'supports_streaming: false' from your provider configuration."
        ));
    }

    Ok(OllamaProviderBuilder::new(api_client)
        .name(config.name.clone())
        .custom_models(custom_models)
        .dynamic_models(config.dynamic_models)
        .skip_canonical_filtering(config.skip_canonical_filtering))
}
