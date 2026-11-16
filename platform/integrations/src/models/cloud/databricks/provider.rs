use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::{json, Value};

use crate::cloud::transport::api_client::{ApiClient, AuthMethod, TlsConfig};
use crate::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata};
use crate::conversation::message::Message;
use super::auth::{
    DatabricksAuth, DatabricksAuthProvider, DatabricksOauthTokenProvider, DatabricksRefreshHook,
    DatabricksSessionIdProvider, DatabricksTokenResolver,
};
use crate::errors::ProviderError;
use crate::cloud::codecs::databricks::{create_request_for_provider, DATABRICKS_PROVIDER_NAME};
use crate::cloud::codecs::openai::{
    extract_reasoning_effort, is_openai_responses_model, openai_reasoning_effort_for_thinking,
};
use crate::cloud::codecs::openai_responses::create_responses_request;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::cloud::transport::openai_compatible::{
    handle_status, map_http_error_to_provider_error, sanitize_url, stream_openai_compat,
    stream_responses_compat,
};
use crate::request_log::{start_log, LoggerHandleExt};
use crate::retry::{
    ProviderRetry, RetryConfig, DEFAULT_BACKOFF_MULTIPLIER, DEFAULT_INITIAL_RETRY_INTERVAL_MS,
    DEFAULT_MAX_RETRIES, DEFAULT_MAX_RETRY_INTERVAL_MS,
};

use super::types::{
    DatabricksEndpointInfo, DatabricksProvider, EndpointMetadataLookup, DATABRICKS_DEFAULT_MODEL,
    DATABRICKS_DOC_URL, DATABRICKS_KNOWN_MODELS, DEFAULT_PROVIDER_TIMEOUT_SECS,
};

impl DatabricksProvider {
    pub async fn cleanup() -> Result<()> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        auth: DatabricksAuth,
        retry_config: RetryConfig,
        tls_config: Option<TlsConfig>,
        oauth_token_provider: Option<DatabricksOauthTokenProvider>,
        token_resolver: Option<DatabricksTokenResolver>,
        request_builder: Option<crate::cloud::transport::api_client::RequestBuilderDecorator>,
        instance_id: Option<String>,
        refresh_hook: Option<DatabricksRefreshHook>,
        session_id_provider: Option<DatabricksSessionIdProvider>,
    ) -> Result<Self> {
        let token_cache = Arc::new(Mutex::new(match &auth {
            DatabricksAuth::Token(t) => Some(t.clone()),
            _ => None,
        }));

        let auth_method = AuthMethod::Custom(Box::new(DatabricksAuthProvider {
            auth: auth.clone(),
            token_cache: token_cache.clone(),
            oauth_token_provider,
            token_resolver,
        }));

        let mut api_client = ApiClient::with_timeout_and_tls(
            host.clone(),
            auth_method,
            Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS),
            tls_config,
        )?;
        if let Some(request_builder) = request_builder {
            api_client = api_client.with_request_builder(request_builder);
        }

        Ok(Self {
            api_client,
            host,
            auth,
            image_format: ImageFormat::OpenAi,
            retry_config,
            name: DATABRICKS_PROVIDER_NAME.to_string(),
            token_cache,
            instance_id,
            refresh_hook,
            session_id_provider,
        })
    }

    pub fn load_retry_config(get_param: impl Fn(&str) -> Option<String>) -> RetryConfig {
        let max_retries = get_param("DATABRICKS_MAX_RETRIES")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let initial_interval_ms = get_param("DATABRICKS_INITIAL_RETRY_INTERVAL_MS")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_INITIAL_RETRY_INTERVAL_MS);

        let backoff_multiplier = get_param("DATABRICKS_BACKOFF_MULTIPLIER")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(DEFAULT_BACKOFF_MULTIPLIER);

        let max_interval_ms = get_param("DATABRICKS_MAX_RETRY_INTERVAL_MS")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_RETRY_INTERVAL_MS);

        RetryConfig::new(
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        )
    }

    pub(crate) fn is_claude_model(model_name: &str) -> bool {
        model_name.to_lowercase().contains("claude")
    }

    pub(crate) fn is_reasoning_capable_model_name(model_name: &str) -> bool {
        Self::is_claude_model(model_name) || is_openai_responses_model(model_name)
    }

    pub(crate) fn resolve_vision_support(
        model_config: &ModelConfig,
        effective_model_name: &str,
    ) -> Option<ModelConfig> {
        if effective_model_name == model_config.model_name {
            return None;
        }

        let resolved = ModelConfig::new(effective_model_name)
            .with_canonical_vision_support(DATABRICKS_PROVIDER_NAME)
            .supports_vision?;

        if model_config.supports_vision == Some(resolved) {
            return None;
        }

        Some(model_config.clone().with_vision_support(resolved))
    }

    pub(crate) fn uses_responses_api(
        endpoint_info: Option<&DatabricksEndpointInfo>,
        model_names: &[&str],
    ) -> bool {
        match endpoint_info {
            Some(info) => info.supports_responses_api,
            None => model_names
                .iter()
                .any(|name| is_openai_responses_model(name)),
        }
    }

    fn get_endpoint_path(&self, model_name: &str, is_responses_model: bool) -> String {
        if is_responses_model {
            "serving-endpoints/responses".to_string()
        } else {
            let (clean_name, _) = extract_reasoning_effort(model_name);
            format!("serving-endpoints/{}/invocations", clean_name)
        }
    }

    fn build_client_request_id(&self, session_id: &str) -> Option<String> {
        self.instance_id.as_ref().map(|instance_id| {
            json!({
                "sessionId": format!("{}_{}", instance_id, session_id),
            })
            .to_string()
        })
    }
}

impl crate::base::ProviderDescriptor for DatabricksProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            DATABRICKS_PROVIDER_NAME,
            "Databricks",
            "Models on Databricks AI Gateway",
            DATABRICKS_DEFAULT_MODEL,
            DATABRICKS_KNOWN_MODELS.to_vec(),
            DATABRICKS_DOC_URL,
            vec![
                ConfigKey::new("DATABRICKS_HOST", true, false, None, true),
                ConfigKey::new("DATABRICKS_TOKEN", false, true, None, true),
            ],
        )
    }
}

#[async_trait]
impl Provider for DatabricksProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        crate::context_limit::ContextLimitResolver::new(self.get_name())
            .resolve(model, override_limit, || async {
                self.fetch_model_info(model)
                    .await
                    .map(|info| info.context_limit)
            })
            .await
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        if let Some(refresh_hook) = &self.refresh_hook {
            refresh_hook();
        }
        *self.token_cache.lock().unwrap() = None;
        tracing::info!("Invalidated secrets cache and token cache for credential refresh");
        Ok(())
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = self
            .session_id_provider
            .as_ref()
            .and_then(|provider| provider())
            .unwrap_or_default();
        let (endpoint_name, _) = extract_reasoning_effort(&model_config.model_name);
        let endpoint_info = self
            .resolve_endpoint_info_cached(&endpoint_name, EndpointMetadataLookup::InferenceRouting)
            .await
            .ok();
        let effective_model_name = endpoint_info
            .as_ref()
            .and_then(|info| info.upstream_model_name.as_deref())
            .unwrap_or(&model_config.model_name);
        let is_responses_model = Self::uses_responses_api(
            endpoint_info.as_ref(),
            &[&model_config.model_name, effective_model_name],
        );
        let vision_resolved_config =
            Self::resolve_vision_support(model_config, effective_model_name);
        let model_config = vision_resolved_config.as_ref().unwrap_or(model_config);
        let path = if is_responses_model {
            "serving-endpoints/responses".to_string()
        } else {
            self.get_endpoint_path(&model_config.model_name, is_responses_model)
        };
        let client_request_id = self.build_client_request_id(&session_id);

        if is_responses_model {
            let responses_model_config;
            let request_model_config = if effective_model_name != model_config.model_name {
                responses_model_config = {
                    let mut config = model_config.clone();
                    config.model_name = effective_model_name.to_string();
                    config
                };
                &responses_model_config
            } else {
                model_config
            };
            let mut payload =
                create_responses_request(request_model_config, system, messages, tools)?;
            payload["model"] = Value::String(endpoint_name.clone());
            if payload.get("reasoning").is_none() {
                if let Some(effort) = model_config.thinking_effort().and_then(|effort| {
                    openai_reasoning_effort_for_thinking(effective_model_name, effort)
                }) {
                    payload.as_object_mut().unwrap().insert(
                        "reasoning".to_string(),
                        json!({
                            "effort": effort,
                            "summary": "auto",
                        }),
                    );
                }
            }
            payload["stream"] = Value::Bool(true);
            if let Some(ref client_request_id) = client_request_id {
                payload["client_request_id"] = Value::String(client_request_id.clone());
            }

            let mut log = start_log(model_config, &payload)?;

            let response = self
                .with_retry(|| async {
                    let payload_clone = payload.clone();
                    let resp = self
                        .api_client
                        .request(&path)
                        .model_headers(model_config)?
                        .streaming(true)
                        .response_post(&payload_clone)
                        .await?;
                    handle_status(resp).await
                })
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            stream_responses_compat(response, log)
        } else {
            let format_model_config;
            let request_model_config = if Self::is_claude_model(effective_model_name)
                && !Self::is_claude_model(&model_config.model_name)
            {
                format_model_config = {
                    let mut config = model_config.clone();
                    config.model_name = effective_model_name.to_string();
                    config
                };
                &format_model_config
            } else {
                model_config
            };

            let mut payload = create_request_for_provider(
                DATABRICKS_PROVIDER_NAME,
                request_model_config,
                system,
                messages,
                tools,
                &self.image_format,
            )?;
            payload
                .as_object_mut()
                .expect("payload should have model key")
                .remove("model");
            if let Some(client_request_id) = client_request_id {
                payload["client_request_id"] = Value::String(client_request_id);
            }

            payload
                .as_object_mut()
                .unwrap()
                .insert("stream".to_string(), Value::Bool(true));

            if let Some(opts) = payload
                .get_mut("stream_options")
                .and_then(|v| v.as_object_mut())
            {
                opts.entry("include_usage").or_insert(json!(true));
            } else {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("stream_options".to_string(), json!({"include_usage": true}));
            }

            let mut log = start_log(model_config, &payload)?;
            let response = self
                .with_retry(|| async {
                    let resp = self
                        .api_client
                        .request(&path)
                        .model_headers(model_config)?
                        .streaming(true)
                        .response_post(&payload)
                        .await?;
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let url = sanitize_url(resp.url().as_str());
                        let error_text = crate::cloud::transport::http_status::read_error_body(resp)
                            .await
                            .unwrap_or_default();

                        let json_payload = serde_json::from_str::<Value>(&error_text).ok();
                        return Err(map_http_error_to_provider_error(status, json_payload, &url));
                    }
                    Ok(resp)
                })
                .await;

            let response = match response {
                Err(e) if e.to_string().contains("stream_options") => {
                    payload.as_object_mut().unwrap().remove("stream_options");
                    self.with_retry(|| async {
                        let resp = self
                            .api_client
                            .request(&path)
                            .model_headers(model_config)?
                            .streaming(true)
                            .response_post(&payload)
                            .await?;
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let url = sanitize_url(resp.url().as_str());
                            let error_text = crate::cloud::transport::http_status::read_error_body(resp)
                                .await
                                .unwrap_or_default();
                            let json_payload = serde_json::from_str::<Value>(&error_text).ok();
                            return Err(map_http_error_to_provider_error(
                                status,
                                json_payload,
                                &url,
                            ));
                        }
                        Ok(resp)
                    })
                    .await
                    .inspect_err(|e| {
                        let _ = log.error(e);
                    })?
                }
                Err(e) => {
                    let _ = log.error(&e);
                    return Err(e);
                }
                Ok(resp) => resp,
            };

            stream_openai_compat(response, log)
        }
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(self
            .fetch_supported_model_info()
            .await?
            .into_iter()
            .map(|model| model.name)
            .collect())
    }

    async fn fetch_supported_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let response = self
            .api_client
            .request("api/2.0/serving-endpoints")
            .response_get()
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to fetch Databricks models: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Failed to fetch Databricks models: {} {}",
                status, detail
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse Databricks API response: {}", e))
        })?;

        let endpoints = json
            .get("endpoints")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed(
                    "Unexpected response format from Databricks API: missing 'endpoints' array"
                        .to_string(),
                )
            })?;

        let mut models = Vec::new();
        for endpoint in endpoints {
            if let Some(endpoint_info) = Self::endpoint_info_from_value(endpoint) {
                models.push(Self::model_info_from_endpoint(endpoint_info));
            }
        }

        Ok(models)
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        let (endpoint_name, _) = extract_reasoning_effort(model_name);
        let endpoint_info = self
            .resolve_endpoint_info_cached(&endpoint_name, EndpointMetadataLookup::ContextDiscovery)
            .await?;
        Ok(Self::model_info_from_endpoint(endpoint_info))
    }

    async fn fetch_recommended_model_info(
        &self,
        _toolshim: bool,
    ) -> Result<Vec<ModelInfo>, ProviderError> {
        self.fetch_supported_model_info().await
    }
}

