use super::super::*;
use crate::model::ModelConfig;
use rmcp::model::Tool;
use rmcp::object;
use serde_json::json;

    #[test]
    fn test_responses_tools_include_strict_false() {
        let model_config = ModelConfig {
            model_name: "gpt-5.4".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let tool = Tool::new(
            "shell",
            "Execute a shell command",
            object!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to run"
                    }
                },
                "required": ["command"]
            }),
        );

        let result =
            create_responses_request(&model_config, "You are helpful.", &[], &[tool]).unwrap();
        let tools = result["tools"]
            .as_array()
            .expect("tools should be an array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["strict"], json!(false),
            "Responses API defaults strict to true, but MCP tool schemas are not strict-compatible; must explicitly set strict: false");
    }

    #[test]
    fn test_responses_request_with_explicit_effort_suffix() {
        for (model_name, expected_model, expected_effort) in [
            ("gpt-5.4-xhigh", "gpt-5.4", "xhigh"),
            ("databricks-gpt-5.4-high", "databricks-gpt-5.4", "high"),
            ("databricks-o3-none", "databricks-o3", "none"),
        ] {
            let model_config = ModelConfig {
                model_name: model_name.to_string(),
                context_limit: None,
                temperature: None,
                max_tokens: None,
                toolshim: false,
                toolshim_model: None,
                request_params: None,
                reasoning: None,
                supports_vision: None,
                request_headers: None,
            };

            let result =
                create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

            assert_eq!(
                result["model"], expected_model,
                "unexpected model for {model_name}"
            );
            assert_eq!(
                result["reasoning"]["effort"], expected_effort,
                "unexpected effort for {model_name}"
            );
            assert_eq!(result["reasoning"]["summary"], "auto");
        }
    }

    #[test]
    fn test_responses_request_with_normalized_effort_suffix() {
        let model_config = ModelConfig::new("o3-mini-high");

        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

        assert_eq!(result["model"], "o3-mini");
        assert_eq!(result["reasoning"]["effort"], "high");
        assert_eq!(result["reasoning"]["summary"], "auto");
    }

    #[test]
    fn test_responses_request_supports_gpt_5_6_xhigh_effort() {
        let model_config = ModelConfig::new("gpt-5.6-sol-xhigh");

        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

        assert_eq!(result["model"], "gpt-5.6-sol");
        assert_eq!(result["reasoning"]["effort"], "xhigh");
        assert_eq!(result["reasoning"]["summary"], "auto");
    }

    #[test]
    fn test_responses_request_gpt6_astra_off_uses_low_not_none() {
        for model_name in ["gpt-6-astra", "data_workflow_tools.goose.goose-gpt-6-astra"] {
            let model_config = ModelConfig::new(model_name)
                .with_thinking_effort(crate::thinking::ThinkingEffort::Off);

            let result =
                create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

            assert_eq!(result["model"], model_name, "{model_name}");
            assert_eq!(
                result["reasoning"]["effort"], "low",
                "{model_name} Off should serialize as low, not none"
            );
        }

        let model_config = ModelConfig::new("gpt-5.6-luna")
            .with_thinking_effort(crate::thinking::ThinkingEffort::Off);
        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();
        assert_eq!(result["reasoning"]["effort"], "none");
    }

    #[test]
    fn test_responses_request_supports_gpt_5_6_reasoning_mode() {
        for model_name in [
            "gpt-5.6-sol",
            "openrouter/openai/gpt-5.6-sol",
            "vendor-gpt-5-6",
        ] {
            let model_config = ModelConfig::new(model_name).with_merged_request_params(
                std::collections::HashMap::from([("reasoning_mode".to_string(), json!("pro"))]),
            );

            let result =
                create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

            assert_eq!(result["reasoning"]["mode"], "pro");
            assert!(result["reasoning"].get("effort").is_none());
            assert!(result["reasoning"].get("summary").is_none());
        }
    }

    #[test]
    fn test_responses_request_rejects_reasoning_mode_for_non_gpt_5_6_model() {
        for model_name in ["gpt-5.5", "gpt-5.60", "notgpt-5.6", "gpt-5.6ish"] {
            let model_config = ModelConfig::new(model_name).with_merged_request_params(
                std::collections::HashMap::from([("reasoning_mode".to_string(), json!("pro"))]),
            );

            let error = create_responses_request(&model_config, "You are helpful.", &[], &[])
                .expect_err("reasoning mode should be gated to GPT-5.6 models");

            assert!(error
                .to_string()
                .contains("reasoning_mode is only supported for GPT-5.6 models"));
        }
    }

    #[test]
    fn test_responses_request_without_effort_suffix_omits_reasoning() {
        for model_name in ["gpt-5.4", "o3", "gpt-5-nano"] {
            let model_config = ModelConfig {
                model_name: model_name.to_string(),
                context_limit: None,
                temperature: None,
                max_tokens: None,
                toolshim: false,
                toolshim_model: None,
                request_params: None,
                reasoning: None,
                supports_vision: None,
                request_headers: None,
            };

            let result =
                create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

            assert_eq!(result["model"], model_name, "model should be unchanged");
            assert!(
                result.get("reasoning").is_none(),
                "reasoning should be omitted for {model_name} without explicit effort suffix"
            );
        }
    }

    #[test]
    fn test_responses_request_omits_default_max_output_tokens_for_unknown_model() {
        let model_config = ModelConfig::new("gpt-5.6-sol");

        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

        assert_eq!(result["model"], "gpt-5.6-sol");
        assert!(
            result.get("max_output_tokens").is_none(),
            "unknown/new models should not receive Goose's fallback max_output_tokens"
        );
    }

    #[test]
    fn test_responses_request_includes_canonical_max_output_tokens() {
        let model_config = ModelConfig::new("gpt-5.4").with_canonical_limits("openai");

        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

        assert_eq!(
            result["max_output_tokens"],
            model_config.max_tokens.unwrap()
        );
    }

    #[test]
    fn test_responses_request_non_reasoning_model_ignores_global_thinking_effort() {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", Some("high"))]);
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "You are helpful.", &[], &[]).unwrap();

        assert_eq!(result["model"], "gpt-4o");
        assert!(
            result.get("reasoning").is_none(),
            "non-reasoning models should not receive reasoning config"
        );
    }

    #[test]
    fn test_request_params_override_store() {
        let model_config = ModelConfig {
            model_name: "o3".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: Some(std::collections::HashMap::from([(
                "store".to_string(),
                serde_json::json!(true),
            )])),
            reasoning: None,
            supports_vision: None,
            request_headers: None,
        };

        let result = create_responses_request(&model_config, "", &[], &[]).unwrap();

        assert_eq!(result["store"], true);
    }

    #[test]
    fn test_user_image_serialized_in_responses_request() {
        use crate::conversation::message::Message;

        let messages = vec![Message::user()
            .with_text("describe this image")
            .with_image("aW1hZ2VkYXRh", "image/png")];

        let model_config = ModelConfig {
            model_name: "gpt-5.5".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            supports_vision: Some(true),
            request_headers: None,
        };

        let result =
            create_responses_request(&model_config, "You are helpful.", &messages, &[]).unwrap();

        let input = result["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);

        assert_eq!(input[0]["role"], "system");

        assert_eq!(input[1]["role"], "user");
        let content = input[1]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);

        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[0]["text"], "describe this image");

        assert_eq!(content[1]["type"], "input_image");
        assert_eq!(
            content[1]["image_url"],
            "data:image/png;base64,aW1hZ2VkYXRh"
        );
    }

