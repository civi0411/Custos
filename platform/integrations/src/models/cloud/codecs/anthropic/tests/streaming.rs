use super::helpers::*;
use crate::conversation::token_usage::CostSource;
use serde_json::json;
#[tokio::test]
async fn test_streaming_marks_max_tokens() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_limit","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Partial answer"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":25}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.text, vec!["Partial answer"]);
    assert_eq!(
        parts.output_token_limit_message_ids,
        vec![Some("msg_limit".to_string())]
    );
}

#[tokio::test]
async fn test_streaming_thinking_and_text() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me analyze"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":" this problem."}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_abc"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"123"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Here is the answer."}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":1}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.thinking.len(), 1);
    assert_eq!(parts.thinking[0].0, "Let me analyze this problem.");
    assert_eq!(parts.thinking[0].1, "sig_abc123");
    assert_eq!(parts.text, vec!["Here is the answer."]);
}

#[tokio::test]
async fn test_streaming_thinking_from_start_block_without_signature() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"Initial reasoning "}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"continues."}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.thinking.len(), 1);
    assert_eq!(parts.thinking[0].0, "Initial reasoning continues.");
    assert_eq!(parts.thinking[0].1, "");
}

#[tokio::test]
async fn test_streaming_redacted_thinking() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_2","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":5,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque_base64_data"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Done."}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":1}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.redacted_thinking, vec!["opaque_base64_data"]);
    assert_eq!(parts.text, vec!["Done."]);
}

#[tokio::test]
async fn test_streaming_thinking_text_then_tool_call() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_3","role":"assistant","content":[],"model":"claude-sonnet-4-6","usage":{"input_tokens":8,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I should search for this."}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"tool_sig_xyz"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Let me search for that."}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":1}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tool_1","name":"search","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"rust\"}"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":2}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.thinking.len(), 1);
    assert_eq!(
        parts.thinking[0],
        (
            "I should search for this.".to_string(),
            "tool_sig_xyz".to_string()
        )
    );
    assert_eq!(parts.text, vec!["Let me search for that."]);
    assert_eq!(parts.tool_calls, vec!["search"]);
}

#[tokio::test]
async fn test_streaming_reassembles_interleaved_parallel_tool_calls() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_par","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":5,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_a","name":"search","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tool_b","name":"write","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"query\":"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"\"/tmp/a.md\"}"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"rust\"}"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":1}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":20}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let requests = collect_tool_requests(events).await;

    assert_eq!(requests.len(), 2);
    let write = requests
        .iter()
        .find(|r| r.id == "tool_b")
        .expect("write tool request");
    assert_eq!(write.provider_index(), Some(1));
    let write_call = write.tool_call.as_ref().expect("write args parsed");
    assert_eq!(write_call.name, "write");
    assert_eq!(
        write_call.arguments.as_ref().unwrap()["path"],
        json!("/tmp/a.md")
    );

    let search = requests
        .iter()
        .find(|r| r.id == "tool_a")
        .expect("search tool request");
    assert_eq!(search.provider_index(), Some(0));
    let search_call = search.tool_call.as_ref().expect("search args parsed");
    assert_eq!(search_call.name, "search");
    assert_eq!(
        search_call.arguments.as_ref().unwrap()["query"],
        json!("rust")
    );
}

#[tokio::test]
async fn test_streaming_preserves_cache_tokens_through_delta_merge() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":7,"cache_creation_input_tokens":10000,"cache_read_input_tokens":5000,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let usage = collect_stream_results(events)
        .await
        .into_iter()
        .filter_map(|r| r.ok().and_then(|(_, usage)| usage))
        .next_back()
        .expect("stream should yield usage");

    assert_eq!(usage.usage.input_tokens, Some(15007));
    assert_eq!(usage.usage.output_tokens, Some(25));
    assert_eq!(usage.usage.cache_read_input_tokens, Some(5000));
    assert_eq!(usage.usage.cache_write_input_tokens, Some(10000));
    assert_eq!(
        usage.finish_reasons.as_deref(),
        Some(&["end_turn".to_string()][..])
    );
    assert_eq!(usage.response_id.as_deref(), Some("msg_1"));
}

#[tokio::test]
async fn test_streaming_surfaces_additional_usage_data() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-sonnet-4-5","usage":{"input_tokens":7,"output_tokens":0,"service_tier":"fast"}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let additional = streamed_usage(events)
        .await
        .additional_data
        .expect("additional data should be reported");
    assert_eq!(additional["service_tier"], json!("fast"));
}

#[tokio::test]
async fn test_streaming_omits_additional_usage_data_when_absent() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-sonnet-4-5","usage":{"input_tokens":7,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    assert!(streamed_usage(events).await.additional_data.is_none());
}

#[tokio::test]
async fn test_streaming_preserves_provider_cost_from_delta() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"m1","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":100,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":50,"cost":0.0123}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let usage = collect_stream_results(events)
        .await
        .into_iter()
        .filter_map(|r| r.ok().and_then(|(_, usage)| usage))
        .next_back()
        .expect("stream should yield usage");

    assert_eq!(usage.cost, Some(0.0123));
    assert_eq!(usage.cost_source, Some(CostSource::ProviderReported));
    assert_eq!(usage.usage.input_tokens, Some(100));
    assert_eq!(usage.usage.output_tokens, Some(50));
}

#[tokio::test]
async fn test_streaming_delta_usage_is_cumulative_and_wins() {
    // Server tool use grows input during the turn: the final
    // message_delta usage is authoritative, not message_start.
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":2679,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":3}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":10682,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":510}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let usage = collect_stream_results(events)
        .await
        .into_iter()
        .filter_map(|r| r.ok().and_then(|(_, usage)| usage))
        .next_back()
        .expect("stream should yield usage");

    assert_eq!(usage.usage.input_tokens, Some(10982)); // 10682 + 100 + 200
    assert_eq!(usage.usage.output_tokens, Some(510));
    assert_eq!(usage.usage.cache_read_input_tokens, Some(200));
    assert_eq!(usage.usage.cache_write_input_tokens, Some(100));
}

#[tokio::test]
async fn test_streaming_refusal() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"explanation":"This request violates the usage policy.","category":"cyber"}},"usage":{"output_tokens":5}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let results = collect_stream_results(events).await;
    let usage = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .find_map(|(_, usage)| usage.clone())
        .expect("a refused request should still yield its usage");
    assert_eq!(usage.usage.input_tokens, Some(10));
    assert_eq!(usage.usage.output_tokens, Some(5));
    assert_eq!(usage.finish_reasons, Some(vec!["refusal".to_string()]));
    assert_eq!(usage.response_id.as_deref(), Some("msg_1"));

    let (details, category) = expect_refusal(results);
    assert_eq!(details, "This request violates the usage policy.");
    assert_eq!(category.as_deref(), Some("cyber"));
}

#[tokio::test]
async fn test_streaming_refusal_forwards_unrecognized_stop_details() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"code":42}},"usage":{"output_tokens":5}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let (details, category) = expect_refusal(collect_stream_results(events).await);
    assert!(details.contains("\"code\":42"), "details: {details}");
    assert_eq!(category, None);
}

#[tokio::test]
async fn test_streaming_forwards_unhandled_stop_details() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"model_context_window_exceeded","stop_details":{"reason":"context_window"}},"usage":{"output_tokens":5}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.text.len(), 1);
    assert!(
        parts.text[0].starts_with("The provider ended the response with:"),
        "text: {}",
        parts.text[0]
    );
    assert!(parts.text[0].contains("context_window"));
}

#[tokio::test]
async fn test_streaming_truncated_tool_args_in_content_block_stop() {
    // Block is closed by content_block_stop, but the concatenated deltas form
    // truncated JSON (each fragment is valid; together they're unterminated).
    let events = concat!(
        r##"data: {"type":"message_start","message":{"id":"msg_t","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"##,
        "\n",
        r##"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_t","name":"write","input":{}}}"##,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/some/path.md\","}}"#,
        "\n",
        r##"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"content\":\"# Very long markdown"}}"##,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":4096}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(
        parts.tool_errors.len(),
        1,
        "expected one tool error, got: {:?}",
        parts.tool_errors
    );
    let msg = &parts.tool_errors[0];
    assert!(
        msg.contains("truncated") || msg.contains("output token limit"),
        "expected actionable truncation message, got: {}",
        msg
    );
    assert!(
        msg.contains("max_tokens") || msg.contains("smaller steps"),
        "expected guidance to increase max_tokens or break up the task, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_streaming_truncated_tool_args_no_content_block_stop() {
    // The stream ends with the tool_use block still open (no content_block_stop),
    // which is what happens when the model is cut off mid-tool-call.
    let events = concat!(
        r##"data: {"type":"message_start","message":{"id":"msg_t2","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"##,
        "\n",
        r##"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_t2","name":"write","input":{}}}"##,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/report.md\","}}"#,
        "\n",
        r##"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"content\":\"# Big report that got cut off mid"}}"##,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":8192}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(
        parts.output_token_limit_message_ids,
        vec![Some("msg_t2".to_string())]
    );
    assert_eq!(
        parts.tool_errors.len(),
        1,
        "expected one tool error for the dropped/truncated tool call, got: {:?}",
        parts.tool_errors
    );
    let msg = &parts.tool_errors[0];
    assert!(
        msg.contains("truncated") || msg.contains("output token limit"),
        "expected actionable truncation message, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_streaming_unfinished_tool_calls_keep_provider_indices() {
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_t3","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_open_a","name":"search","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tool_open_b","name":"write","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"ru"}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/re"}}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":8192}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let requests = collect_tool_requests(events).await;

    assert_eq!(requests.len(), 2);
    let indexed: Vec<(String, Option<i32>)> = requests
        .iter()
        .map(|r| (r.id.clone(), r.provider_index()))
        .collect();
    assert_eq!(
        indexed,
        vec![
            ("tool_open_a".to_string(), Some(0)),
            ("tool_open_b".to_string(), Some(1)),
        ]
    );
    assert!(requests.iter().all(|r| r.tool_call.is_err()));
}

#[tokio::test]
async fn test_streaming_complete_tool_call_unaffected() {
    // Regression guard: a normal, complete tool call must still parse and
    // produce no error even though stop_reason handling is added.
    let events = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_ok","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"#,
        "\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_ok","name":"write","input":{}}}"#,
        "\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/ok.md\",\"content\":\"hello\"}"}}"#,
        "\n",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}"#,
        "\n",
        r#"data: {"type":"message_stop"}"#,
    );

    let parts = collect_stream(events).await;
    assert_eq!(parts.tool_calls, vec!["write"]);
    assert!(parts.tool_errors.is_empty());
}
