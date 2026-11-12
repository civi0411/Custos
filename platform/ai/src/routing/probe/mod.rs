//! Model Endpoint Probe
//!
//! Discovers the models and context window sizes a custom or remote AI endpoint serves.
//! Runs in Rust daemon/adapters to avoid webview CORS limitations and enforce strict timeout bounds.
//! Adapted from Open Science Desktop model probe and extended for Custos provider protocol.

pub mod cloud_probes;
pub mod metadata;
pub mod ollama;
pub mod quota;

#[cfg(test)]
mod tests;

pub use cloud_probes::{
    parse_chatgpt_codex_models, parse_gemini_models, parse_openai_models,
    parse_openai_models_for_transport, probe_chatgpt_codex_endpoint, probe_copilot_models,
    probe_endpoint_models, probe_oauth_models,
};
#[cfg(test)]
pub(crate) use cloud_probes::parse_provider_models;
pub use metadata::{
    advertised_default_effort, advertised_efforts, codex_user_agent, conversation_model_is_visible,
    model_is_selectable, verified_catalog_key, CODEX_CLIENT_VERSION,
};
pub use ollama::{ollama_root, parse_ollama_show_context, parse_ollama_tags};
pub use quota::{parse_account_quota, probe_account_quota};
