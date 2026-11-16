use crate::cloud::openrouter::gemini_compat::{
    escape_gemini_schema_ref_keys_in_tool_responses, gemini_schema_ref_note,
};
use serde_json::{json, Value};

#[test]
fn gemini_schema_ref_escape_honors_escaped_quotes_and_backslashes() {
    let original_content =
        r#"{"text":"escaped quote \" then \\ and \"$ref\": still text","$ref":"A"}"#;
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": original_content
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        2
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            r#"{}{{"text":"escaped quote \" then \\ and \"dollar_ref\": still text","dollar_ref":"A"}}"#,
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_preserves_absent_content_bytes() {
    let original_content = "prefix { \"ordinary\" : [ 1, 2 ] } suffix";
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": original_content
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(
        payload["messages"][0]["content"]
            .as_str()
            .unwrap()
            .as_bytes(),
        original_content.as_bytes()
    );
}

#[test]
fn gemini_schema_ref_escape_tolerates_malformed_json() {
    let mut payload = json!({
        "messages": [
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\": \"A\""
            },
            {
                "role": "tool",
                "tool_call_id": "call_2",
                "content": "}"
            }
        ]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref\": \"A\"",
            gemini_schema_ref_note("dollar_ref")
        )
    );
    assert_eq!(payload["messages"][1]["content"], "}");
}

#[test]
fn gemini_schema_ref_escape_avoids_existing_safe_key() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref_2\":\"A\",\"dollar_ref\":\"B\"}}",
            gemini_schema_ref_note("dollar_ref_2")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_advances_past_multiple_key_collisions() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\",\"dollar_ref_2\":\"C\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref_3\":\"A\",\"dollar_ref\":\"B\",\"dollar_ref_2\":\"C\"}}",
            gemini_schema_ref_note("dollar_ref_3")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_advances_past_deep_collision_ladder() {
    let mut payload = json!({
        "messages": [
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\",\"nested\":{\"dollar_ref_2\":\"C\",\"items\":[{\"dollar_ref_3\":\"D\"},{\"dollar_ref_4\":\"E\"}]}}"
            },
            {
                "role": "tool",
                "tool_call_id": "call_2",
                "content": "{\"dollar_ref_5\":\"F\"}"
            }
        ]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref_6\":\"A\",\"dollar_ref\":\"B\",\"nested\":{{\"dollar_ref_2\":\"C\",\"items\":[{{\"dollar_ref_3\":\"D\"}},{{\"dollar_ref_4\":\"E\"}}]}}}}",
            gemini_schema_ref_note("dollar_ref_6")
        )
    );
    assert_eq!(
        payload["messages"][1]["content"],
        "{\"dollar_ref_5\":\"F\"}"
    );
}

#[test]
fn gemini_schema_ref_escape_ignores_non_tool_content_and_safe_tool_results() {
    let mut payload = json!({
        "messages": [
            { "role": "user", "content": "{\"$ref\":\"#/components/schemas/UserText\"}" },
            { "role": "assistant", "content": "{\"$ref\":\"#/components/schemas/AssistantText\"}" },
            { "role": "tool", "tool_call_id": "call_1", "content": "ordinary output" },
            { "role": "tool", "tool_call_id": "call_2", "content": [{ "type": "text", "text": "{'$ref': '#/components/schemas/Structured'}" }] }
        ]
    });
    let original = payload.clone();

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(payload, original);
}

#[test]
fn gemini_schema_ref_escape_ignores_braces_inside_string_literals() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"a":"}}}}","$ref":"B"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        Value::String(
            gemini_schema_ref_note("dollar_ref") + r#"{"a":"}}}}","dollar_ref":"B"}"#
        )
    );
}

#[test]
fn gemini_schema_ref_escape_ignores_brackets_inside_string_literals() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"a":"[[[","$ref":"B"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        Value::String(gemini_schema_ref_note("dollar_ref") + r#"{"a":"[[[","dollar_ref":"B"}"#)
    );
}

#[test]
fn gemini_schema_ref_escape_avoids_safe_key_used_in_another_tool_result() {
    let untouched = r#"{"dollar_ref":"x"}"#;
    let mut payload = json!({
        "messages": [
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": r#"{"$ref":"A"}"#
            },
            {
                "role": "tool",
                "tool_call_id": "call_2",
                "content": untouched
            }
        ]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        Value::String(gemini_schema_ref_note("dollar_ref_2") + r#"{"dollar_ref_2":"A"}"#)
    );
    assert_eq!(
        payload["messages"][1]["content"],
        Value::String(untouched.to_string())
    );
}

#[test]
fn gemini_schema_ref_escape_rewrites_nested_encoded_json() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"payload":"{\"$ref\":\"A\"}"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        Value::String(
            gemini_schema_ref_note("dollar_ref") + r#"{"payload":"{\"dollar_ref\":\"A\"}"}"#
        )
    );
}

#[test]
fn gemini_schema_ref_escape_preserves_multibyte_content() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"\u{540d}\u{524d}\":\"\u{1f389} ok\",\"$ref\":\"A\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        Value::String(
            gemini_schema_ref_note("dollar_ref")
                + "{\"\u{540d}\u{524d}\":\"\u{1f389} ok\",\"dollar_ref\":\"A\"}"
        )
    );
}

#[test]
fn gemini_schema_ref_escape_is_idempotent() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"$ref":"A","b":[{"$ref":"B"}]}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        2
    );
    let after_first_pass = payload.clone();
    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(payload, after_first_pass);
}
