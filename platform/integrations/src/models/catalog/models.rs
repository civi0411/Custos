//! Model Descriptor and Limits
//!
//! Canonical representation of model capabilities, context limits, and modalities.
//! Synthesized from 9Router catalog and models.dev specifications.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextLimits {
    pub context_window: u64,
    pub max_output: u64,
}

impl Default for ContextLimits {
    fn default() -> Self {
        Self {
            context_window: 128_000,
            max_output: 4_096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Modalities {
    pub vision: bool,
    pub pdf: bool,
    pub audio_input: bool,
    pub video_input: bool,
    pub image_output: bool,
    pub audio_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingFormat {
    OpenAi,
    ClaudeAdaptive,
    ClaudeBudget,
    GeminiBudget,
    DeepSeek,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub provider_family: String,
    pub display_name: String,
    pub limits: ContextLimits,
    pub modalities: Modalities,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub default_thinking_format: Option<ThinkingFormat>,
}
