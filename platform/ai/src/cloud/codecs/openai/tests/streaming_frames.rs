use super::super::*;
use super::helpers::*;
use crate::conversation::message::MessageContentBlock;
use crate::errors::ProviderError;
use serde_json::Value;
use test_case::test_case;
use tokio_stream::{self, StreamExt};

#[test_case(
    "data: {\"error\":{\"message\":\"Internal server error\",\"type\":\"server_error\",\"code\":500}}\ndata: [DONE]",
    "Internal server error";
    "openai error format"
)]
#[test_case(
    "data: {\"object\":\"error\",\"message\":\"CUDA out of memory\",\"code\":500}\ndata: [DONE]",
    "CUDA out of memory";
    "vllm error format"
)]
#[test_case(
    "data: {\"error\":{\"message\":\"Rate limit exceeded\",\"type\":\"rate_limit_error\"}}",
    "Rate limit exceeded";
    "error as first chunk"
)]
#[tokio::test]
async fn test_mid_stream_server_error(response_lines: &str, expected_msg: &str) {
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
    let mut found_error = false;
    while let Some(result) = messages.next().await {
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains(expected_msg),
                "unexpected error text: {err_str}"
            );
            found_error = true;
            break;
        }
    }
    assert!(
        found_error,
        "expected an error but stream completed successfully"
    );
}

fn content_chunk(content: &str) -> String {
    format!(
        concat!(
            r#"data: {{"id":"x","object":"chat.completion.chunk","created":1,"model":"m","#,
            r#""choices":[{{"index":0,"delta":{{"content":"{}"}},"finish_reason":null}}]}}"#
        ),
        content
    )
}

#[tokio::test]
async fn test_metadata_only_first_frame_does_not_abort_stream() -> anyhow::Result<()> {
    // The reported bug: a metadata-only opening frame made the whole turn fail with
    // "Failed to parse streaming chunk: missing field `choices`".
    let response_lines = format!("{METADATA_FRAME}\n{}\ndata: [DONE]", content_chunk("hello"));
    assert_eq!(run_streaming_test(&response_lines).await?.text, "hello");
    Ok(())
}

#[tokio::test]
async fn test_metadata_only_frame_interleaved_mid_stream_is_skipped() -> anyhow::Result<()> {
    let response_lines = format!(
        "{}\n{METADATA_FRAME}\n{}\ndata: [DONE]",
        content_chunk("hel"),
        content_chunk("lo")
    );
    assert_eq!(run_streaming_test(&response_lines).await?.text, "hello");
    Ok(())
}

#[tokio::test]
async fn test_metadata_only_frame_mid_tool_call_keeps_arguments_intact() -> anyhow::Result<()> {
    // A metadata frame arriving between two tool_calls argument deltas must not end
    // argument accumulation. Merely defaulting `choices` to an empty vec would route this
    // frame into the inner loop's empty-choices branch (`done = true`) and silently
    // truncate the arguments to `{"city":"Pa` — a quiet corruption instead of a loud error.
    let response_lines = concat!(
        r#"data: {"id":"x","object":"chat.completion.chunk","model":"m","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"Pa"}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"hook_results":{"before_request_hooks":[{"verdict":true,"type":"guardrail","deny":false}]}}"#,
        "\n",
        r#"data: {"id":"x","object":"chat.completion.chunk","model":"m","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ris\"}"}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"id":"x","object":"chat.completion.chunk","model":"m","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        "\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let mut tool_calls = Vec::new();
    while let Some(result) = messages.next().await {
        let (message, _usage) = result?;
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(req) = content {
                    if let Ok(call) = &req.tool_call {
                        tool_calls.push(call.clone());
                    }
                }
            }
        }
    }

    assert_eq!(tool_calls.len(), 1, "expected exactly one tool call");
    assert_eq!(tool_calls[0].name, "get_weather");
    assert_eq!(
        tool_calls[0]
            .arguments
            .as_ref()
            .and_then(|a| a.get("city"))
            .and_then(Value::as_str),
        Some("Paris"),
        "arguments must survive the interleaved metadata frame intact"
    );
    Ok(())
}

#[tokio::test]
async fn test_empty_choices_array_is_not_treated_as_metadata() -> anyhow::Result<()> {
    // `"choices": []` is the standard usage-only chunk, NOT a metadata frame: it must
    // still deserialize and still surface its usage.
    let response_lines = concat!(
        r#"data: {"id":"x","object":"chat.completion.chunk","model":"m","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":"stop"}]}"#,
        "\n",
        r#"data: {"id":"x","object":"chat.completion.chunk","model":"m","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}}"#,
        "\n",
        "data: [DONE]"
    );
    let result = run_streaming_test(response_lines).await?;
    assert_eq!(result.usage_count, 1, "the usage-only chunk must be kept");
    let usage = result.usage.expect("usage should be reported");
    assert_eq!(usage.usage.output_tokens, Some(3));
    assert!(result.has_text_content);
    Ok(())
}

#[test]
fn test_error_frames_still_surface_as_server_error() {
    // Skipping choice-less frames must not swallow gateway error frames, which are also
    // choice-less. Every one of these is handled ahead of the metadata skip.
    for line in [
        r#"{"error":{"message":"upstream exploded"}}"#,
        r#"{"object":"error","message":"upstream exploded"}"#,
        // No `error` key and no `object`: the shape Azure APIM rate-limits with, on an
        // HTTP 200. Skipping this would report the failed turn as an empty success.
        r#"{"statusCode":429,"message":"upstream exploded"}"#,
        // Some gateways stringify the status.
        r#"{"status":"503","message":"upstream exploded"}"#,
        // FastAPI's HTTPException shape.
        r#"{"detail":"upstream exploded"}"#,
        // FastAPI's RequestValidationError shape: `detail` is a LIST, so a string-only
        // read would drop it and silently skip the frame.
        r#"{"detail":[{"loc":["body"],"msg":"upstream exploded","type":"value_error"}]}"#,
        // A non-string `message` must not be dropped either.
        r#"{"statusCode":500,"message":{"text":"upstream exploded"}}"#,
        // `type: "error"` is a third error marker some gateways use.
        r#"{"type":"error","message":"upstream exploded"}"#,
    ] {
        match parse_streaming_chunk(line) {
            Err(ProviderError::ServerError(msg)) => assert!(
                msg.contains("upstream exploded"),
                "message preserved for {line}, got {msg:?}"
            ),
            other => panic!("expected ServerError for {line}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn test_informational_choiceless_frame_is_still_skipped() -> anyhow::Result<()> {
    // Prose alone is not an error signal. A keepalive/progress frame carrying a message
    // but no status must NOT abort the turn — doing so would reintroduce exactly the bug
    // the metadata skip exists to fix.
    let response_lines = format!(
        "{}\n{}\ndata: [DONE]",
        r#"data: {"message":"processing"}"#,
        content_chunk("hello")
    );
    assert_eq!(run_streaming_test(&response_lines).await?.text, "hello");
    Ok(())
}

#[test]
fn test_status_only_error_frame_still_fails_loudly() {
    // No message text anywhere: the frame must still surface rather than vanish.
    match parse_streaming_chunk(r#"{"statusCode":503}"#) {
        Err(ProviderError::ServerError(msg)) => {
            assert!(msg.contains("503"), "status should reach the caller: {msg}")
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[test]
fn test_choiceless_frame_error_text_is_capped() {
    let long = "x".repeat(5_000);
    let line = format!(r#"{{"statusCode":500,"message":"{long}"}}"#);
    match parse_streaming_chunk(&line) {
        Err(ProviderError::ServerError(msg)) => assert!(
            msg.chars().count() <= MAX_STREAM_ERROR_LEN + 1,
            "error text should be truncated, got {} chars",
            msg.chars().count()
        ),
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[tokio::test]
async fn test_in_stream_error_frame_aborts_rather_than_ending_empty() {
    // End-to-end counterpart: the turn must fail, not complete with no content. A silent
    // skip here tells the user to resend — the worst possible advice into a 429.
    let response_lines = concat!(
        r#"data: {"statusCode":429,"message":"rate limited"}"#,
        "\n",
        "data: [DONE]"
    );
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

    let first = messages.next().await.expect("stream should yield an item");
    let err = first.expect_err("an in-stream error frame must not be skipped");
    assert!(
        err.to_string().contains("rate limited"),
        "the gateway's message must reach the caller: {err}"
    );
}

#[test]
fn test_parse_streaming_chunk_returns_none_for_metadata_frames() {
    // Unit-level counterpart to the streaming tests above.
    let metadata = parse_streaming_chunk(r#"{"hook_results":{"before_request_hooks":[]}}"#)
        .expect("metadata frame must not be an error");
    assert!(metadata.is_none(), "metadata frame should be skipped");

    let real = parse_streaming_chunk(r#"{"choices":[],"usage":{"completion_tokens":1}}"#)
        .expect("usage-only chunk must parse");
    assert!(
        real.is_some(),
        "`choices: []` is a real chunk, not metadata"
    );
}

