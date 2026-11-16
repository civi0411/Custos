pub mod builder;
pub mod options;
pub mod provider;
pub mod streaming;

#[cfg(test)]
mod tests;

pub use builder::{from_declarative_config, OllamaProviderBuilder};
pub use options::{
    OllamaOptions, OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS, OLLAMA_DEFAULT_MODEL, OLLAMA_DEFAULT_PORT,
    OLLAMA_DOC_URL, OLLAMA_HOST, OLLAMA_KNOWN_MODELS, OLLAMA_PROVIDER_NAME, OLLAMA_TIMEOUT,
};
pub use provider::{fetch_ollama_model_names, OllamaProvider};
