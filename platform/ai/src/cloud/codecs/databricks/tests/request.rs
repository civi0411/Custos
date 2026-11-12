use super::super::*;
use crate::conversation::message::Message;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use serde_json::json;

    #[test]
    fn test_create_request_gpt_4o() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "system"
                }
            ],
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("high"));
        let model_config = ModelConfig {
            model_name: "o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: Some(params),
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "high");
        Ok(())
    }

    #[test]
    fn test_create_request_off_effort_uses_low() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("off"));
        let model_config = ModelConfig {
            model_name: "databricks-o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: Some(params),
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "low");
        assert!(request.get("thinking_effort").is_none());
        Ok(())
    }

    #[test]
    fn test_create_request_max_effort_uses_supported_level() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("max"));
        let model_config = ModelConfig {
            model_name: "databricks-gpt-5.2-pro".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: Some(params),
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "high");
        assert!(request.get("thinking_effort").is_none());
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_xhigh() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "o3-xhigh".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "o3");
        assert_eq!(request["reasoning_effort"], "xhigh");
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_none() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "o3-none".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "o3");
        assert_eq!(request["reasoning_effort"], "none");
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_for_prefixed_gpt5_model() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "databricks-gpt-5.4-high".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "databricks-gpt-5.4");
        assert_eq!(request["reasoning_effort"], "high");
        Ok(())
    }

    #[test]
    fn test_create_request_one_shot_claude() -> anyhow::Result<()> {
        let model_config = ModelConfig::new("databricks-claude-sonnet-4-5")
            .with_merged_request_params(std::collections::HashMap::from([
                ("anthropic_beta".to_string(), serde_json::json!(["ctx-1m"])),
                ("disable_prompt_cache".to_string(), serde_json::json!(true)),
            ]));
        let messages = vec![Message::user().with_text("Summarize the conversation above.")];

        let request = create_request(
            &model_config,
            "system",
            &messages,
            &[],
            &ImageFormat::OpenAi,
        )?;

        assert_eq!(request["anthropic_beta"], serde_json::json!(["ctx-1m"]));
        assert!(request.get("disable_prompt_cache").is_none());
        assert!(!request.to_string().contains("cache_control"));

        Ok(())
    }

    #[test]
    fn test_create_request_adaptive_thinking_for_46_models() -> anyhow::Result<()> {
        let mut model_config = ModelConfig::new("databricks-claude-opus-4-6");
        model_config.max_tokens = Some(4096);
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("low"));
        model_config.request_params = Some(params);

        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

        assert_eq!(request["thinking"]["type"], "adaptive");
        assert_eq!(request["output_config"]["effort"], "low");
        assert!(request.get("temperature").is_none());
        assert_eq!(request["max_completion_tokens"], 4096);
        assert!(request.get("max_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_adaptive_thinking_for_new_anthropic_models() -> anyhow::Result<()> {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

        for name in [
            "databricks-claude-opus-4-7",
            "databricks-claude-opus-4-8",
            "databricks-claude-fable-5",
            "global.anthropic.claude-fable-5",
        ] {
            let mut model_config = ModelConfig::new(name);
            model_config.max_tokens = Some(4096);
            let mut params = std::collections::HashMap::new();
            params.insert("thinking_effort".to_string(), serde_json::json!("high"));
            model_config.request_params = Some(params);

            let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

            assert_eq!(request["thinking"]["type"], "adaptive", "{name}");
            assert!(request.get("temperature").is_none(), "{name}");
            assert_eq!(request["max_completion_tokens"], 4096, "{name}");
            assert!(request.get("max_tokens").is_none(), "{name}");
        }

        Ok(())
    }

    #[test]
    fn test_create_request_always_on_adaptive_off_effort_falls_back_to_high() -> anyhow::Result<()>
    {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
        let mut model_config = ModelConfig::new("databricks-claude-fable-5");
        model_config.max_tokens = Some(4096);
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("off"));
        model_config.request_params = Some(params);

        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

        assert_eq!(request["thinking"]["type"], "adaptive");
        assert_eq!(request["output_config"]["effort"], "high");

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_with_budget() -> anyhow::Result<()> {
        let mut model_config = ModelConfig::new("databricks-claude-sonnet-4.5");
        model_config.max_tokens = Some(4096);
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("high"));
        model_config.request_params = Some(params);

        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

        assert_eq!(request["thinking"]["type"], "enabled");
        assert_eq!(request["thinking"]["budget_tokens"], 16000);
        assert_eq!(request["max_tokens"], 20096);
        assert_eq!(request["temperature"], 2);
        assert!(request.get("max_completion_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_budget_tracks_effort() -> anyhow::Result<()> {
        for (effort, expected_budget) in [
            ("low", 4000),
            ("medium", 10000),
            ("high", 16000),
            ("max", 32000),
        ] {
            let mut model_config = ModelConfig::new("databricks-claude-sonnet-4.5");
            model_config.max_tokens = Some(4096);
            let mut params = std::collections::HashMap::new();
            params.insert("thinking_effort".to_string(), serde_json::json!(effort));
            model_config.request_params = Some(params);

            let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

            assert_eq!(request["thinking"]["type"], "enabled");
            assert_eq!(request["thinking"]["budget_tokens"], expected_budget);
            assert_eq!(request["max_tokens"], 4096 + expected_budget);
        }

        Ok(())
    }
