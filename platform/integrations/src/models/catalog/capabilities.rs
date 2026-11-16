//! Model Capabilities and Matrix Engine
//!
//! Synthesized from 9Router (capabilities.js) and Custos ModelPort specifications.
//! Determines modalities, reasoning support, tool-calling abilities, and limits.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub pdf: bool,
    pub tools: bool,
    pub reasoning: bool,
    pub search: bool,
    pub context_window: u64,
    pub max_output: u64,
    pub thinking_format: Option<String>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            vision: false,
            pdf: false,
            tools: true,
            reasoning: false,
            search: false,
            context_window: 128_000,
            max_output: 4_096,
            thinking_format: None,
        }
    }
}

/// Lookup model capabilities by name, inspired by 9Router capabilities.js
pub fn get_model_capabilities(model: &str) -> ModelCapabilities {
    let lower = model.to_lowercase();

    // 1. Anthropic Claude 3.7 Sonnet / 3.5 Sonnet
    if lower.contains("claude-3-7-sonnet") || lower.contains("claude-sonnet-4") {
        return ModelCapabilities {
            vision: true,
            pdf: true,
            tools: true,
            reasoning: true,
            search: true,
            context_window: 200_000,
            max_output: 64_000,
            thinking_format: Some("claude-budget".into()),
        };
    }
    if lower.contains("claude-3-5-sonnet") {
        return ModelCapabilities {
            vision: true,
            pdf: true,
            tools: true,
            reasoning: false,
            search: false,
            context_window: 200_000,
            max_output: 8_192,
            thinking_format: None,
        };
    }
    if lower.contains("claude-3-5-haiku") {
        return ModelCapabilities {
            vision: false,
            pdf: false,
            tools: true,
            reasoning: false,
            search: false,
            context_window: 200_000,
            max_output: 8_192,
            thinking_format: None,
        };
    }

    // 2. OpenAI o-series & GPT-4o
    if lower.contains("o1") || lower.contains("o3-mini") {
        return ModelCapabilities {
            vision: true,
            pdf: false,
            tools: true,
            reasoning: true,
            search: true,
            context_window: 200_000,
            max_output: 100_000,
            thinking_format: Some("openai".into()),
        };
    }
    if lower.contains("gpt-4o") {
        return ModelCapabilities {
            vision: true,
            pdf: false,
            tools: true,
            reasoning: false,
            search: true,
            context_window: 128_000,
            max_output: 16_384,
            thinking_format: None,
        };
    }

    // 3. Google Gemini
    if lower.contains("gemini-2.0-flash") || lower.contains("gemini-2.5") {
        return ModelCapabilities {
            vision: true,
            pdf: true,
            tools: true,
            reasoning: true,
            search: true,
            context_window: 1_048_576,
            max_output: 65_536,
            thinking_format: Some("gemini-budget".into()),
        };
    }
    if lower.contains("gemini-1.5-pro") {
        return ModelCapabilities {
            vision: true,
            pdf: true,
            tools: true,
            reasoning: false,
            search: true,
            context_window: 2_097_152,
            max_output: 8_192,
            thinking_format: None,
        };
    }

    // 4. DeepSeek
    if lower.contains("deepseek-reasoner") || lower.contains("deepseek-r1") {
        return ModelCapabilities {
            vision: false,
            pdf: false,
            tools: true,
            reasoning: true,
            search: false,
            context_window: 64_000,
            max_output: 8_192,
            thinking_format: Some("deepseek".into()),
        };
    }
    if lower.contains("deepseek-chat") || lower.contains("deepseek-v3") {
        return ModelCapabilities {
            vision: false,
            pdf: false,
            tools: true,
            reasoning: false,
            search: false,
            context_window: 64_000,
            max_output: 8_192,
            thinking_format: None,
        };
    }

    ModelCapabilities::default()
}
