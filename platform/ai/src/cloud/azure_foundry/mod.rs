pub mod provider;
pub mod types;

pub use provider::AzureFoundryProvider;
pub use types::{
    endpoint_kind, is_project_endpoint, EndpointKind, ModelPublisher,
    AZURE_FOUNDRY_DEFAULT_MODEL, AZURE_FOUNDRY_DOC_URL, AZURE_FOUNDRY_KNOWN_MODELS,
    AZURE_FOUNDRY_PROVIDER_NAME,
};

#[allow(unused_imports)]
pub(crate) use types::{
    inference_route, model_info_for_deployment, with_api_version, DeploymentCache,
    DeploymentMetadata, DeploymentMetadataLookup, InferenceRoute,
};

#[cfg(test)]
mod tests;
