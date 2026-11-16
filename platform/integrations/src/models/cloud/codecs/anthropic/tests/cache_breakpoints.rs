use super::super::*;
use super::*;
use crate::conversation::message::Message;
use rmcp::model::{CallToolRequestParams, CallToolResult, Tool};
use rmcp::object;
use serde_json::{json, Value};

fn sample_tools() -> Vec<Tool> {
    vec![
        Tool::new(
            "read_file",
            "Read a file from disk",
            object!({
                "type": "object",
                "properties": { "path": { "type": "string" } }
            }),
        ),
        Tool::new(
            "write_file",
            "Write a file to disk",
            object!({
                "type": "object",
                "properties": { "path": { "type": "string" }, "content": { "type": "string" } }
            }),
        ),
    ]
}

fn breakpoints(messages: &[Value]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (mi, message) in messages.iter().enumerate() {
        for (bi, block) in message["content"].as_array().unwrap().iter().enumerate() {
            if block.get(CACHE_CONTROL_FIELD).is_some() {
                found.push((mi, bi));
            }
        }
    }
    found
}

#[test]
fn breakpoints_cover_tools_system_and_last_two_user_messages() {
    let messages = vec![
        Message::user().with_text("What does the main entrypoint do?"),
        Message::assistant().with_tool_request(
            "tool_1",
            Ok(CallToolRequestParams::new("read_file")
                .with_arguments(object!({"path": "src/main.rs"}))),
        ),
        Message::user().with_tool_response(
            "tool_1",
            Ok(CallToolResult::success(vec![
                rmcp::model::ContentBlock::text("fn main() { run(); }"),
            ])),
        ),
        Message::assistant().with_text("It calls `run()`."),
        Message::user().with_text("Now add error handling to it."),
    ];
    let req = create_request_with_default_options(
        &cfg("claude-sonnet-4-5"),
        "You are a careful coding assistant.",
        &messages,
        &sample_tools(),
    )
    .unwrap();

    let tools = req["tools"].as_array().unwrap();
    assert!(tools[0].get(CACHE_CONTROL_FIELD).is_none());
    assert!(tools[1].get(CACHE_CONTROL_FIELD).is_some());
    assert!(req["system"][0].get(CACHE_CONTROL_FIELD).is_some());

    let request_messages = req["messages"].as_array().unwrap();
    let marked = breakpoints(request_messages);
    assert_eq!(
        marked,
        vec![(2, 0), (4, 0)],
        "message breakpoints should sit on the last block of the last two user messages"
    );
}

#[test]
fn disable_prompt_cache_removes_every_breakpoint() {
    let config = cfg("claude-sonnet-4-5").with_merged_request_params(
        std::collections::HashMap::from([(
            "disable_prompt_cache".to_string(),
            json!(true),
        )]),
    );
    let req = create_request_with_default_options(
        &config,
        "You are a summarizer.",
        &[Message::user().with_text("Summarize the conversation above.")],
        &sample_tools(),
    )
    .unwrap();

    assert!(!req.to_string().contains(CACHE_CONTROL_FIELD));
}

fn cache_control_values(req: &Value) -> Vec<Value> {
    let mut found = Vec::new();
    let tools = req["tools"].as_array().unwrap();
    let system = req["system"].as_array().unwrap();
    let messages = req["messages"].as_array().unwrap();
    for block in tools
        .iter()
        .chain(system.iter())
        .chain(messages.iter().flat_map(|m| {
            m["content"]
                .as_array()
                .map(|c| c.iter())
                .unwrap_or_default()
        }))
    {
        if let Some(cc) = block.get(CACHE_CONTROL_FIELD) {
            found.push(cc.clone());
        }
    }
    found
}

#[test]
fn default_breakpoints_omit_ttl() {
    let req = create_request_with_default_options(
        &cfg("claude-sonnet-4-5"),
        "You are a careful coding assistant.",
        &[Message::user().with_text("Hello")],
        &sample_tools(),
    )
    .unwrap();

    let values = cache_control_values(&req);
    assert_eq!(values.len(), 3);
    for cc in values {
        assert_eq!(cc, json!({ "type": "ephemeral" }));
    }
}

#[test]
fn one_hour_ttl_stamps_every_breakpoint() {
    let config = cfg("claude-sonnet-4-5").with_cache_ttl("1h");
    let req = create_request_with_default_options(
        &config,
        "You are a careful coding assistant.",
        &[
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi."),
            Message::user().with_text("Continue"),
        ],
        &sample_tools(),
    )
    .unwrap();

    let values = cache_control_values(&req);
    assert_eq!(values.len(), 4);
    for cc in values {
        assert_eq!(cc, json!({ "type": "ephemeral", "ttl": "1h" }));
    }
}

#[test]
fn explicit_five_minute_ttl_matches_default_wire_format() {
    let config = cfg("claude-sonnet-4-5").with_cache_ttl("5m");
    let req = create_request_with_default_options(
        &config,
        "You are a careful coding assistant.",
        &[Message::user().with_text("Hello")],
        &sample_tools(),
    )
    .unwrap();

    for cc in cache_control_values(&req) {
        assert_eq!(cc, json!({ "type": "ephemeral" }));
    }
}

#[test]
fn unrecognized_ttl_value_falls_back_to_default() {
    let config = cfg("claude-sonnet-4-5").with_cache_ttl("2h");
    let req = create_request_with_default_options(
        &config,
        "You are a careful coding assistant.",
        &[Message::user().with_text("Hello")],
        &sample_tools(),
    )
    .unwrap();

    for cc in cache_control_values(&req) {
        assert_eq!(cc, json!({ "type": "ephemeral" }));
    }
}

#[test]
fn disable_prompt_cache_wins_over_ttl() {
    let config = cfg("claude-sonnet-4-5")
        .with_cache_ttl("1h")
        .with_prompt_cache_disabled();
    let req = create_request_with_default_options(
        &config,
        "You are a summarizer.",
        &[Message::user().with_text("Summarize.")],
        &sample_tools(),
    )
    .unwrap();

    assert!(!req.to_string().contains(CACHE_CONTROL_FIELD));
}

#[test]
fn breakpoints_land_on_the_last_content_block() {
    let messages = vec![Message::user()
        .with_text("Here is the context.")
        .with_text("And the question.")];
    let req = create_request_with_default_options(
        &cfg("claude-sonnet-4-5"),
        "You are a careful coding assistant.",
        &messages,
        &sample_tools(),
    )
    .unwrap();

    let request_messages = req["messages"].as_array().unwrap();
    assert_eq!(breakpoints(request_messages), vec![(0, 1)]);
}
