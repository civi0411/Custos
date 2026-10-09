//! Pricing Engine for Model Invocations
//!
//! Synthesized from 9Router (pricing.js) and Custos Domain economics.
//! Rates are stored in USD per 1 Million tokens.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingRate {
    /// Cost per 1M input tokens
    pub input_per_m: f64,
    /// Cost per 1M output tokens
    pub output_per_m: f64,
    /// Cost per 1M prompt cached tokens read
    pub cached_per_m: Option<f64>,
    /// Cost per 1M reasoning / thinking tokens (if billed differently)
    pub reasoning_per_m: Option<f64>,
    /// Cost per 1M cache creation tokens
    pub cache_creation_per_m: Option<f64>,
}

impl PricingRate {
    pub fn zero() -> Self {
        Self {
            input_per_m: 0.0,
            output_per_m: 0.0,
            cached_per_m: Some(0.0),
            reasoning_per_m: Some(0.0),
            cache_creation_per_m: Some(0.0),
        }
    }

    /// Calculate turn cost in USD based on actual token usage.
    pub fn calculate_cost(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
    ) -> f64 {
        let cached = cached_tokens.unwrap_or(0);
        let pure_input = input_tokens.saturating_sub(cached);
        let cached_rate = self.cached_per_m.unwrap_or(self.input_per_m);
        let reasoning = reasoning_tokens.unwrap_or(0);
        let pure_output = output_tokens.saturating_sub(reasoning);
        let reasoning_rate = self.reasoning_per_m.unwrap_or(self.output_per_m);

        let input_cost = (pure_input as f64 / 1_000_000.0) * self.input_per_m;
        let cached_cost = (cached as f64 / 1_000_000.0) * cached_rate;
        let output_cost = (pure_output as f64 / 1_000_000.0) * self.output_per_m;
        let reasoning_cost = (reasoning as f64 / 1_000_000.0) * reasoning_rate;

        input_cost + cached_cost + output_cost + reasoning_cost
    }
}

/// Lookup model pricing rate based on model name, inspired by 9Router pricing.js
pub fn get_model_pricing(model: &str) -> PricingRate {
    let lower = model.to_lowercase();

    // Free models / local inference
    if lower.starts_with("local") || lower.contains("free") || lower.starts_with("mock") {
        return PricingRate::zero();
    }

    // Anthropic Claude
    if lower.contains("claude-3-7-sonnet") || lower.contains("claude-sonnet-4") {
        return PricingRate {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cached_per_m: Some(0.30),
            reasoning_per_m: Some(15.00),
            cache_creation_per_m: Some(3.75),
        };
    }
    if lower.contains("claude-3-5-sonnet") {
        return PricingRate {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cached_per_m: Some(0.30),
            reasoning_per_m: Some(15.00),
            cache_creation_per_m: Some(3.75),
        };
    }
    if lower.contains("claude-3-5-haiku") || lower.contains("claude-haiku-4") {
        return PricingRate {
            input_per_m: 0.80,
            output_per_m: 4.00,
            cached_per_m: Some(0.08),
            reasoning_per_m: Some(4.00),
            cache_creation_per_m: Some(1.00),
        };
    }
    if lower.contains("claude-3-opus") || lower.contains("claude-opus-4") {
        return PricingRate {
            input_per_m: 15.00,
            output_per_m: 75.00,
            cached_per_m: Some(1.50),
            reasoning_per_m: Some(75.00),
            cache_creation_per_m: Some(18.75),
        };
    }

    // OpenAI models
    if lower.contains("gpt-4o-mini") {
        return PricingRate {
            input_per_m: 0.15,
            output_per_m: 0.60,
            cached_per_m: Some(0.075),
            reasoning_per_m: None,
            cache_creation_per_m: None,
        };
    }
    if lower.contains("gpt-4o") {
        return PricingRate {
            input_per_m: 2.50,
            output_per_m: 10.00,
            cached_per_m: Some(1.25),
            reasoning_per_m: None,
            cache_creation_per_m: None,
        };
    }
    if lower.contains("o1-mini") {
        return PricingRate {
            input_per_m: 1.10,
            output_per_m: 4.40,
            cached_per_m: Some(0.55),
            reasoning_per_m: Some(4.40),
            cache_creation_per_m: None,
        };
    }
    if lower.contains("o1") {
        return PricingRate {
            input_per_m: 15.00,
            output_per_m: 60.00,
            cached_per_m: Some(7.50),
            reasoning_per_m: Some(60.00),
            cache_creation_per_m: None,
        };
    }
    if lower.contains("o3-mini") {
        return PricingRate {
            input_per_m: 1.10,
            output_per_m: 4.40,
            cached_per_m: Some(0.55),
            reasoning_per_m: Some(4.40),
            cache_creation_per_m: None,
        };
    }

    // Google Gemini
    if lower.contains("gemini-2.0-flash") || lower.contains("gemini-2.5-flash") {
        return PricingRate {
            input_per_m: 0.10,
            output_per_m: 0.40,
            cached_per_m: Some(0.025),
            reasoning_per_m: Some(0.40),
            cache_creation_per_m: None,
        };
    }
    if lower.contains("gemini-1.5-pro") || lower.contains("gemini-2.5-pro") {
        return PricingRate {
            input_per_m: 1.25,
            output_per_m: 5.00,
            cached_per_m: Some(0.3125),
            reasoning_per_m: Some(5.00),
            cache_creation_per_m: None,
        };
    }

    // DeepSeek
    if lower.contains("deepseek-reasoner") || lower.contains("deepseek-r1") {
        return PricingRate {
            input_per_m: 0.55,
            output_per_m: 2.19,
            cached_per_m: Some(0.14),
            reasoning_per_m: Some(2.19),
            cache_creation_per_m: None,
        };
    }
    if lower.contains("deepseek-chat") || lower.contains("deepseek-v3") {
        return PricingRate {
            input_per_m: 0.27,
            output_per_m: 1.10,
            cached_per_m: Some(0.07),
            reasoning_per_m: None,
            cache_creation_per_m: None,
        };
    }

    // Fallback baseline standard tier
    PricingRate {
        input_per_m: 1.00,
        output_per_m: 3.00,
        cached_per_m: Some(0.25),
        reasoning_per_m: Some(3.00),
        cache_creation_per_m: None,
    }
}
