use super::super::*;
use super::helpers::*;
use crate::conversation::message::{Message, MessageContentBlock};
use anyhow::Result;
use serde_json::json;
#[test]
fn test_create_request_adaptive_thinking_for_46_models() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

    let mut params = std::collections::HashMap::new();
    params.insert("thinking_effort".to_string(), json!("high"));

    let mut config = cfg("claude-opus-4-6");
    config.max_tokens = Some(4096);
    config.request_params = Some(params);
    let messages = vec![Message::user().with_text("Hello")];
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"]["type"], "adaptive");
    assert_eq!(payload["output_config"]["effort"], "high");
    assert!(payload.get("budget_tokens").is_none());

    Ok(())
}

#[test]
fn test_create_request_adaptive_thinking_for_opus_5() -> Result<()> {
    // Claude 5 models reject the legacy thinking.type=enabled shape and
    // require thinking.type=adaptive + output_config.effort.
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

    let mut params = std::collections::HashMap::new();
    params.insert("thinking_effort".to_string(), json!("high"));

    let mut config = cfg("claude-opus-5");
    config.max_tokens = Some(4096);
    config.request_params = Some(params);
    let messages = vec![Message::user().with_text("Hello")];
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"]["type"], "adaptive");
    assert_eq!(payload["output_config"]["effort"], "high");
    assert!(payload["thinking"].get("budget_tokens").is_none());

    Ok(())
}

#[test]
fn test_create_request_enabled_thinking_with_budget() -> Result<()> {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
    ]);

    let mut config = cfg_with_effort("claude-sonnet-4-5-20250929", "high");
    config.max_tokens = Some(64000);

    let messages = vec![Message::user().with_text("Hello")];
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"]["type"], "enabled");
    let budget = payload["thinking"]["budget_tokens"].as_i64().unwrap();
    assert!(budget > 0);
    assert_eq!(payload["max_tokens"], 64000);
    assert!(budget < 64000);

    Ok(())
}

#[test]
fn test_create_request_clamps_thinking_budget_to_fit_max_tokens() -> Result<()> {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
    ]);

    let mut config = cfg_with_effort("claude-sonnet-4-5-20250929", "high");
    let messages = vec![Message::user().with_text("Hello")];

    // Budget larger than max_tokens is clamped to leave room for a response.
    config.max_tokens = Some(4096);
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;
    let budget = payload["thinking"]["budget_tokens"].as_i64().unwrap();
    assert!(budget >= 1024);
    assert!(budget <= 4096 - 1024);
    assert_eq!(payload["max_tokens"], 4096);

    // Too small to fit any thinking alongside a response — drop it.
    config.max_tokens = Some(1500);
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;
    assert!(payload.get("thinking").is_none());
    assert_eq!(payload["max_tokens"], 1500);

    Ok(())
}

#[test]
fn test_create_request_disabled_thinking_no_thinking_field() -> Result<()> {
    let _guard = env_lock::lock_env([
        ("GOOSE_THINKING_EFFORT", None::<&str>),
        ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
    ]);

    let config = cfg_with_effort("claude-sonnet-4-20250514", "off");
    let messages = vec![Message::user().with_text("Hello")];
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert!(payload.get("thinking").is_none());
    assert!(payload.get("output_config").is_none());

    // Adaptive models treat an omitted field as adaptive, so off must be explicit.
    let config = cfg_with_effort("claude-opus-5", "off");
    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"], json!({"type": "disabled"}));

    Ok(())
}

#[test]
fn test_create_request_preserves_thinking_context_for_compatible_models() -> Result<()> {
    let _guard = env_lock::lock_env([
        ("CLAUDE_THINKING_ENABLED", None::<&str>),
        ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
    ]);

    let mut config = cfg("glm-4.7");
    config.max_tokens = Some(64000);
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::user().with_text("Continue"),
    ];

    let payload = create_request_with_options_provider(
        &config,
        "system",
        &messages,
        &[],
        AnthropicFormatOptions {
            preserve_unsigned_thinking: true,
            preserve_thinking_context: true,
            emit_clear_thinking: true,
            ..Default::default()
        },
    )?;

    assert_eq!(payload["thinking"]["type"], "enabled");
    assert!(payload["thinking"]["budget_tokens"].as_i64().unwrap() >= 1024);
    assert_eq!(payload["thinking"]["clear_thinking"], false);
    assert_eq!(payload["max_tokens"], 64000);
    assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
    assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");
    assert!(payload["messages"][0]["content"][0]
        .get("signature")
        .is_none());

    // Preserved context still wins on models that need an explicit thinking disable.
    let mut config = cfg("claude-opus-5");
    config.max_tokens = Some(64000);
    let payload = create_request_with_options_provider(
        &config,
        "system",
        &messages,
        &[],
        AnthropicFormatOptions {
            preserve_thinking_context: true,
            ..Default::default()
        },
    )?;

    assert_eq!(payload["thinking"]["type"], "enabled");

    Ok(())
}

#[test]
fn test_enabled_thinking_without_optin_omits_clear_thinking() -> Result<()> {
    let mut config = cfg("claude-sonnet-4-5-20250929");
    config.max_tokens = Some(64000);
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::user().with_text("Continue"),
    ];

    let payload = create_request_with_options_provider(
        &config,
        "system",
        &messages,
        &[],
        AnthropicFormatOptions {
            preserve_unsigned_thinking: true,
            preserve_thinking_context: true,
            emit_clear_thinking: false,
            ..Default::default()
        },
    )?;

    assert_eq!(payload["thinking"]["type"], "enabled");
    assert!(
        payload["thinking"].get("clear_thinking").is_none(),
        "Anthropic enabled thinking must not carry clear_thinking, got {}",
        payload["thinking"]
    );

    Ok(())
}

#[test]
fn test_create_request_model_params_enable_preserved_thinking_context() -> Result<()> {
    let _guard = env_lock::lock_env([
        ("CLAUDE_THINKING_ENABLED", None::<&str>),
        ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
    ]);

    let mut params = std::collections::HashMap::new();
    params.insert("preserve_thinking_context".to_string(), json!(true));
    params.insert("emit_clear_thinking".to_string(), json!(true));

    let mut config = cfg("glm-4.7");
    config.request_params = Some(params);
    config.max_tokens = Some(64000);
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::user().with_text("Continue"),
    ];

    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"]["clear_thinking"], false);
    assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
    assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");

    Ok(())
}

#[test]
fn test_adaptive_model_preserved_thinking_omits_clear_thinking() -> Result<()> {
    let mut config = cfg_with_effort("claude-opus-4-8", "high");
    config.max_tokens = Some(64000);
    let messages = vec![
        Message::assistant().with_content(MessageContentBlock::thinking("internal", "")),
        Message::user().with_text("Continue"),
    ];

    let payload = create_request_with_options_provider(
        &config,
        "system",
        &messages,
        &[],
        AnthropicFormatOptions {
            preserve_unsigned_thinking: true,
            preserve_thinking_context: true,
            emit_clear_thinking: false,
            ..Default::default()
        },
    )?;

    assert_eq!(payload["thinking"]["type"], "adaptive");
    assert!(
        payload["thinking"].get("clear_thinking").is_none(),
        "adaptive thinking must not carry clear_thinking, got {}",
        payload["thinking"]
    );
    assert_eq!(payload["output_config"]["effort"], "high");

    Ok(())
}

#[test]
fn test_thinking_type_from_effort() {
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
    // Adaptive model with effort → adaptive
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-4-6", "high")),
        ThinkingType::Adaptive
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-4-7", "high")),
        ThinkingType::Adaptive
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-4-8", "high")),
        ThinkingType::Adaptive
    );
    // Adaptive model with off → disabled
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
        ThinkingType::Disabled
    );
    // Non-adaptive Claude with effort → enabled
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-sonnet-4-5-20250929", "high")),
        ThinkingType::Enabled
    );
    // Non-adaptive Claude with off → disabled
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-sonnet-4-5-20250929", "off")),
        ThinkingType::Disabled
    );
}

#[test]
fn test_thinking_type_always_on_adaptive() {
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

    assert_eq!(
        thinking_type(&cfg("claude-fable-5")),
        ThinkingType::Adaptive
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-fable-5", "off")),
        ThinkingType::Adaptive
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-fable-5", "high")),
        ThinkingType::Adaptive
    );
}

#[test]
fn test_thinking_type_opus_5_5_cannot_disable_thinking() {
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

    assert_eq!(
        thinking_type(&cfg("claude-opus-5-5")),
        ThinkingType::Adaptive
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-5-5", "off")),
        ThinkingType::Adaptive
    );
}

#[test]
fn test_create_request_fable_5_omits_temperature() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
    let mut config = cfg("claude-fable-5");
    config.max_tokens = Some(4096);
    config.temperature = Some(0.7);
    let messages = vec![Message::user().with_text("Hello")];

    let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

    assert_eq!(payload["thinking"]["type"], "adaptive");
    assert!(payload.get("temperature").is_none());
    assert_eq!(payload["output_config"]["effort"], "high");

    Ok(())
}

#[test]
fn test_thinking_type_non_claude_always_disabled() {
    assert_eq!(
        thinking_type(&cfg_with_effort("gpt-4o", "off")),
        ThinkingType::Disabled
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("gpt-4o", "high")),
        ThinkingType::Disabled
    );
}

#[test]
fn test_thinking_type_off_means_disabled() {
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
        ThinkingType::Disabled
    );
    assert_eq!(
        thinking_type(&cfg_with_effort("claude-sonnet-4-5-20250929", "off")),
        ThinkingType::Disabled
    );
}

