//! Pricing Engine for Model Invocations
//!
//! Synthesized from 9Router (pricing.js) and Custos Domain economics.
//! Rates are stored in USD per 1 Million tokens.

use custos_domain::ModelPricing;
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

/// Normalizes a model identifier by lowercasing and stripping common revision/date suffixes.
pub fn normalize_model_id(model_id: &str) -> String {
    let lower = model_id.trim().to_lowercase();
    // Strip trailing "-latest", date patterns like "-2024-08-06" or "-preview"
    let without_latest = lower.strip_suffix("-latest").unwrap_or(&lower);
    let without_preview = without_latest
        .strip_suffix("-preview")
        .unwrap_or(without_latest);
    without_preview.to_string()
}

/// Resolves standard pricing rates for a known AI model into domain `ModelPricing`.
pub fn lookup_model_pricing(model_id: &str) -> Option<ModelPricing> {
    let normalized = normalize_model_id(model_id);

    // 1. Anthropic Claude models
    if normalized.contains("claude") {
        if normalized.contains("opus") {
            return Some(ModelPricing {
                input_cost_per_m: 5.0,
                output_cost_per_m: 25.0,
                cache_read_cost_per_m: Some(0.50),
                cache_write_cost_per_m: Some(6.25),
                currency: "USD".into(),
            });
        }
        if normalized.contains("haiku") {
            return Some(ModelPricing {
                input_cost_per_m: 0.80,
                output_cost_per_m: 4.0,
                cache_read_cost_per_m: Some(0.08),
                cache_write_cost_per_m: Some(1.0),
                currency: "USD".into(),
            });
        }
        // Sonnet default (Claude 3.5 Sonnet, Claude 3.7 Sonnet)
        return Some(ModelPricing {
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            cache_read_cost_per_m: Some(0.30),
            cache_write_cost_per_m: Some(3.75),
            currency: "USD".into(),
        });
    }

    // 2. OpenAI / Codex models
    if normalized.contains("gpt-4o-mini") || normalized.contains("gpt-5.4-mini") {
        return Some(ModelPricing {
            input_cost_per_m: 0.15,
            output_cost_per_m: 0.60,
            cache_read_cost_per_m: Some(0.075),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }
    if normalized.contains("gpt-4o")
        || normalized.contains("gpt-5.4")
        || normalized.contains("gpt-5")
    {
        return Some(ModelPricing {
            input_cost_per_m: 2.50,
            output_cost_per_m: 10.00,
            cache_read_cost_per_m: Some(1.25),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }
    if normalized.contains("o3-mini") || normalized.contains("o1-mini") {
        return Some(ModelPricing {
            input_cost_per_m: 1.10,
            output_cost_per_m: 4.40,
            cache_read_cost_per_m: Some(0.55),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }
    if normalized.contains("o1") || normalized.contains("o3") {
        return Some(ModelPricing {
            input_cost_per_m: 15.0,
            output_cost_per_m: 60.0,
            cache_read_cost_per_m: Some(7.50),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }

    // 3. DeepSeek models
    if normalized.contains("deepseek-reasoner") || normalized.contains("deepseek-r1") {
        return Some(ModelPricing {
            input_cost_per_m: 0.55,
            output_cost_per_m: 2.19,
            cache_read_cost_per_m: Some(0.14),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }
    if normalized.contains("deepseek-chat")
        || normalized.contains("deepseek-v3")
        || normalized.contains("deepseek")
    {
        return Some(ModelPricing {
            input_cost_per_m: 0.14,
            output_cost_per_m: 0.28,
            cache_read_cost_per_m: Some(0.014),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }

    // 4. Google Gemini models
    if normalized.contains("gemini-2.5-flash")
        || normalized.contains("gemini-1.5-flash")
        || normalized.contains("flash")
    {
        return Some(ModelPricing {
            input_cost_per_m: 0.075,
            output_cost_per_m: 0.30,
            cache_read_cost_per_m: Some(0.018),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }
    if normalized.contains("gemini") {
        return Some(ModelPricing {
            input_cost_per_m: 1.25,
            output_cost_per_m: 5.00,
            cache_read_cost_per_m: Some(0.30),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }

    // 5. Local / Ollama / Fake
    if normalized.contains("llama")
        || normalized.contains("qwen")
        || normalized.contains("mistral")
        || normalized.contains("local")
        || normalized.contains("mock")
    {
        return Some(ModelPricing {
            input_cost_per_m: 0.0,
            output_cost_per_m: 0.0,
            cache_read_cost_per_m: Some(0.0),
            cache_write_cost_per_m: Some(0.0),
            currency: "USD".into(),
        });
    }

    None
}

/// Calculates the estimated invocation cost in USD given token usage and model pricing.
pub fn calculate_token_cost(
    pricing: &ModelPricing,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
) -> f64 {
    let input_cost = (input_tokens as f64 / 1_000_000.0) * pricing.input_cost_per_m;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * pricing.output_cost_per_m;

    let cache_read_cost = match (cache_read_tokens, pricing.cache_read_cost_per_m) {
        (Some(tokens), Some(rate)) => (tokens as f64 / 1_000_000.0) * rate,
        _ => 0.0,
    };

    let cache_write_cost = match (cache_write_tokens, pricing.cache_write_cost_per_m) {
        (Some(tokens), Some(rate)) => (tokens as f64 / 1_000_000.0) * rate,
        _ => 0.0,
    };

    input_cost + output_cost + cache_read_cost + cache_write_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model_id() {
        assert_eq!(
            normalize_model_id("claude-3-7-sonnet-latest"),
            "claude-3-7-sonnet"
        );
        assert_eq!(normalize_model_id("GPT-4O-preview"), "gpt-4o");
    }

    #[test]
    fn test_lookup_pricing() {
        let sonnet = lookup_model_pricing("claude-3-7-sonnet").unwrap();
        assert_eq!(sonnet.input_cost_per_m, 3.0);
        assert_eq!(sonnet.output_cost_per_m, 15.0);

        let gpt4 = lookup_model_pricing("gpt-4o").unwrap();
        assert_eq!(gpt4.input_cost_per_m, 2.50);

        let deepseek = lookup_model_pricing("deepseek-chat").unwrap();
        assert_eq!(deepseek.input_cost_per_m, 0.14);

        let local = lookup_model_pricing("qwen2.5-coder:32b").unwrap();
        assert_eq!(local.input_cost_per_m, 0.0);
    }

    #[test]
    fn test_calculate_cost() {
        let pricing = ModelPricing {
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            cache_read_cost_per_m: Some(0.30),
            cache_write_cost_per_m: Some(3.75),
            currency: "USD".into(),
        };

        let cost = calculate_token_cost(&pricing, 100_000, 10_000, Some(50_000), None);
        assert!((cost - 0.465).abs() < 1e-6);
    }
}

