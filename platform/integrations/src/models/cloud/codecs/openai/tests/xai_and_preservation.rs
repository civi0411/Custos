use super::super::*;
use crate::base::ThinkingPreservationFormat;
use crate::conversation::message::Message;
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock};
use rmcp::object;
use serde_json::{json, Value};

#[test]
fn test_is_openai_responses_model_matches_openai_reasoning_families() {
    for model in [
        "o3",
        "o3-mini",
        "o4-mini",
        "gpt-5",
        "gpt-5-pro",
        "gpt-5.4",
        "gpt-5.4-mini",
        "gpt-5-4",
        "gpt-5-2-pro",
        "databricks-gpt-5.4",
        "goose-gpt-5.4-high",
        "gpt-6-astra",
        "data_workflow_tools.goose.goose-gpt-6-astra",
        "headless-goose-o3-mini",
    ] {
        assert!(is_openai_responses_model(model), "{model} should match");
    }
}

#[test]
fn test_is_openai_responses_model_rejects_other_families() {
    for model in [
        "gpt-4o",
        "claude-sonnet-4",
        "databricks-claude-sonnet-4",
        "llama-3-70b",
    ] {
        assert!(
            !is_openai_responses_model(model),
            "{model} should not match"
        );
    }
}

#[test]
fn test_xai_reasoning_model_capabilities_are_model_specific() {
    for model in ["grok-4.5", "grok-4.3", "grok-3-mini"] {
        assert!(supports_xai_reasoning_effort(model), "{model}");
        assert!(is_xai_reasoning_model(model), "{model}");
    }

    for model in [
        "grok-4.20",
        "grok-4.20-0309-reasoning",
        "grok-4.20-multi-agent",
        "grok-4-0709",
        "grok-4-fast-reasoning",
    ] {
        assert!(!supports_xai_reasoning_effort(model), "{model}");
        assert!(is_xai_reasoning_model(model), "{model}");
    }

    for model in [
        "grok-4.20-0309-non-reasoning",
        "grok-4-fast-non-reasoning",
        "grok-3",
        "grok-build-0.1",
    ] {
        assert!(!supports_xai_reasoning_effort(model), "{model}");
        assert!(!is_xai_reasoning_model(model), "{model}");
    }
}

#[test]
fn test_xai_reasoning_effort_uses_each_models_supported_levels() {
    assert_eq!(
        xai_reasoning_effort_for_thinking("grok-4.5", ThinkingEffort::Off),
        Some("low".to_string())
    );
    assert_eq!(
        xai_reasoning_effort_for_thinking("grok-4.5", ThinkingEffort::Medium),
        Some("medium".to_string())
    );
    assert_eq!(
        xai_reasoning_effort_for_thinking("grok-4.3", ThinkingEffort::Off),
        Some("none".to_string())
    );
    assert_eq!(
        xai_reasoning_effort_for_thinking("grok-3-mini", ThinkingEffort::Medium),
        Some("high".to_string())
    );
    assert_eq!(
        xai_reasoning_effort_for_thinking("grok-4.5", ThinkingEffort::Max),
        Some("high".to_string())
    );
}

#[test]
fn test_create_request_applies_grok_4_5_reasoning_controls() {
    let model_config = ModelConfig::new("grok-4.5")
        .with_thinking_effort(ThinkingEffort::Medium)
        .with_temperature(Some(0.7));

    let payload = create_request(
        &model_config,
        "system prompt",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )
    .unwrap();

    assert_eq!(payload.get("reasoning_effort"), Some(&json!("medium")));
    assert!(payload.get("temperature").is_none());
}

#[test]
fn test_create_request_maps_xai_off_to_supported_effort() {
    for (model, expected) in [("grok-4.5", "low"), ("grok-4.3", "none")] {
        let model_config = ModelConfig::new(model).with_thinking_effort(ThinkingEffort::Off);
        let payload = create_request(
            &model_config,
            "system prompt",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )
        .unwrap();

        assert_eq!(
            payload.get("reasoning_effort"),
            Some(&json!(expected)),
            "{model}"
        );
    }
}

#[test]
fn test_create_request_keeps_grok_4_20_fixed_reasoning_parameters() {
    let model_config = ModelConfig::new("grok-4.20-0309-reasoning")
        .with_thinking_effort(ThinkingEffort::High)
        .with_temperature(Some(0.7));

    let payload = create_request(
        &model_config,
        "system prompt",
        &[],
        &[],
        &ImageFormat::OpenAi,
        false,
    )
    .unwrap();

    assert!(payload.get("reasoning_effort").is_none());
    assert_eq!(payload.get("temperature"), Some(&json!(0.7_f32)));
}

#[test]
fn test_extract_reasoning_effort_for_responses_models() {
    for (model, expected_name, expected_effort) in [
        ("o3-none", "o3", Some("none")),
        ("o3-xhigh", "o3", Some("xhigh")),
        ("gpt-5-low", "gpt-5", Some("low")),
        ("gpt-5.4", "gpt-5.4", None),
        (
            "databricks-gpt-5.4-high",
            "databricks-gpt-5.4",
            Some("high"),
        ),
        ("databricks-o3-low", "databricks-o3", Some("low")),
        ("goose-gpt-5-high", "goose-gpt-5", Some("high")),
        ("gpt-4o", "gpt-4o", None),
    ] {
        let (name, effort) = extract_reasoning_effort(model);
        assert_eq!(name, expected_name, "unexpected base model for {model}");
        assert_eq!(
            effort.as_deref(),
            expected_effort,
            "unexpected effort for {model}"
        );
    }
}

#[test]
fn test_sanitize_function_name() {
    assert_eq!(sanitize_function_name("hello-world"), "hello-world");
    assert_eq!(sanitize_function_name("hello world"), "hello_world");
    assert_eq!(sanitize_function_name("hello@world"), "hello_world");
    assert_eq!(
        sanitize_function_name(&"a".repeat(MAX_FUNCTION_NAME_LENGTH)),
        "a".repeat(MAX_FUNCTION_NAME_LENGTH)
    );
    assert_eq!(
        sanitize_function_name(&"a".repeat(MAX_FUNCTION_NAME_LENGTH + 32)),
        "a".repeat(MAX_FUNCTION_NAME_LENGTH)
    );
}

#[test]
fn test_is_valid_function_name() {
    assert!(is_valid_function_name("hello-world"));
    assert!(is_valid_function_name("hello_world"));
    assert!(is_valid_function_name(
        &"a".repeat(MAX_FUNCTION_NAME_LENGTH + 1)
    ));
    assert!(!is_valid_function_name("hello world"));
    assert!(!is_valid_function_name("hello@world"));
}

#[test]
fn formatter_post_parse_error_history_is_wellformed() {
    use rmcp::model::{ErrorCode, ErrorData};
    let err = ErrorData::new(
        ErrorCode::INVALID_PARAMS,
        "Tool arguments for id call_bad must be a JSON object".to_string(),
        None,
    );
    // Shape the agent loop builds today for a failed parse:
    let request_msg = Message::assistant().with_tool_request("call_bad", Err(err.clone()));
    let mut final_resp = Message::user();
    final_resp.add_tool_response_with_metadata("call_bad", Err(err), None);
    let messages = vec![
        Message::user().with_text("do the thing"),
        request_msg,
        final_resp,
    ];

    let spec = format_messages(&messages, &ImageFormat::OpenAi);

    let mut open = std::collections::HashSet::new();
    for m in &spec {
        match m.get("role").and_then(|v| v.as_str()) {
            Some("assistant") => {
                for tc in m
                    .get("tool_calls")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                {
                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                        open.insert(id.to_string());
                    }
                }
            }
            Some("tool") => {
                let id = m.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                assert!(open.contains(id), "orphan role:tool message for id {id:?}");
            }
            _ => {}
        }
    }
}

fn format_with_preservation(
    messages: &[Message],
    format: ThinkingPreservationFormat,
) -> Vec<Value> {
    format_messages_with_options(
        messages,
        &ImageFormat::OpenAi,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
            supports_vision: false,
            thinking_preservation_format: Some(format),
        },
    )
}

#[test]
fn test_thinking_preservation_content_prepend() {
    let message = Message::assistant()
        .with_thinking("Thinking process", "")
        .with_text("Hello");

    let spec = format_with_preservation(
        std::slice::from_ref(&message),
        ThinkingPreservationFormat::ContentPrepend,
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["content"], json!("Thinking process\n\nHello"));
    assert!(spec[0].get("reasoning_content").is_none());
}

#[test]
fn test_thinking_preservation_content_xml() {
    let message = Message::assistant()
        .with_thinking("Thinking process", "")
        .with_text("Hello");

    let spec = format_with_preservation(
        std::slice::from_ref(&message),
        ThinkingPreservationFormat::ContentXml,
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(
        spec[0]["content"],
        json!("<think>\nThinking process\n</think>\n\nHello")
    );
    assert!(spec[0].get("reasoning_content").is_none());
}

#[test]
fn test_thinking_preservation_reasoning_content_is_unchanged() {
    let message = Message::assistant()
        .with_thinking("Thinking process", "")
        .with_text("Hello");

    let spec = format_with_preservation(
        std::slice::from_ref(&message),
        ThinkingPreservationFormat::ReasoningContent,
    );

    assert_eq!(spec.len(), 1);
    assert_eq!(spec[0]["content"], json!("Hello"));
    assert_eq!(spec[0]["reasoning_content"], json!("Thinking process"));
}

#[test]
fn test_thinking_preservation_runs_after_split_tool_call_merge() {
    // Split tool-call messages are reunited by matching reasoning_content, so
    // inlining must happen afterwards or the merge silently stops working.
    let messages = vec![
        Message::assistant().with_thinking("reasoning", ""),
        Message::assistant()
            .with_thinking("reasoning", "")
            .with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("tool_a").with_arguments(object!({}))),
            ),
        Message::user().with_tool_response(
            "tool1",
            Ok(rmcp::model::CallToolResult::success(vec![
                ContentBlock::text("result1"),
            ])),
        ),
        Message::assistant()
            .with_thinking("reasoning", "")
            .with_tool_request(
                "tool2",
                Ok(CallToolRequestParams::new("tool_b").with_arguments(object!({}))),
            ),
    ];

    let spec = format_with_preservation(&messages, ThinkingPreservationFormat::ContentXml);

    let assistant: Vec<_> = spec
        .iter()
        .filter(|m| m.get("role") == Some(&json!("assistant")))
        .collect();

    assert_eq!(assistant.len(), 1);
    assert_eq!(assistant[0]["tool_calls"].as_array().unwrap().len(), 2);
    assert!(assistant[0].get("reasoning_content").is_none());
    assert_eq!(
        assistant[0]["content"],
        json!("<think>\nreasoning\n</think>")
    );
}

mod cache_prefix_stability {
    use super::*;

    fn turn_context(time: &str, turn_budget: &str) -> String {
        format!(
            "<turn-context>\n\
             <current-time>{time}</current-time>\n\
             <working-directory>/Users/me/code/goose</working-directory>\n\
             <turn-budget>{turn_budget}</turn-budget>\n\
             </turn-context>"
        )
    }

    fn tool_loop_conversation(turn_context_block: &str) -> Vec<Message> {
        vec![
            Message::user().with_text("What does the main entrypoint do?"),
            Message::assistant().with_text("Let me read it."),
            Message::user()
                .with_text("Now add error handling to it.")
                .with_text(turn_context_block),
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("read_file")
                    .with_arguments(object!({"path": "src/main.rs"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    "fn main() { run(); }",
                )])),
            ),
        ]
    }

    #[test]
    fn turn_context_stays_on_its_source_message() {
        let block = turn_context("2026-08-03 12:00:00", "14/40 used");
        let spec = format_messages(&tool_loop_conversation(&block), &ImageFormat::OpenAi);

        let occurrences: Vec<usize> = spec
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                m["content"]
                    .as_str()
                    .is_some_and(|c| c.contains("<turn-context>"))
                    .then_some(i)
            })
            .collect();
        assert_eq!(
            occurrences,
            vec![2],
            "turn-context must appear exactly once, on its source message"
        );
    }
}
