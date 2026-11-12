use super::builder::{
    OpenAiProviderBuilder, OPEN_AI_DEFAULT_BASE_PATH, OPEN_AI_DEFAULT_MODEL, OPEN_AI_DOC_URL,
    OPEN_AI_PROVIDER_NAME,
};
use super::compat::{
    apply_declared_request_params, sanitize_request_for_compat,
    PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS,
};
use super::context_limit::{
    parse_model_ids, parse_n_ctx_from_models, CachedContextLimit, N_CTX_PROBE_TIMEOUT,
};
use crate::cloud::transport::api_client::ApiClient;
use crate::base::{
    known_models_from_registry, ConfigKey, MessageStream, ModelInfo, Provider, ProviderDescriptor,
    ProviderMetadata,
};
use crate::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::conversation::token_usage::{CostSource, ProviderUsage};
use crate::errors::ProviderError;
use crate::cloud::codecs::openai::{
    create_request_with_options, extract_reasoning_effort, get_cost, get_usage,
    is_openai_responses_model, record_response_metadata, response_to_message, OpenAiFormatOptions,
};
use crate::cloud::codecs::openai_responses::{
    create_responses_request_for_model, get_responses_usage, responses_api_to_message,
    ResponsesApiResponse,
};
use crate::cloud::transport::http_status::read_json_response;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::cloud::transport::openai_compatible::{
    handle_response_openai_compat, handle_status, stream_openai_compat, stream_responses_compat,
};
use crate::request_log::{start_log, LoggerHandleExt};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::StatusCode;
use rmcp::model::Tool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) const OPEN_AI_DEFAULT_RESPONSES_PATH: &str = "v1/responses";
pub(crate) const OPEN_AI_DEFAULT_MODELS_PATH: &str = "v1/models";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiProvider {
    #[serde(skip)]
    pub(crate) api_client: ApiClient,
    pub(crate) base_path: String,
    pub(crate) organization: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) custom_headers: Option<HashMap<String, String>>,
    pub(crate) supports_streaming: bool,
    pub(crate) name: String,
    pub(crate) custom_models: Option<Vec<ModelInfo>>,
    pub(crate) dynamic_models: Option<bool>,
    pub(crate) skip_canonical_filtering: bool,
    pub(crate) preserve_thinking_context: bool,
    #[serde(skip)]
    pub(crate) n_ctx_cache: Arc<Mutex<HashMap<String, CachedContextLimit>>>,
}

impl OpenAiProvider {
    pub(crate) fn from_builder(b: OpenAiProviderBuilder) -> Self {
        Self {
            api_client: b.api_client,
            base_path: b.base_path,
            organization: b.organization,
            project: b.project,
            custom_headers: b.custom_headers,
            supports_streaming: b.supports_streaming,
            name: b.name,
            custom_models: b.custom_models,
            dynamic_models: b.dynamic_models,
            skip_canonical_filtering: b.skip_canonical_filtering,
            preserve_thinking_context: b.preserve_thinking_context,
            n_ctx_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn stream_for_model(
        &self,
        model_config: &ModelConfig,
        wire_model: &str,
        capability_model: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_responses_request_for_model(
            model_config,
            wire_model,
            capability_model,
            system,
            messages,
            tools,
        )?;
        payload["stream"] = serde_json::Value::Bool(self.supports_streaming);
        self.stream_responses_payload(model_config, payload).await
    }

    async fn stream_responses_payload(
        &self,
        model_config: &ModelConfig,
        payload: serde_json::Value,
    ) -> Result<MessageStream, ProviderError> {
        let mut log = start_log(model_config, &payload)?;
        let response = self
            .with_retry(|| async {
                handle_status(
                    self.api_client
                        .request(&Self::map_base_path(
                            &self.base_path,
                            "responses",
                            OPEN_AI_DEFAULT_RESPONSES_PATH,
                        ))
                        .model_headers(model_config)?
                        .streaming(self.supports_streaming)
                        .response_post(&payload)
                        .await?,
                )
                .await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        if self.supports_streaming {
            stream_responses_compat(response, log)
        } else {
            let json: serde_json::Value = read_json_response(response).await?;
            let parsed: ResponsesApiResponse =
                serde_json::from_value(json.clone()).map_err(|e| {
                    ProviderError::ExecutionError(format!(
                        "Failed to parse responses API response: {}",
                        e
                    ))
                })?;
            let message = responses_api_to_message(&parsed)?;
            let usage_data = get_responses_usage(&parsed);
            let usage_json = json.get("usage").unwrap_or(&serde_json::Value::Null);
            let mut usage = ProviderUsage::new(model_config.model_name.clone(), usage_data);
            usage.response_id = Some(parsed.id.clone());
            let finish_reason = json
                .pointer("/incomplete_details/reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&parsed.status);
            usage.finish_reasons = Some(vec![finish_reason.to_string()]);
            if let Some(cost) = get_cost(usage_json) {
                usage = usage.with_cost(cost, CostSource::ProviderReported);
            }
            log.write(
                &serde_json::to_value(&message).unwrap_or_default(),
                Some(&usage_data),
            )?;
            Ok(crate::base::stream_from_single_message(message, usage))
        }
    }

    #[doc(hidden)]
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            base_path: OPEN_AI_DEFAULT_BASE_PATH.to_string(),
            organization: None,
            project: None,
            custom_headers: None,
            supports_streaming: true,
            name: OPEN_AI_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            preserve_thinking_context: false,
            n_ctx_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn normalize_base_path(base_path: &str) -> String {
        if let Some(path) = base_path.strip_prefix('/') {
            format!("/{}", path.trim_end_matches('/'))
        } else {
            base_path.trim_end_matches('/').to_string()
        }
    }

    pub(crate) fn is_chat_completions_path(base_path: &str) -> bool {
        let normalized = Self::normalize_base_path(base_path).to_ascii_lowercase();
        normalized.contains("chat/completions")
    }

    pub(crate) fn is_responses_path(base_path: &str) -> bool {
        let normalized = Self::normalize_base_path(base_path).to_ascii_lowercase();
        normalized.ends_with("responses") || normalized.contains("/responses")
    }

    pub(crate) fn is_responses_model(model_name: &str) -> bool {
        is_openai_responses_model(model_name)
    }

    pub(crate) fn should_use_responses_api(model_name: &str, base_path: &str) -> bool {
        let normalized_base_path = Self::normalize_base_path(base_path);
        // Only the standard "v1/chat/completions" is treated as a default
        // path that defers to model-based routing.  The versionless
        // "chat/completions" (derived from an OPENAI_BASE_URL without /v1)
        // is treated as custom because versionless gateways typically do not
        // support the Responses API.
        let has_custom_base_path = normalized_base_path != OPEN_AI_DEFAULT_BASE_PATH;

        if has_custom_base_path {
            if Self::is_responses_path(&normalized_base_path) {
                return true;
            }
            if Self::is_chat_completions_path(&normalized_base_path) {
                return false;
            }
        }

        Self::is_responses_model(model_name)
    }

    pub(crate) fn declared_model(&self, model_name: &str) -> Option<&ModelInfo> {
        self.custom_models
            .as_ref()?
            .iter()
            .find(|m| m.name == model_name)
    }

    pub(crate) fn sanitize_request_for_compat(
        &self,
        payload: serde_json::Value,
        model_config: &ModelConfig,
    ) -> serde_json::Value {
        sanitize_request_for_compat(
            &self.name,
            payload,
            model_config,
            Self::is_responses_model,
        )
    }

    pub(crate) fn should_use_responses_api_for_provider(&self, model_name: &str) -> bool {
        if PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS.contains(&self.name.as_str()) {
            return false;
        }

        Self::should_use_responses_api(model_name, &self.base_path)
    }

    pub(crate) fn map_base_path(base_path: &str, target: &str, fallback: &str) -> String {
        let normalized = Self::normalize_base_path(base_path);
        if normalized.ends_with(target) || normalized.contains(&format!("/{target}")) {
            return normalized;
        }

        if Self::is_chat_completions_path(&normalized) {
            return normalized.replacen("chat/completions", target, 1);
        }

        if Self::is_responses_path(&normalized) {
            return normalized.replacen("responses", target, 1);
        }

        if normalized.starts_with('/') {
            format!("/{}", fallback.trim_start_matches('/'))
        } else {
            fallback.to_string()
        }
    }

    pub(crate) async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        let models_path =
            Self::map_base_path(&self.base_path, "models", OPEN_AI_DEFAULT_MODELS_PATH);
        let response = self.api_client.request(&models_path).response_get().await?;

        if response.status() == StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::EndpointNotFound(body));
        }

        let response = handle_status(response).await?;

        let body = response.bytes().await.map_err(|e| {
            ProviderError::NetworkError(format!("Failed to read response body: {}", e))
        })?;
        let json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
            ProviderError::EndpointNotFound(format!("Response body is not valid JSON: {}", e))
        })?;

        if let Some(err_obj) = json.get("error").filter(|error| !error.is_null()) {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        parse_model_ids(&json)
    }

    /// llama.cpp and Ollama expose the actual allocated context window in the
    /// non-standard `meta.n_ctx` field of `/v1/models`. Returns `None` when absent
    /// (e.g. real OpenAI).
    pub(crate) async fn fetch_n_ctx_from_api(
        &self,
        model_name: &str,
    ) -> Result<Option<usize>, ProviderError> {
        let models_path =
            Self::map_base_path(&self.base_path, "models", OPEN_AI_DEFAULT_MODELS_PATH);
        let response = self.api_client.request(&models_path).response_get().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let json = handle_response_openai_compat(response).await.map_err(|error| {
            if matches!(&error, ProviderError::RequestFailed(message) if message.contains("not valid JSON")) {
                ProviderError::EndpointNotFound(error.to_string())
            } else {
                error
            }
        })?;
        Ok(parse_n_ctx_from_models(&json, model_name))
    }
}

impl ProviderDescriptor for OpenAiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            OPEN_AI_PROVIDER_NAME,
            "OpenAI",
            "GPT-4 and other OpenAI models, including OpenAI compatible ones",
            OPEN_AI_DEFAULT_MODEL,
            known_models_from_registry(OPEN_AI_PROVIDER_NAME),
            OPEN_AI_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_API_KEY", false, true, None, true),
                ConfigKey::new("OPENAI_BASE_URL", false, false, None, false),
                ConfigKey::new(
                    "OPENAI_HOST",
                    true,
                    false,
                    Some("https://api.openai.com"),
                    false,
                ),
                ConfigKey::new(
                    "OPENAI_BASE_PATH",
                    true,
                    false,
                    Some("v1/chat/completions"),
                    false,
                ),
                ConfigKey::new("OPENAI_ORGANIZATION", false, false, None, false),
                ConfigKey::new("OPENAI_PROJECT", false, false, None, false),
                ConfigKey::new("OPENAI_CUSTOM_HEADERS", false, true, None, false),
                ConfigKey::new("OPENAI_TIMEOUT", false, false, Some("600"), false),
            ],
        )
        .with_setup_steps(vec![
            "Go to https://platform.openai.com and sign up or log in",
            "Navigate to API Keys in the left sidebar",
            "Click 'Create new secret key'",
            "Copy the key and paste it above",
        ])
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        self.api_client
            .refresh_credentials()
            .await
            .map_err(|error| ProviderError::Authentication(error.to_string()))
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        let configured_limits = self
            .custom_models
            .iter()
            .flatten()
            .filter_map(|model| model.context_limit.map(|limit| (model.name.clone(), limit)));
        let resolver = custos_provider::context_limit::ContextLimitResolver::new(&self.name)
            .with_configured_limits(configured_limits);

        resolver
            .resolve(model, override_limit, || async {
                if let Some(cached) = self
                    .n_ctx_cache
                    .lock()
                    .ok()
                    .and_then(|cache| cache.get(model).copied())
                    .and_then(CachedContextLimit::value)
                {
                    return Ok(cached);
                }

                let probed = match tokio::time::timeout(
                    N_CTX_PROBE_TIMEOUT,
                    self.fetch_n_ctx_from_api(model),
                )
                .await
                {
                    Ok(Ok(limit)) => Ok(limit),
                    Ok(Err(error)) if error.is_endpoint_not_found() => Ok(None),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(ProviderError::RequestFailed(
                        "Context-limit discovery timed out".into(),
                    )),
                };

                if let Ok(mut cache) = self.n_ctx_cache.lock() {
                    let cached = match probed.as_ref() {
                        Ok(limit) => CachedContextLimit::Success(*limit),
                        Err(_) => CachedContextLimit::Failure(Instant::now()),
                    };
                    cache.insert(model.to_string(), cached);
                }
                probed
            })
            .await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(custom_models) = &self.custom_models {
            let names: Vec<String> = custom_models.iter().map(|m| m.name.clone()).collect();
            if self.dynamic_models == Some(false) {
                return Ok(names);
            }
            match self.fetch_models_from_api().await {
                Ok(models) => return Ok(models),
                Err(e) if e.is_endpoint_not_found() => {
                    tracing::debug!(
                        "Models endpoint not implemented for provider '{}' ({}), using predefined list",
                        self.name,
                        e
                    );
                    return Ok(names);
                }
                Err(e) => return Err(e),
            }
        }

        self.fetch_models_from_api().await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        if self.should_use_responses_api_for_provider(&model_config.model_name) {
            let (wire_model, _) =
                extract_reasoning_effort(&model_config.model_name);
            self.stream_for_model(
                model_config,
                &wire_model,
                &model_config.model_name,
                system,
                messages,
                tools,
            )
            .await
        } else {
            let declared_model = self.declared_model(&model_config.model_name);
            let thinking_preservation_format =
                declared_model.and_then(|m| m.thinking_preservation_format);

            let mut payload = create_request_with_options(
                model_config,
                system,
                messages,
                tools,
                &ImageFormat::OpenAi,
                self.supports_streaming,
                OpenAiFormatOptions {
                    preserve_thinking_context: self.preserve_thinking_context
                        || thinking_preservation_format.is_some(),
                    supports_vision: model_config.supports_vision.unwrap_or_default(),
                    thinking_preservation_format,
                },
            )?;

            if let Some(params) = declared_model.and_then(|m| m.request_params.as_ref()) {
                apply_declared_request_params(&mut payload, params);
            }

            let payload = self.sanitize_request_for_compat(payload, model_config);
            let mut log = start_log(model_config, &payload)?;

            let response = self
                .with_retry(|| async {
                    let resp = self
                        .api_client
                        .request(&self.base_path)
                        .model_headers(model_config)?
                        .streaming(self.supports_streaming)
                        .response_post(&payload)
                        .await?;
                    handle_status(resp).await
                })
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            if self.supports_streaming {
                stream_openai_compat(response, log)
            } else {
                let json: serde_json::Value = read_json_response(response).await?;

                let message = response_to_message(&json).map_err(|e| {
                    ProviderError::RequestFailed(format!("Failed to parse message: {}", e))
                })?;

                let usage_json = json.get("usage").unwrap_or(&serde_json::Value::Null);
                let usage_data = get_usage(usage_json);
                let mut usage = ProviderUsage::new(model_config.model_name.clone(), usage_data);
                record_response_metadata(&mut usage, &json);
                if let Some(cost) = get_cost(usage_json) {
                    usage = usage.with_cost(cost, CostSource::ProviderReported);
                }

                log.write(
                    &serde_json::to_value(&message).unwrap_or_default(),
                    Some(&usage_data),
                )?;

                Ok(crate::base::stream_from_single_message(message, usage))
            }
        }
    }
}
