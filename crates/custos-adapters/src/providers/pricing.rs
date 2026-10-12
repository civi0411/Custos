//! Model Token Pricing Engine
//!
//! Re-exports canonical token rates and cost calculations from `custos_provider::catalog::pricing`.
//! The Single Source of Truth for model rates resides in `custos-provider`.

pub use custos_provider::catalog::pricing::{
    calculate_token_cost, lookup_model_pricing, normalize_model_id,
};

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::ModelPricing;

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
