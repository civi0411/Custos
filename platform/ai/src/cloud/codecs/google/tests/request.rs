use super::super::*;
use super::super::request::{get_thinking_config, ThinkingLevel, DEFAULT_THINKING_BUDGET};
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use serde_json::json;
use std::collections::HashMap;

    #[test]
    fn test_get_thinking_config_disabled_reasoning() {
        use crate::model::ModelConfig;

        let config = ModelConfig::new("gemini-2.5-flash").with_thinking_effort(ThinkingEffort::Off);
        let thinking_config = get_thinking_config(&config, None).unwrap();
        assert_eq!(thinking_config.thinking_budget, Some(0));
        assert!(!thinking_config.include_thoughts);

        let config = ModelConfig::new("gemini-2.5-pro").with_thinking_effort(ThinkingEffort::Off);
        assert!(get_thinking_config(&config, None).is_none());

        let config =
            ModelConfig::new("gemini-3.5-flash-lite").with_thinking_effort(ThinkingEffort::Off);
        let thinking_config = get_thinking_config(&config, None).unwrap();
        assert!(matches!(
            thinking_config.thinking_level,
            Some(ThinkingLevel::Minimal)
        ));
        assert!(!thinking_config.include_thoughts);
    }

    #[test]
    fn test_get_thinking_config() {
        use crate::model::ModelConfig;

        // Test 1: Gemini 3 model with low thinking effort
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("low"));
        let mut config = ModelConfig::new("gemini-3-pro");
        config.request_params = Some(params);
        let result = get_thinking_config(&config, None);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert!(thinking_config.thinking_level.is_some());
        assert!(thinking_config.thinking_budget.is_none());
        assert!(thinking_config.include_thoughts);

        let config =
            ModelConfig::new("gemini-3.6-flash").with_thinking_effort(ThinkingEffort::Medium);
        let thinking_config = get_thinking_config(&config, None).unwrap();
        assert!(matches!(
            thinking_config.thinking_level,
            Some(ThinkingLevel::Medium)
        ));

        let config =
            ModelConfig::new("gemini-3-pro-preview").with_thinking_effort(ThinkingEffort::Medium);
        let thinking_config = get_thinking_config(&config, None).unwrap();
        assert!(matches!(
            thinking_config.thinking_level,
            Some(ThinkingLevel::Low)
        ));

        // Test 2: Gemini 3 model with high thinking effort
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("high"));
        let mut config = ModelConfig::new("Gemini-3-Flash");
        config.request_params = Some(params);
        let result = get_thinking_config(&config, None);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert!(matches!(
            thinking_config.thinking_level,
            Some(ThinkingLevel::High)
        ));

        let config = ModelConfig::new("gemini-2.5-flash");
        let result = get_thinking_config(&config, None);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert!(thinking_config.include_thoughts);
        assert!(thinking_config.thinking_level.is_none());
        assert_eq!(
            thinking_config.thinking_budget,
            Some(DEFAULT_THINKING_BUDGET)
        );

        let mut params = HashMap::new();
        params.insert("thinking_budget".to_string(), json!(4096));
        let config = ModelConfig::new("gemini-2.5-flash").with_merged_request_params(params);
        let result = get_thinking_config(&config, None);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert_eq!(thinking_config.thinking_budget, Some(4096));

        let mut params = HashMap::new();
        params.insert("thinking_budget".to_string(), json!(-1));
        let config = ModelConfig::new("gemini-2.5-flash").with_merged_request_params(params);
        let result = get_thinking_config(&config, None);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert_eq!(
            thinking_config.thinking_budget,
            Some(DEFAULT_THINKING_BUDGET)
        );

        let config = ModelConfig::new("gemini-2.0-flash");
        let result = get_thinking_config(&config, None);
        assert!(result.is_none());

        let config = ModelConfig::new("gpt-4o");
        let result = get_thinking_config(&config, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_gemini_3_request_omits_temperature() {
        let config = ModelConfig::new("gemini-3.6-flash").with_temperature(Some(0.2));
        let payload = create_request(&config, "system", &[], &[]).unwrap();

        assert!(payload["generationConfig"].get("temperature").is_none());

}
