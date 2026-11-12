//! CUSTOS UNIFIED AI SUBSYSTEM
//!
//! Pillar 1: Model Intelligence, Connectivity, and Economics.
//!
//! Organizes model intelligence into canonical pillars:
//! - `contracts`: Zero-I/O contracts, ModelPort, ModelRequest, Turn, Thinking, Pricing
//! - `catalog`: Unified model catalog, capabilities, combo rules, pricing
//! - `cloud`: Live network API clients (OpenAI, Anthropic, Gemini, DeepSeek, Ollama, Declarative)
//! - `local`: On-device inference (GGUF inspector, llama.cpp, Metal probe, MLX)
//! - `routing`: Economics (429/402 Cooldown Filter, Fallback Cascade, Probes)

extern crate self as custos_provider;

pub mod contracts;
pub mod catalog;
pub mod cloud;
pub mod local;
pub mod routing;

#[allow(unused_imports)]
pub use contracts::*;

// Pillar convenience namespaces
pub use cloud as providers;
pub use local::local_inference;

// Prompt template loader convenience export
pub use local::local_inference::prompt_template;

pub mod model {
    pub use crate::contracts::types::model::*;
    pub use crate::local::local_inference::model::*;
}
