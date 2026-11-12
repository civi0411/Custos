use anyhow::Result;
use async_trait::async_trait;
use reqwest::StatusCode;
use rmcp::model::Tool;
use serde_json::Value;

use super::builder::OllamaProviderBuilder;
use super::options::{
    apply_ollama_options, OllamaOptions, OLLAMA_BACKOFF_MULTIPLIER, OLLAMA_DEFAULT_MODEL,
    OLLAMA_DOC_URL, OLLAMA_HOST, OLLAMA_INITIAL_RETRY_INTERVAL_MS, OLLAMA_KNOWN_MODELS,
    OLLAMA_MAX_RETRIES, OLLAMA_MAX_RETRY_INTERVAL_MS, OLLAMA_PROVIDER_NAME, OLLAMA_TIMEOUT,
};
use super::streaming::stream_ollama;
use crate::cloud::transport::api_client::ApiClient;
use crate::base::{
    ConfigKey, MessageStream, ModelInfo, Provider, ProviderDescriptor, ProviderMetadata,
};
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::cloud::codecs::ollama::create_request;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::cloud::transport::openai_compatible::handle_status;
use crate::request_log::{start_log, LoggerHandleExt};
use crate::retry::{ProviderRetry, RetryConfig};

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    pub(crate) api_client: ApiClient,
    pub(crate) name: String,
    pub(crate) custom_models: Option<Vec<ModelInfo>>,
    pub(crate) dynamic_models: Option<bool>,
    pub(crate) skip_canonical_filtering: bool,
    pub(crate) options: OllamaOptions,
}

impl OllamaProvider {
    pub fn new(
        api_client: ApiClient,
        name: String,
        skip_canonical_filtering: bool,
        options: OllamaOptions,
    ) -> Self {
        OllamaProviderBuilder::new(api_client)
            .name(name)
            .skip_canonical_filtering(skip_canonical_filtering)
            .options(options)
            .build()
    }

    pub(crate) fn from_builder(
        api_client: ApiClient,
        name: String,
        custom_models: Option<Vec<ModelInfo>>,
        dynamic_models: Option<bool>,
        skip_canonical_filtering: bool,
        options: OllamaOptions,
    ) -> Self {
        Self {
            api_client,
            name,
            custom_models,
            dynamic_models,
            skip_canonical_filtering,
            options,
        }
    }

    pub fn with_options(mut self, options: OllamaOptions) -> Self {
        self.options = options;
        self
    }

    async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        fetch_ollama_model_names(&self.api_client)
            .await?
            .ok_or_else(|| ProviderError::RequestFailed("No models array in response".to_string()))
    }
}

pub async fn fetch_ollama_model_names(
    client: &ApiClient,
) -> Result<Option<Vec<String>>, ProviderError> {
    let response = client
        .request("api/tags")
        .response_get()
        .await
        .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(ProviderError::EndpointNotFound(
            "Ollama models endpoint not found".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err(ProviderError::RequestFailed(format!(
            "Failed to fetch models: HTTP {}",
            response.status()
        )));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse response: {}", e)))?;

    let Some(models) = json.get("models").and_then(|m| m.as_array()) else {
        return Ok(None);
    };

    let mut names: Vec<String> = models
        .iter()
        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    names.sort();
    Ok(Some(names))
}

impl ProviderDescriptor for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OLLAMA_PROVIDER_NAME,
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.to_vec(),
            OLLAMA_DOC_URL,
            vec![
                ConfigKey::new("OLLAMA_HOST", true, false, Some(OLLAMA_HOST), true),
                ConfigKey::new(
                    "OLLAMA_TIMEOUT",
                    false,
                    false,
                    Some(&(OLLAMA_TIMEOUT.to_string())),
                    false,
                ),
            ],
        )
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    fn retry_config(&self) -> RetryConfig {
        RetryConfig::new(
            OLLAMA_MAX_RETRIES,
            OLLAMA_INITIAL_RETRY_INTERVAL_MS,
            OLLAMA_BACKOFF_MULTIPLIER,
            OLLAMA_MAX_RETRY_INTERVAL_MS,
        )
        .transient_only()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;
        apply_ollama_options(&mut payload, &self.options, model_config);
        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .request("v1/chat/completions")
                    .model_headers(model_config)?
                    .streaming(true)
                    .response_post(&payload)
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        stream_ollama(response, self.options.chunk_timeout_secs, log)
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        let configured_limits = self
            .custom_models
            .iter()
            .flatten()
            .filter_map(|model| model.context_limit.map(|limit| (model.name.clone(), limit)));
        crate::context_limit::ContextLimitResolver::new(&self.name)
            .with_configured_limits(configured_limits)
            .resolve(model, override_limit, || async {
                Ok(self.options.input_limit)
            })
            .await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(custom_models) = &self.custom_models {
            if self.dynamic_models == Some(false) {
                return Ok(custom_models
                    .iter()
                    .map(|model| model.name.clone())
                    .collect());
            }

            match self.fetch_models_from_api().await {
                Ok(models) => return Ok(models),
                Err(e) if e.is_endpoint_not_found() => {
                    tracing::debug!(
                        "Models endpoint not implemented for provider '{}' ({}), using predefined list",
                        self.name,
                        e
                    );
                    return Ok(custom_models
                        .iter()
                        .map(|model| model.name.clone())
                        .collect());
                }
                Err(e) => return Err(e),
            }
        }

        self.fetch_models_from_api().await
    }
}
