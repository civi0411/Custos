use super::super::*;
use super::helpers::*;
use crate::images::ImageFormat;
use crate::thinking::ThinkingEffort;
use serde_json::json;

#[test]
fn test_create_request_gpt_4o() -> anyhow::Result<()> {
    // Test default medium reasoning effort for O3 model
    let model_config = test_model_config("gpt-4o").with_max_tokens(Some(1024));
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();
    let expected = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": "system"
            }
        ],
        "max_tokens": 1024
    });

    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(obj.get(key).unwrap(), value);
    }

    Ok(())
}

#[test]
fn test_create_request_omits_max_tokens_when_unset() -> anyhow::Result<()> {
    // Unknown models on OpenAI-compatible local providers (llama_swap,
    // lmstudio) have no canonical record and no GOOSE_MAX_TOKENS, so the
    // request must not pin the legacy 4096 default. See issue #9007.
    let model_config = test_model_config("some-unknown-local-model");
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();
    assert!(
        !obj.contains_key("max_tokens"),
        "max_tokens should be omitted when model_config.max_tokens is None"
    );
    assert!(
        !obj.contains_key("max_completion_tokens"),
        "max_completion_tokens should be omitted when model_config.max_tokens is None"
    );
    Ok(())
}

#[test]
fn test_request_params_preserve_reserved_fields() -> anyhow::Result<()> {
    let params = std::collections::HashMap::from([
        (
            "thinking".to_string(),
            json!({
                "type": "enabled",
                "clear_thinking": false
            }),
        ),
        ("stream".to_string(), json!(false)),
        (
            "stream_options".to_string(),
            json!({"include_usage": false}),
        ),
        ("model".to_string(), json!("wrong-model")),
        ("messages".to_string(), json!([])),
        ("max_tokens".to_string(), json!(1)),
        ("temperature".to_string(), json!(2.0)),
        ("provider_custom".to_string(), json!("allowed")),
        ("thinking_effort".to_string(), json!("high")),
        ("disable_prompt_cache".to_string(), json!(true)),
        ("preserve_thinking_context".to_string(), json!(true)),
    ]);
    let model_config = test_model_config("glm-4.7")
        .with_max_tokens(Some(4096))
        .with_merged_request_params(params);

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    assert_eq!(
        request["thinking"],
        json!({
            "type": "enabled",
            "clear_thinking": false
        })
    );
    assert_eq!(request["stream"], true);
    assert_eq!(request["stream_options"], json!({"include_usage": true}));
    assert_eq!(request["model"], "glm-4.7");
    assert_eq!(request["messages"][0]["role"], "system");
    assert_eq!(request["max_tokens"], 1);
    assert_eq!(request["temperature"], 2.0);
    assert_eq!(request["provider_custom"], "allowed");
    assert!(request.get("thinking_effort").is_none());
    assert!(request.get("disable_prompt_cache").is_none());
    assert!(request.get("preserve_thinking_context").is_none());

    Ok(())
}

#[test]
fn test_create_request_o1_default() -> anyhow::Result<()> {
    let model_config = test_model_config("o1").with_max_tokens(Some(1024));
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();
    let expected = json!({
        "model": "o1",
        "messages": [
            {
                "role": "developer",
                "content": "system"
            }
        ],
        "max_completion_tokens": 1024
    });

    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(obj.get(key).unwrap(), value);
    }
    assert!(
        obj.get("reasoning_effort").is_none(),
        "reasoning_effort should be omitted when no explicit suffix is provided"
    );

    Ok(())
}

#[test]
fn test_create_request_o1_medium_effort() -> anyhow::Result<()> {
    let model_config = test_model_config("o1")
        .with_max_tokens(Some(1024))
        .with_thinking_effort(ThinkingEffort::Medium);
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("medium")));
    assert!(obj.get("thinking_effort").is_none());

    Ok(())
}

#[test]
fn test_create_request_gpt56_max_effort_uses_xhigh() -> anyhow::Result<()> {
    let model_config = test_model_config("gpt-5.6-luna")
        .with_max_tokens(Some(1024))
        .with_thinking_effort(ThinkingEffort::Max);
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("xhigh")));
    assert!(obj.get("thinking_effort").is_none());

    Ok(())
}

#[test]
fn test_openai_reasoning_effort_gpt6_sol_and_luna() {
    for model in ["gpt-6-sol", "gpt-6-luna"] {
        assert_eq!(
            openai_reasoning_effort_for_thinking(model, ThinkingEffort::Off),
            Some("none".to_string())
        );
        assert_eq!(
            openai_reasoning_effort_for_thinking(model, ThinkingEffort::Max),
            Some("max".to_string())
        );
        assert_eq!(
            openai_reasoning_efforts_for_model(model),
            &["none", "low", "medium", "high", "xhigh", "max"]
        );
        assert!(
            is_openai_responses_model(model),
            "{model} uses /v1/responses"
        );
    }

    assert_eq!(
        openai_reasoning_effort_for_thinking("gpt-6-astra", ThinkingEffort::Max),
        Some("max".to_string())
    );
    assert_eq!(
        openai_reasoning_effort_for_thinking("gpt-5.6-sol", ThinkingEffort::Max),
        Some("xhigh".to_string())
    );
}

#[test]
fn test_openai_reasoning_effort_gpt6_does_not_support_none() {
    for model in [
        "gpt-6-astra",
        "data_workflow_tools.goose.goose-gpt-6-astra",
        "openrouter/openai/gpt-6-astra",
    ] {
        assert_eq!(
            openai_reasoning_effort_for_thinking(model, ThinkingEffort::Off),
            Some("low".to_string()),
            "{model} Off should map to low, not none"
        );
        assert_eq!(
            openai_reasoning_effort_for_thinking(model, ThinkingEffort::Medium),
            Some("medium".to_string()),
            "{model} medium should remain medium"
        );
        assert!(
            !openai_reasoning_efforts_for_model(model).contains(&"none"),
            "{model} must not advertise none"
        );
    }

    assert_eq!(
        openai_reasoning_effort_for_thinking("gpt-5.6-luna", ThinkingEffort::Off),
        Some("none".to_string())
    );
    assert_eq!(
        openai_reasoning_effort_for_thinking("gpt-5.4", ThinkingEffort::Off),
        Some("none".to_string())
    );
    assert_eq!(
        openai_reasoning_effort_for_thinking("gpt-5", ThinkingEffort::Off),
        Some("low".to_string())
    );
}

#[test]
fn test_create_request_gpt5_pro_max_effort_uses_supported_level() -> anyhow::Result<()> {
    let model_config = test_model_config("gpt-5.2-pro-2025-12-11")
        .with_max_tokens(Some(1024))
        .with_thinking_effort(ThinkingEffort::Max);
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();

    assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
    assert!(obj.get("thinking_effort").is_none());

    Ok(())
}

#[test]
fn test_create_request_o3_custom_reasoning_effort() -> anyhow::Result<()> {
    let model_config = test_model_config("o3-mini")
        .with_max_tokens(Some(1024))
        .with_thinking_effort(ThinkingEffort::High);
    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )?;
    let obj = request.as_object().unwrap();
    let expected = json!({
        "model": "o3-mini",
        "messages": [
            {
                "role": "developer",
                "content": "system"
            }
        ],
        "reasoning_effort": "high",
        "max_completion_tokens": 1024
    });

    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(obj.get(key).unwrap(), value);
    }
    assert!(obj.get("thinking_effort").is_none());

    Ok(())
}

