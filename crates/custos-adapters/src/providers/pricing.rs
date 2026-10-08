//! Model Token Pricing Engine
//!
//! Provides canonical token rates (in USD per Million tokens) and cost calculations for
//! LLM invocations, cache reads, and prompt cache writes.
//! Synthesized from OrCa (claude-model-pricing, codex-model-pricing) and Open Science Desktop (model_prices).

use custos_domain::ModelPricing;

/// Normalizes a model identifier by lowercasing and stripping common revision/date suffixes.
pub fn normalize_model_id(model_id: &str) -> String {
    let lower = model_id.trim().to_lowercase();
    // Strip trailing "-latest", date patterns like "-2024-08-06" or "-preview"
    let without_latest = lower.strip_suffix("-latest").unwrap_or(&lower);
    let without_preview = without_latest.strip_suffix("-preview").unwrap_or(without_latest);
    without_preview.to_string()
}

/// Resolves standard pricing rates for a known AI model.
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
    if normalized.contains("gpt-4o") || normalized.contains("gpt-5.4") || normalized.contains("gpt-5") {
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
    if normalized.contains("deepseek-chat") || normalized.contains("deepseek-v3") || normalized.contains("deepseek") {
        return Some(ModelPricing {
            input_cost_per_m: 0.14,
            output_cost_per_m: 0.28,
            cache_read_cost_per_m: Some(0.014),
            cache_write_cost_per_m: None,
            currency: "USD".into(),
        });
    }

    // 4. Google Gemini models
    if normalized.contains("gemini-2.5-flash") || normalized.contains("gemini-1.5-flash") || normalized.contains("flash") {
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
    if normalized.contains("llama") || normalized.contains("qwen") || normalized.contains("mistral") || normalized.contains("local") {
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
        assert_eq!(normalize_model_id("claude-3-7-sonnet-latest"), "claude-3-7-sonnet");
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

        // 100k input, 10k output, 50k cached read
        let cost = calculate_token_cost(&pricing, 100_000, 10_000, Some(50_000), None);
        // input: 0.1 * 3.0 = 0.30
        // output: 0.01 * 15.0 = 0.15
        // cache: 0.05 * 0.30 = 0.015
        // total: 0.465
        assert!((cost - 0.465).abs() < 1e-6);
    }
}
