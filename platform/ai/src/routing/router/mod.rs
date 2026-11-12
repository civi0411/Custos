//! Unified Router Model Provider
//!
//! Routes model inference requests to live provider endpoints (OpenAI, Anthropic,
//! Gemini, DeepSeek, Local/Ollama) based on requested model name and stored credentials.

pub mod catalog;
pub(crate) mod classification;
pub mod credentials;
pub(crate) mod execution_backends;
pub(crate) mod generate_dispatch;
pub(crate) mod provider;
pub(crate) mod response_handler;
pub(crate) mod stream_dispatch;
pub(crate) mod wire_types;

#[cfg(test)]
mod tests;

pub use catalog::*;
pub use classification::{classify_provider, format_provider_error, parse_codex_sse_stream};
pub use credentials::*;
pub use provider::RouterModelProvider;
