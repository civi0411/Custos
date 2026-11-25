use crate::cloud::openrouter::gemini_compat::{
    apply_gemini_compatibility, escape_gemini_schema_ref_keys_in_tool_responses,
    gemini_schema_ref_note, is_gemini_model,
};
use serde_json::json;

#[test]
fn gemini_tool_result_schema_ref_keys_are_escaped_reversibly() {
    let mut payload = json!({
        "messages": [
            {
                "role": "assistant",
                "content": "Keep {'$ref': '#/components/schemas/AssistantText'} unchanged"
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\": \"#/components/schemas/Usage\", \"nested\": {\"$ref\" : \"#/components/schemas/Base64Image\"}, \"items\": [{\"$ref\": \"#/components/schemas/Item\"}], \"description\": \"use $ref here\", \"literal\": \"$ref\", \"identifier\": \"$reference\"}"
            }
        ]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        5
    );
    assert_eq!(
        payload["messages"][0]["content"],
        "Keep {'$ref': '#/components/schemas/AssistantText'} unchanged"
    );
    assert_eq!(
        payload["messages"][1]["content"],
        format!(
            "{}{{\"dollar_ref\": \"#/components/schemas/Usage\", \"nested\": {{\"dollar_ref\" : \"#/components/schemas/Base64Image\"}}, \"items\": [{{\"dollar_ref\": \"#/components/schemas/Item\"}}], \"description\": \"use dollar_ref here\", \"literal\": \"dollar_ref\", \"identifier\": \"$reference\"}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_rewrites_issue_11260_reproduction() {
    let reproduction = "{'properties': {'image': {'$ref': '#/components/schemas/Example'}}}";
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": reproduction
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{'properties': {{'image': {{'dollar_ref': '#/components/schemas/Example'}}}}}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_preserves_longer_identifiers() {
    let untouched = "$refs and $refresh_token and $reference stay intact";
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": untouched
        }]
    });
    let original = payload.clone();

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(payload, original);
}

#[test]
fn gemini_schema_ref_escape_preserves_literal_escaped_unicode_text() {
    let original = r#"{"literal":"\\u0024ref"}"#;
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": original
        }]
    });
    let untouched = payload.clone();

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(payload, untouched);
}

#[test]
fn gemini_schema_ref_escape_rewrites_unicode_token_after_even_backslashes() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"a":"x\\\\","\u0024ref":"A"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert!(payload["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains(r#""dollar_ref":"A""#));
}

#[test]
fn gemini_schema_ref_escape_matches_partially_escaped_keys() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"$\u0072ef":"A"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert!(payload["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains(r#""dollar_ref":"A""#));
}

#[test]
fn gemini_schema_ref_escape_matches_fully_escaped_keys() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"\u0024\u0072\u0065\u0066":"A"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert!(payload["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains(r#""dollar_ref":"A""#));
}

#[test]
fn gemini_schema_ref_escape_preserves_escaped_longer_identifiers() {
    let original = r#"{"$\u0072efresh_token":"A"}"#;
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": original
        }]
    });
    let untouched = payload.clone();

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        0
    );
    assert_eq!(payload, untouched);
}

#[test]
fn gemini_compatibility_only_applies_to_gemini_models() {
    let tool_result = r##"{"$ref":"#/components/schemas/Usage"}"##;
    let mut gemma_payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": tool_result
        }]
    });
    let original_gemma_payload = gemma_payload.clone();

    apply_gemini_compatibility("google/gemma-3-27b-it", &mut gemma_payload, &[]);

    assert_eq!(gemma_payload, original_gemma_payload);

    let mut anthropic_payload = original_gemma_payload.clone();
    apply_gemini_compatibility("anthropic/claude-sonnet-4.5", &mut anthropic_payload, &[]);

    assert_eq!(anthropic_payload, original_gemma_payload);

    let mut gemini_payload = original_gemma_payload;
    apply_gemini_compatibility("google/gemini-2.5-flash", &mut gemini_payload, &[]);

    assert_eq!(
        gemini_payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref\":\"#/components/schemas/Usage\"}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
    assert!(is_gemini_model("google/gemini-2.0-flash-exp:free"));
    assert!(!is_gemini_model("google/gemma-3-27b-it"));
}

#[test]
fn gemini_schema_ref_escape_rewrites_value_position_tokens() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"description\":\"use $ref here\",\"literal\":\"$ref\",\"identifier\":\"$reference\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        2
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"description\":\"use dollar_ref here\",\"literal\":\"dollar_ref\",\"identifier\":\"$reference\"}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_rewrites_non_json_prose() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "log entry, \"$ref\": not a JSON key"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}log entry, \"dollar_ref\": not a JSON key",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_preserves_duplicate_ref_members() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"$ref\":\"A\",\"$ref\":\"B\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        2
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref\":\"A\",\"dollar_ref\":\"B\"}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_preserves_unrelated_duplicate_members() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"duplicate\":\"first\",\"$ref\":\"A\",\"duplicate\":\"second\"}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"duplicate\":\"first\",\"dollar_ref\":\"A\",\"duplicate\":\"second\"}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_rewrites_json_embedded_in_prose() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "before output { \"$ref\" : \"A\" } after output"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}before output {{ \"dollar_ref\" : \"A\" }} after output",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_rewrites_multiple_occurrences_in_one_result() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "{\"$ref\":\"A\",\"nested\":{\"$ref\":\"B\"}}"
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        2
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            "{}{{\"dollar_ref\":\"A\",\"nested\":{{\"dollar_ref\":\"B\"}}}}",
            gemini_schema_ref_note("dollar_ref")
        )
    );
}

#[test]
fn gemini_schema_ref_escape_matches_decoded_keys_and_collisions() {
    let mut payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_1",
            "content": r#"{"\u0024ref":"A","\u0064ollar_ref":"B"}"#
        }]
    });

    assert_eq!(
        escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
        1
    );
    assert_eq!(
        payload["messages"][0]["content"],
        format!(
            r#"{}{{"dollar_ref_2":"A","\u0064ollar_ref":"B"}}"#,
            gemini_schema_ref_note("dollar_ref_2")
        )
    );
}
