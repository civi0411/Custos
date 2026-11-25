//! Google Cloud CodeAssist / Gemini Transport Adapter
//!
//! Provides live transport for Google Cloud CodeAssist and Gemini inference,
//! streaming, and model discovery.

pub mod inference;
pub mod models;
pub mod project;
pub mod transport;

#[cfg(test)]
mod tests;

pub use inference::{generate_codeassist_content, generate_content, stream_codeassist};
pub use models::{probe_codeassist_models, probe_models, supports_tiered_reasoning};
pub use project::discover_project;
