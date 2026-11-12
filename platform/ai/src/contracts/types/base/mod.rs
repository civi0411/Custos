//! Core Provider trait, metadata, configuration, and streaming types.

pub mod metadata;
pub mod provider_trait;
pub mod stream;

pub use metadata::{
    ConfigKey, ModelInfo, ProviderDeprecation, ProviderDescriptor, ProviderMetadata,
    ThinkingPreservationFormat,
};
pub use provider_trait::{
    known_models_from_registry, model_info_for_provider_model, Provider,
};
pub use stream::{
    collect_stream, stream_from_single_message, MessageStream, PermissionRouting,
};

#[cfg(test)]
mod tests;
