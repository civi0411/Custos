use super::gemini_compat::apply_gemini_compatibility;
use crate::cloud::transport::api_client::ApiClient;
use crate::base::{ConfigKey, MessageStream, Provider, ProviderDescriptor, ProviderMetadata};
use crate::cache_semantics::{apply_chat_payload_breakpoints, CacheSemantics};
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::cloud::codecs::openai::create_request;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::cloud::transport::openai_compatible::{handle_status, stream_openai_compat};
use crate::cloud::codecs::openrouter as openrouter_format;
use crate::request_log::{start_log, LoggerHandleExt};
use crate::retry::ProviderRetry;
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::collections::HashMap;

pub const OPENROUTER_PROVIDER_NAME: &str = "openrouter";
pub(crate) const OPENROUTER_PARAMETERS_CONFIG_KEY: &str = "OPENROUTER_PARAMETERS";
pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4";
pub const OPENROUTER_DECISION_DEFAULT_MODEL: &str = "typesafe/jev-1.13";

pub const OPENROUTER_KNOWN_MODELS: &[&str] = &[
    "x-ai/grok-code-fast-1",
    "anthropic/claude-sonnet-4.5",
    "anthropic/claude-sonnet-4",
    "anthropic/claude-opus-4.1",
    "anthropic/claude-opus-4",
    "google/gemini-2.5-pro",
    "google/gemini-2.5-flash",
    "deepseek/deepseek-r1-0528",
    "qwen/qwen3-coder",
    "moonshotai/kimi-k2",
];
pub const OPENROUTER_DOC_URL: &str = "https://openrouter.ai/models";

pub type OpenRouterSessionIdProvider = Box<dyn Fn() -> Option<String> + Send + Sync>;

#[derive(serde::Serialize)]
pub struct OpenRouterProvider {
    #[serde(skip)]
    pub(crate) api_client: ApiClient,
    pub(crate) supports_streaming: bool,
    #[serde(skip)]
    pub(crate) name: String,
    #[serde(skip)]
    pub(crate) configured_parameters: Option<HashMap<String, Value>>,
    #[serde(skip)]
    pub(crate) session_id_provider: Option<OpenRouterSessionIdProvider>,
}

impl OpenRouterProvider {
    pub fn new(
        api_client: ApiClient,
        configured_parameters: Option<HashMap<String, Value>>,
        session_id_provider: Option<OpenRouterSessionIdProvider>,
    ) -> Self {
        Self {
            api_client,
            supports_streaming: true,
            name: OPENROUTER_PROVIDER_NAME.to_string(),
            configured_parameters,
            session_id_provider,
        }
    }

    pub(crate) async fn post_chat_completions(
        &self,
        model_config: &ModelConfig,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        self.with_retry(|| async {
            let resp = self
                .api_client
                .request("api/v1/chat/completions")
                .model_headers(model_config)?
                .streaming(true)
                .response_post(payload)
                .await?;
            handle_status(resp).await
        })
        .await
    }
}

pub(crate) fn is_mandatory_reasoning_error(error: &ProviderError) -> bool {
    matches!(error, ProviderError::RequestFailed(message) if message.contains("Reasoning is mandatory"))
}

pub(crate) fn merge_request_params(
    request_params: &mut Option<HashMap<String, Value>>,
    params: HashMap<String, Value>,
) {
    request_params
        .get_or_insert_with(HashMap::new)
        .extend(params);
}

pub(crate) fn merge_openrouter_parameters(model: &mut ModelConfig, params: HashMap<String, Value>) {
    merge_request_params(&mut model.request_params, params);
}

impl ProviderDescriptor for OpenRouterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OPENROUTER_PROVIDER_NAME,
            "OpenRouter",
            "Router for many model providers",
            OPENROUTER_DEFAULT_MODEL,
            OPENROUTER_KNOWN_MODELS.to_vec(),
            OPENROUTER_DOC_URL,
            vec![
                ConfigKey::new("OPENROUTER_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "OPENROUTER_HOST",
                    false,
                    false,
                    Some("https://openrouter.ai"),
                    false,
                ),
                ConfigKey::new(OPENROUTER_PARAMETERS_CONFIG_KEY, false, false, None, false),
            ],
        )
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        true
    }

    async fn fetch_recommended_models(&self, toolshim: bool) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request("api/v1/models")
            .response_get()
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to fetch models from OpenRouter API: {}",
                    e
                ))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse OpenRouter API response as JSON: {}",
                e
            ))
        })?;

        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::RequestFailed(format!(
                "OpenRouter API returned an error: {}",
                msg
            )));
        }

        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|model| {
                let id = model.get("id").and_then(|v| v.as_str())?;
                if toolshim {
                    return Some(id.to_string());
                }
                let supports_tools = model
                    .get("supported_parameters")
                    .and_then(|v| v.as_array())
                    .is_some_and(|params| params.iter().any(|p| p.as_str() == Some("tools")));
                if supports_tools {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        models.sort();
        Ok(models)
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

        let mut merged_model;
        let model_config = if let Some(params) = &self.configured_parameters {
            merged_model = model_config.clone();
            merge_openrouter_parameters(&mut merged_model, params.clone());
            &merged_model
        } else {
            model_config
        };

        let mut payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;

        if !session_id.is_empty() {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("user".to_string(), Value::String(session_id.to_string()));
                obj.insert(
                    "session_id".to_string(),
                    Value::String(session_id.to_string()),
                );
            }
        }

        if CacheSemantics::for_model(OPENROUTER_PROVIDER_NAME, &model_config.model_name)
            .uses_explicit_breakpoints()
            && !model_config.prompt_cache_disabled()
        {
            apply_chat_payload_breakpoints(&mut payload);
        }

        apply_gemini_compatibility(&model_config.model_name, &mut payload, messages);
        let sent_reasoning_disable =
            openrouter_format::apply_reasoning_config(&mut payload, model_config);

        if let Some(obj) = payload.as_object_mut() {
            obj.insert("transforms".to_string(), json!(["middle-out"]));
            obj.insert("usage".to_string(), json!({ "include": true }));
        }

        let mut log = start_log(model_config, &payload)?;

        let response = match self.post_chat_completions(model_config, &payload).await {
            // Mandatory-reasoning endpoints reject the disable request, so
            // downgrade to the lowest effort they all accept and retry once.
            Err(error) if sent_reasoning_disable && is_mandatory_reasoning_error(&error) => {
                let _ = log.error(&error);
                payload["reasoning"] = json!({ "effort": "low" });
                log = start_log(model_config, &payload)?;
                self.post_chat_completions(model_config, &payload).await
            }
            result => result,
        }
        .inspect_err(|e| {
            let _ = log.error(e);
        })?;

        stream_openai_compat(response, log)
    }
}
