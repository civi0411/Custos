//! Local inference adapters implementing `Provider` and `ModelProvider` (`ModelPort`).

#[allow(clippy::module_inception)]
pub mod adapter;
pub mod provider;
pub mod provider_utils;

pub use adapter::NativeLocalModelAdapter;
pub(crate) use provider::StreamSender;
pub use provider::{LocalInferenceProvider, LOCAL_LLM_MODEL_CONFIG_KEY};

