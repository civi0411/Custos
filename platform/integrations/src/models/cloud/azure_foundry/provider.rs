use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;

use super::types::{
    configured_client, endpoint_kind, inference_route, is_project_endpoint,
    model_info_for_deployment, with_api_version, DeploymentCache, DeploymentMetadata,
    DeploymentMetadataLookup, EndpointKind, InferenceRoute, ModelPublisher,
    AZURE_FOUNDRY_DEFAULT_MODEL, AZURE_FOUNDRY_DOC_URL, AZURE_FOUNDRY_KNOWN_MODELS,
    AZURE_FOUNDRY_PROVIDER_NAME, DEPLOYMENT_METADATA_TIMEOUT_SECS,
};
use crate::cloud::anthropic::{AnthropicProvider, AnthropicProviderBuilder, ANTHROPIC_API_VERSION};
use crate::cloud::transport::api_client::{ApiClient, AuthMethod, RequestBuilderDecorator, TlsConfig};
use crate::base::{
    ConfigKey, MessageStream, ModelInfo, Provider, ProviderDescriptor, ProviderMetadata,
};
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::model::ModelConfig;
use crate::cloud::openai::{OpenAiProvider, OpenAiProviderBuilder};
use crate::cloud::transport::openai_compatible::{handle_response_openai_compat, OpenAiCompatibleProvider};

pub struct AzureFoundryProvider {
    pub(crate) chat: OpenAiCompatibleProvider,
    pub(crate) responses: Option<OpenAiProvider>,
    pub(crate) anthropic: Option<AnthropicProvider>,
    pub(crate) deployments_client: ApiClient,
    pub(crate) endpoint: String,
    pub(crate) api_version: Option<String>,
    pub(crate) maas_model: Option<String>,
    pub(crate) deployments: Mutex<DeploymentCache>,
}

impl ProviderDescriptor for AzureFoundryProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AZURE_FOUNDRY_PROVIDER_NAME,
            "Azure AI Foundry",
            "OpenAI, Anthropic, and partner models deployed through Azure AI Foundry",
            AZURE_FOUNDRY_DEFAULT_MODEL,
            AZURE_FOUNDRY_KNOWN_MODELS.to_vec(),
            AZURE_FOUNDRY_DOC_URL,
            vec![
                ConfigKey::new("AZURE_FOUNDRY_ENDPOINT", true, false, None, true),
                ConfigKey::new("AZURE_FOUNDRY_API_KEY", false, true, Some(""), true),
                ConfigKey::new("AZURE_FOUNDRY_AD_TOKEN", false, true, Some(""), false),
                ConfigKey::new("AZURE_FOUNDRY_MODEL", false, false, None, true),
                ConfigKey::new("AZURE_FOUNDRY_API_VERSION", false, false, None, false),
            ],
        )
    }
}

impl AzureFoundryProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        endpoint: String,
        api_version: Option<String>,
        maas_model: Option<String>,
        chat_auth: AuthMethod,
        responses_auth: AuthMethod,
        anthropic_auth: AuthMethod,
        deployments_auth: AuthMethod,
        tls_config: Option<TlsConfig>,
        request_builder: Option<RequestBuilderDecorator>,
    ) -> Result<Self> {
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let endpoint_kind = endpoint_kind(&endpoint);
        let native_inference = endpoint_kind != EndpointKind::Maas;
        let maas_model = if native_inference {
            None
        } else {
            Some(
                maas_model
                    .filter(|model| !model.trim().is_empty())
                    .ok_or_else(|| {
                        anyhow::anyhow!("AZURE_FOUNDRY_MODEL is required for MaaS endpoints")
                    })?
                    .trim()
                    .to_string(),
            )
        };
        let chat_prefix = if native_inference {
            "openai/v1/"
        } else {
            "v1/"
        };

        let chat_client = configured_client(
            endpoint.clone(),
            chat_auth,
            tls_config.clone(),
            request_builder.clone(),
        )?;
        let chat = OpenAiCompatibleProvider::new(
            AZURE_FOUNDRY_PROVIDER_NAME.to_string(),
            chat_client,
            chat_prefix.to_string(),
        );

        let (responses, anthropic) = if native_inference {
            let responses_client = configured_client(
                endpoint.clone(),
                responses_auth,
                tls_config.clone(),
                request_builder.clone(),
            )?;
            let responses = OpenAiProviderBuilder::new(responses_client)
                .name(AZURE_FOUNDRY_PROVIDER_NAME)
                .base_path("openai/v1/responses")
                .skip_canonical_filtering(true)
                .build();

            let hub = endpoint.split("/api/projects/").next().unwrap_or(&endpoint);
            let anthropic_client = configured_client(
                format!("{hub}/anthropic"),
                anthropic_auth,
                tls_config.clone(),
                request_builder.clone(),
            )?
            .with_header("anthropic-version", ANTHROPIC_API_VERSION)?;
            let anthropic = AnthropicProviderBuilder::new(anthropic_client)
                .name(AZURE_FOUNDRY_PROVIDER_NAME)
                .skip_canonical_filtering(true)
                .build();
            (Some(responses), Some(anthropic))
        } else {
            (None, None)
        };

        let deployments_client = configured_client(
            endpoint.clone(),
            deployments_auth,
            tls_config,
            request_builder,
        )?;

        Ok(Self {
            chat,
            responses,
            anthropic,
            deployments_client,
            endpoint,
            api_version,
            maas_model,
            deployments: Mutex::new(DeploymentCache::default()),
        })
    }

    pub(crate) async fn fetch_deployments(
        &self,
    ) -> Result<(Vec<String>, HashMap<String, DeploymentMetadata>), ProviderError> {
        let version = self
            .api_version
            .as_deref()
            .or_else(|| is_project_endpoint(&self.endpoint).then_some("v1"));
        let mut next = Some(match version {
            Some(version) => format!("deployments?api-version={version}"),
            None => "deployments".to_string(),
        });
        let mut models = Vec::new();
        let mut deployments = HashMap::new();

        while let Some(path) = next {
            let response = self
                .deployments_client
                .response_get(&path)
                .await
                .map_err(|error| ProviderError::NetworkError(error.to_string()))?;
            let json = handle_response_openai_compat(response).await?;
            if let Some(items) = json.get("value").and_then(|value| value.as_array()) {
                for item in items {
                    let Some(name) = item.get("name").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    let model_name = item
                        .get("modelName")
                        .and_then(|value| value.as_str())
                        .unwrap_or(name);
                    let publisher = item
                        .get("modelPublisher")
                        .and_then(|value| value.as_str())
                        .map(ModelPublisher::from_azure)
                        .unwrap_or_else(|| ModelPublisher::from_model_name(model_name));
                    models.push(name.to_string());
                    deployments.insert(
                        name.to_string(),
                        DeploymentMetadata {
                            publisher,
                            model_name: model_name.to_string(),
                        },
                    );
                }
            }
            next = json
                .get("nextLink")
                .and_then(|value| value.as_str())
                .map(|link| with_api_version(link, version));
        }

        models.sort();
        models.dedup();
        Ok((models, deployments))
    }

    pub(crate) async fn deployment_for(
        &self,
        deployment_name: &str,
        lookup: DeploymentMetadataLookup,
    ) -> Option<DeploymentMetadata> {
        {
            let cache = self
                .deployments
                .lock()
                .expect("Azure Foundry deployment cache poisoned");
            if cache.applies_to(lookup) {
                return cache.deployments.get(deployment_name).cloned();
            }
        }

        let fetch_result = tokio::time::timeout(
            Duration::from_secs(DEPLOYMENT_METADATA_TIMEOUT_SECS),
            self.fetch_deployments(),
        )
        .await
        .ok()
        .and_then(Result::ok);
        let fetch_failed = fetch_result.is_none();
        let deployments = fetch_result
            .map(|(_, deployments)| deployments)
            .unwrap_or_default();
        let deployment = deployments.get(deployment_name).cloned();
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = DeploymentCache {
            deployments,
            fetched_at: Some(Instant::now()),
            fetch_failed,
        };
        deployment
    }
}

#[async_trait]
impl Provider for AzureFoundryProvider {
    fn get_name(&self) -> &str {
        AZURE_FOUNDRY_PROVIDER_NAME
    }

    fn skip_canonical_filtering(&self) -> bool {
        true
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        self.deployments_client
            .refresh_credentials()
            .await
            .map_err(|error| ProviderError::Authentication(error.to_string()))
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(model) = &self.maas_model {
            return Ok(vec![model.clone()]);
        }
        if !is_project_endpoint(&self.endpoint) {
            return Ok(AZURE_FOUNDRY_KNOWN_MODELS
                .iter()
                .map(ToString::to_string)
                .collect());
        }
        let (models, deployments) = self.fetch_deployments().await?;
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = DeploymentCache {
            deployments,
            fetched_at: Some(Instant::now()),
            fetch_failed: false,
        };
        Ok(models)
    }

    async fn fetch_supported_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        if let Some(model) = &self.maas_model {
            return Ok(vec![model_info_for_deployment(model, model)]);
        }
        if !is_project_endpoint(&self.endpoint) {
            return Ok(AZURE_FOUNDRY_KNOWN_MODELS
                .iter()
                .map(|model| model_info_for_deployment(model, model))
                .collect());
        }
        let (models, deployments) = self.fetch_deployments().await?;
        let model_info = models
            .iter()
            .filter_map(|name| {
                deployments
                    .get(name)
                    .map(|deployment| model_info_for_deployment(name, &deployment.model_name))
            })
            .collect();
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = DeploymentCache {
            deployments,
            fetched_at: Some(Instant::now()),
            fetch_failed: false,
        };
        Ok(model_info)
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        let resolved_model = if let Some(model) = &self.maas_model {
            model.clone()
        } else if is_project_endpoint(&self.endpoint) {
            self.deployment_for(model_name, DeploymentMetadataLookup::ContextDiscovery)
                .await
                .map(|deployment| deployment.model_name)
                .unwrap_or_else(|| model_name.to_string())
        } else {
            model_name.to_string()
        };
        Ok(model_info_for_deployment(model_name, &resolved_model))
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        custos_provider::context_limit::ContextLimitResolver::new(self.get_name())
            .resolve(model, override_limit, || async {
                self.fetch_model_info(model)
                    .await
                    .map(|info| info.context_limit)
            })
            .await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let maas_config = self.maas_model.as_ref().map(|model| {
            let mut config = model_config.clone();
            config.model_name = model.clone();
            config
        });
        let model_config = maas_config.as_ref().unwrap_or(model_config);
        let wire_model = self
            .maas_model
            .clone()
            .unwrap_or_else(|| model_config.model_name.clone());
        let deployment = if is_project_endpoint(&self.endpoint) {
            self.deployment_for(&wire_model, DeploymentMetadataLookup::InferenceRouting)
                .await
        } else {
            None
        };
        let publisher = deployment
            .as_ref()
            .map(|deployment| deployment.publisher)
            .unwrap_or_else(|| ModelPublisher::from_model_name(&model_config.model_name));
        let underlying_model = deployment
            .as_ref()
            .map(|deployment| deployment.model_name.as_str())
            .unwrap_or(&model_config.model_name);
        let route = inference_route(self.responses.is_some(), publisher, underlying_model);
        let capability_model = deployment
            .as_ref()
            .map(|deployment| deployment.model_name.as_str())
            .unwrap_or(&model_config.model_name);
        let mut capability_config = model_config.clone();
        capability_config.model_name = capability_model.to_string();
        let capability_config =
            capability_config.with_canonical_limits(AZURE_FOUNDRY_PROVIDER_NAME);
        match route {
            InferenceRoute::ProjectResponses => {
                self.responses
                    .as_ref()
                    .expect("checked above")
                    .stream_for_model(
                        &capability_config,
                        &wire_model,
                        capability_model,
                        system,
                        messages,
                        tools,
                    )
                    .await
            }
            InferenceRoute::AnthropicMessages => {
                self.anthropic
                    .as_ref()
                    .expect("checked above")
                    .stream_for_model(&capability_config, &wire_model, system, messages, tools)
                    .await
            }
            InferenceRoute::MaasChatCompletions | InferenceRoute::ProjectChatCompletions => {
                self.chat
                    .stream_for_model(
                        &capability_config,
                        &wire_model,
                        capability_model,
                        system,
                        messages,
                        tools,
                    )
                    .await
            }
        }
    }
}
