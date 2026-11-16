use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::cloud::transport::api_client::{ApiClient, AuthMethod, RequestBuilderDecorator, TlsConfig};
use crate::base::ModelInfo;
use crate::cloud::codecs::openai::{extract_reasoning_effort, is_openai_responses_model};
use crate::model::ModelConfig;

pub const AZURE_FOUNDRY_PROVIDER_NAME: &str = "azure_foundry";
pub const AZURE_FOUNDRY_DEFAULT_MODEL: &str = "Phi-4";
pub const AZURE_FOUNDRY_DOC_URL: &str =
    "https://learn.microsoft.com/azure/ai-foundry/foundry-models/how-to/inference";

pub(crate) const DEPLOYMENT_METADATA_TIMEOUT_SECS: u64 = 5;
pub(crate) const DEPLOYMENT_METADATA_TTL_SECS: u64 = 60;

pub const AZURE_FOUNDRY_KNOWN_MODELS: &[&str] = &[
    "Phi-4",
    "Phi-4-mini",
    "Meta-Llama-3.3-70B-Instruct",
    "Mistral-large-2411",
    "Cohere-command-r-plus-08-2024",
    "AI21-Jamba-1.5-Large",
    "DeepSeek-R1",
    "DeepSeek-V3",
    "glm-4.7",
    "Kimi-K2-Instruct",
    "claude-sonnet-4-6",
    "claude-opus-4-6",
    "gpt-5",
    "o3",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointKind {
    Maas,
    Resource,
    Project,
}

pub fn endpoint_kind(endpoint: &str) -> EndpointKind {
    if endpoint.contains("/api/projects/") {
        EndpointKind::Project
    } else if endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .is_some_and(|host| host.ends_with(".services.ai.azure.com"))
    {
        EndpointKind::Resource
    } else {
        EndpointKind::Maas
    }
}

pub fn is_project_endpoint(endpoint: &str) -> bool {
    endpoint_kind(endpoint) == EndpointKind::Project
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPublisher {
    OpenAi,
    Anthropic,
    Partner,
}

impl ModelPublisher {
    pub(crate) fn from_azure(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "openai" => Self::OpenAi,
            "anthropic" => Self::Anthropic,
            _ => Self::Partner,
        }
    }

    pub(crate) fn from_model_name(value: &str) -> Self {
        let value = value.to_ascii_lowercase();
        if value.starts_with("claude") {
            Self::Anthropic
        } else if is_openai_responses_model(&value) {
            Self::OpenAi
        } else {
            Self::Partner
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeploymentMetadata {
    pub(crate) publisher: ModelPublisher,
    pub(crate) model_name: String,
}

#[derive(Default)]
pub(crate) struct DeploymentCache {
    pub(crate) deployments: HashMap<String, DeploymentMetadata>,
    pub(crate) fetched_at: Option<Instant>,
    pub(crate) fetch_failed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeploymentMetadataLookup {
    ContextDiscovery,
    InferenceRouting,
}

impl DeploymentCache {
    pub(crate) fn applies_to(&self, lookup: DeploymentMetadataLookup) -> bool {
        self.fetched_at.is_some_and(|fetched_at| {
            fetched_at.elapsed() < Duration::from_secs(DEPLOYMENT_METADATA_TTL_SECS)
        }) && (lookup == DeploymentMetadataLookup::ContextDiscovery || !self.fetch_failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InferenceRoute {
    MaasChatCompletions,
    ProjectChatCompletions,
    ProjectResponses,
    AnthropicMessages,
}

pub(crate) fn inference_route(
    project_endpoint: bool,
    publisher: ModelPublisher,
    underlying_model: &str,
) -> InferenceRoute {
    if !project_endpoint {
        return InferenceRoute::MaasChatCompletions;
    }
    match publisher {
        ModelPublisher::Anthropic => InferenceRoute::AnthropicMessages,
        ModelPublisher::OpenAi if is_openai_responses_model(underlying_model) => {
            InferenceRoute::ProjectResponses
        }
        ModelPublisher::OpenAi | ModelPublisher::Partner => InferenceRoute::ProjectChatCompletions,
    }
}

pub(crate) fn with_api_version(link: &str, api_version: Option<&str>) -> String {
    let Some(api_version) = api_version else {
        return link.to_string();
    };
    if link.contains("api-version=") {
        return link.to_string();
    }
    let separator = if link.contains('?') { '&' } else { '?' };
    format!("{link}{separator}api-version={api_version}")
}

pub(crate) fn model_info_for_deployment(deployment_name: &str, model_name: &str) -> ModelInfo {
    let canonical = crate::catalog::canonical::maybe_get_canonical_model("azure_foundry", model_name)
        .or_else(|| {
            crate::catalog::canonical::maybe_get_canonical_model(
                "azure_foundry",
                &model_name.to_ascii_lowercase(),
            )
        })
        .or_else(|| {
            let (base_model, effort) = extract_reasoning_effort(model_name);
            effort.and_then(|_| {
                crate::catalog::canonical::maybe_get_canonical_model("azure_foundry", &base_model).or_else(
                    || {
                        crate::catalog::canonical::maybe_get_canonical_model(
                            "azure_foundry",
                            &base_model.to_ascii_lowercase(),
                        )
                    },
                )
            })
        });
    ModelInfo {
        name: deployment_name.to_string(),
        resolved_model: Some(model_name.to_string()),
        context_limit: canonical.as_ref().map(|model| model.limit.context),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning: canonical
            .and_then(|model| model.reasoning)
            .unwrap_or_else(|| ModelConfig::new(model_name).is_reasoning_model()),
        thinking_preservation_format: None,
        request_params: None,
    }
}

pub(crate) fn configured_client(
    host: String,
    auth: AuthMethod,
    tls_config: Option<TlsConfig>,
    request_builder: Option<RequestBuilderDecorator>,
) -> Result<ApiClient> {
    let restrict_redirects = matches!(&auth, AuthMethod::ApiKey { .. });
    let mut client = ApiClient::new_with_tls(host, auth, tls_config)?;
    if restrict_redirects {
        client = client.with_same_origin_redirects()?;
    }
    if let Some(request_builder) = request_builder {
        client = client.with_request_builder(request_builder);
    }
    Ok(client)
}
