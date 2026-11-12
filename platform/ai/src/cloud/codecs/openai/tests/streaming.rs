use super::super::*;
use super::helpers::*;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::token_usage::Usage;
use rmcp::model::{ErrorCode, ErrorData};
use serde_json::json;
use tokio::pin;
use tokio_stream::{self, StreamExt};

async fn run_streaming_test(response_lines: &str) -> anyhow::Result<StreamingUsageTestResult> {
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let messages = response_to_streaming_message(response_stream);
    pin!(messages);

    let mut result = StreamingUsageTestResult {
        usage_count: 0,
        usage: None,
        tool_calls: Vec::new(),
        has_text_content: false,
        text: String::new(),
        output_token_limit_message_ids: Vec::new(),
    };

    while let Some(Ok((message, usage))) = messages.next().await {
        if let Some(u) = usage {
            result.usage_count += 1;
            result.usage = Some(u);
        }
        if let Some(msg) = message {
            if msg.metadata.output_token_limit_reached {
                result.output_token_limit_message_ids.push(msg.id.clone());
            }
            for content in &msg.content {
                match content {
                    MessageContentBlock::ToolRequest(req) => {
                        if let Ok(tool_call) = &req.tool_call {
                            result.tool_calls.push(tool_call.name.to_string());
                        }
                    }
                    MessageContentBlock::Text(text) if !text.text.is_empty() => {
                        result.has_text_content = true;
                        result.text.push_str(&text.text);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(result)
}

fn assert_usage_yielded_once(
    result: &StreamingUsageTestResult,
    expected_input: i32,
    expected_output: i32,
    expected_total: i32,
) {
    assert_eq!(
        result.usage_count, 1,
        "Usage should be yielded exactly once, but was yielded {} times",
        result.usage_count
    );

    let usage = result.usage.as_ref().expect("Expected usage to be present");
    assert_eq!(usage.usage.input_tokens, Some(expected_input));
    assert_eq!(usage.usage.output_tokens, Some(expected_output));
    assert_eq!(usage.usage.total_tokens, Some(expected_total));
}

async fn collect_streamed_tool_requests(
    response_lines: &str,
) -> Vec<crate::conversation::message::ToolRequest> {
    let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
    let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
    let messages = response_to_streaming_message(response_stream);
    pin!(messages);

    let mut requests = Vec::new();
    while let Some(Ok((message, _usage))) = messages.next().await {
        if let Some(msg) = message {
            for content in &msg.content {
                if let MessageContentBlock::ToolRequest(req) = content {
                    requests.push(req.clone());
                }
            }
        }
    }
    requests
}

#[tokio::test]
async fn test_streaming_reassembles_interleaved_parallel_tool_calls() {
    let response_lines = concat!(
        r#"data: {"id":"chatcmpl-par","model":"test-model","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_a","type":"function","function":{"name":"search","arguments":""}},{"index":1,"id":"call_b","type":"function","function":{"name":"write","arguments":""}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"id":"chatcmpl-par","model":"test-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"path\":"}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"id":"chatcmpl-par","model":"test-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"query\":"}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"id":"chatcmpl-par","model":"test-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"\"/tmp/a.md\"}"}}]},"finish_reason":null}]}"#,
        "\n",
        r#"data: {"id":"chatcmpl-par","model":"test-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"rust\"}"}}]},"finish_reason":"tool_calls"}]}"#,
        "\n",
        "data: [DONE]",
    );

    let requests = collect_streamed_tool_requests(response_lines).await;

    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests
            .iter()
            .map(|r| r.provider_index())
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );

    let search = requests[0].tool_call.as_ref().expect("search args parsed");
    assert_eq!(search.name, "search");
    assert_eq!(search.arguments.as_ref().unwrap()["query"], json!("rust"));

    let write = requests[1].tool_call.as_ref().expect("write args parsed");
    assert_eq!(write.name, "write");
    assert_eq!(
        write.arguments.as_ref().unwrap()["path"],
        json!("/tmp/a.md")
    );
}

#[tokio::test]
async fn test_streaming_marks_length_on_empty_terminal_chunk() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"chatcmpl-limit","model":"test-model","choices":[{"index":0,"delta":{"role":"assistant","content":"Partial answer"},"finish_reason":null}]}
data: {"id":"chatcmpl-limit","model":"test-model","choices":[{"index":0,"delta":{},"finish_reason":"length"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert_eq!(result.text, "Partial answer");
    assert_eq!(
        result.output_token_limit_message_ids,
        vec![Some("chatcmpl-limit".to_string())]
    );
    assert_usage_yielded_once(&result, 10, 5, 15);

    Ok(())
}

#[tokio::test]
async fn test_streamed_multi_tool_response_to_messages() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"I'll run both"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" `ls` commands in a"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" single turn for you -"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" one on the current directory an"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"d one on the `working_dir`."},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"id":"toolu_bdrk_01RMTd7R9DzQjEEWgDwzcBsU","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"command\": \"l"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"s\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"id":"toolu_bdrk_016bgVTGZdpjP8ehjMWp9cWW","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"command\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":": \"ls wor"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"king_dir"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":""},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4982,"completion_tokens":122,"total_tokens":5104},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;
    assert_eq!(
        result.tool_calls.len(),
        2,
        "Expected 2 tool calls, got {}",
        result.tool_calls.len()
    );
    assert!(result
        .tool_calls
        .iter()
        .all(|name| name == "developer__shell"));

    assert_usage_yielded_once(&result, 4982, 122, 5104);

    Ok(())
}

#[tokio::test]
async fn test_streaming_empty_finish_reason_is_not_terminal() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"model":"m","choices":[{"delta":{"role":"assistant","content":"Checking."},"index":0,"finish_reason":""}],"object":"chat.completion.chunk","id":"1","created":1753288340}
data: {"model":"m","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":""}],"object":"chat.completion.chunk","id":"1","created":1753288340}
data: {"model":"m","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"function":{"arguments":"{\"command\""}}]},"index":0,"finish_reason":""}],"object":"chat.completion.chunk","id":"1","created":1753288340}
data: {"model":"m","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"function":{"arguments":": \"ls\"}"}}]},"index":0,"finish_reason":""}],"object":"chat.completion.chunk","id":"1","created":1753288340}
data: {"model":"m","choices":[{"delta":{"role":"assistant","content":""},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120},"object":"chat.completion.chunk","id":"1","created":1753288340}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert!(result.has_text_content, "Expected text content in response");
    assert_eq!(
        result.tool_calls,
        vec!["developer__shell"],
        "tool call must survive intermediate empty-string finish_reason"
    );
    assert_usage_yielded_once(&result, 100, 20, 120);

    Ok(())
}

#[tokio::test]
async fn test_streaming_metadata_without_usage() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"chatcmpl-no-usage","model":"test-model","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}
data: {"id":"chatcmpl-no-usage","model":"test-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert_eq!(result.usage_count, 1);
    let usage = result.usage.unwrap();
    assert_eq!(usage.model, "test-model");
    assert_eq!(usage.usage, Usage::default());
    assert_eq!(usage.finish_reasons, Some(vec!["stop".to_string()]));
    assert_eq!(usage.response_id.as_deref(), Some("chatcmpl-no-usage"));

    Ok(())
}

#[tokio::test]
async fn test_openrouter_streaming_usage_yielded_once() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"There","reasoning":"","reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" are","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" **47**","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" files.","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":"stop","native_finish_reason":"stop","logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null,"native_finish_reason":null,"logprobs":null}],"usage":{"prompt_tokens":7007,"completion_tokens":49,"total_tokens":7056}}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert!(result.has_text_content, "Expected text content in response");
    assert_usage_yielded_once(&result, 7007, 49, 7056);
    let usage = result.usage.as_ref().unwrap();
    assert_eq!(
        usage.finish_reasons.as_deref(),
        Some(&["stop".to_string()][..])
    );
    assert_eq!(
        usage.response_id.as_deref(),
        Some("gen-1768896871-9HgAQqS1Z72C6gApaidi")
    );

    Ok(())
}

#[tokio::test]
async fn test_openai_gpt5_streaming_usage_yielded_once() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_x4CIvBVfQhYMhyO0T1VEddua","type":"function","function":{"name":"developer__shell","arguments":""}}],"refusal":null},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\":\"ls ~/Desktop | wc -l\"}"}}]},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[],"usage":{"prompt_tokens":8320,"completion_tokens":172,"total_tokens":8492}}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
    assert_eq!(result.tool_calls[0], "developer__shell");
    assert_usage_yielded_once(&result, 8320, 172, 8492);
    assert_eq!(
        result.usage.as_ref().map(|usage| usage.model.as_str()),
        Some("gpt-5.2-1106-preview")
    );
    let usage = result.usage.as_ref().unwrap();
    assert_eq!(
        usage.finish_reasons.as_deref(),
        Some(&["tool_calls".to_string()][..])
    );
    assert_eq!(
        usage.response_id.as_deref(),
        Some("chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU")
    );

    Ok(())
}

#[tokio::test]
async fn test_tetrate_claude_streaming_usage_yielded_once() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"role":"assistant"}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"toolu_011Yj5pGczhs1597iLXp5XJK","type":"function","function":{"name":"developer__shell","arguments":""}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"{\"command\": \"find ~/Desktop -type f | wc -l\"}"}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk","usage":{"completion_tokens":79,"prompt_tokens":12376,"total_tokens":12455}}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
    assert_eq!(result.tool_calls[0], "developer__shell");
    assert_usage_yielded_once(&result, 12376, 79, 12455);

    Ok(())
}

#[tokio::test]
async fn test_azure_annotation_chunk_without_delta_does_not_fail() -> anyhow::Result<()> {
    let response_lines = r#"
data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":1234567890,"model":"gpt-5.4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}],"usage":null}
data: {"choices":[{"content_filter_offsets":{"check_offset":5,"start_offset":5,"end_offset":5},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"finish_reason":null,"index":0}],"created":0,"id":"","model":"","object":""}
data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":1234567891,"model":"gpt-5.4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}}
data: [DONE]
"#;

    let result = run_streaming_test(response_lines).await?;

    assert!(result.has_text_content, "Expected text content in response");
    assert_usage_yielded_once(&result, 10, 1, 11);

    Ok(())
}

#[tokio::test]
async fn test_streaming_response_with_nested_extra_content() -> anyhow::Result<()> {
    let response_lines = r#"data: {"model":"test-model","choices":[{"delta":{"role":"assistant","tool_calls":[{"extra_content":{"google":{"thought_signature":"nested_stream_sig"}},"id":"call_nested","function":{"name":"test_tool","arguments":"{}"},"type":"function","index":0}]},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110},"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: [DONE]"#;

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let messages = response_to_streaming_message(response_stream);
    pin!(messages);

    while let Some(Ok((message, _usage))) = messages.next().await {
        if let Some(msg) = message {
            if let MessageContentBlock::ToolRequest(request) = &msg.content[0] {
                assert!(request.tool_call.is_ok());
                assert!(request.metadata.is_some());
                let metadata = request.metadata.as_ref().unwrap();
                let extra_content = metadata.get("extra_content").unwrap();
                assert_eq!(
                    extra_content["google"]["thought_signature"],
                    "nested_stream_sig"
                );
                return Ok(());
            }
        }
    }

    panic!("Expected tool call message with nested extra_content metadata");
}

#[tokio::test]
async fn test_streaming_non_object_arguments_does_not_panic() -> anyhow::Result<()> {
    // Streamed tool call whose arguments are valid JSON but NOT an object.
    // Must yield an INVALID_PARAMS tool error, not panic via rmcp `object()`.
    let response_lines = r#"data: {"model":"test-model","choices":[{"delta":{"role":"assistant","tool_calls":[{"id":"call_bad","function":{"name":"test_tool","arguments":"[1, "},"type":"function","index":0}]},"index":0,"finish_reason":null}],"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: {"model":"test-model","choices":[{"delta":{"tool_calls":[{"function":{"arguments":"2, 3]"},"index":0}]},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110},"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: [DONE]"#;

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let messages = response_to_streaming_message(response_stream);
    pin!(messages);

    let mut found_tool_error = false;
    let mut usage_count = 0;
    while let Some(result) = messages.next().await {
        let (message, usage) = result?;
        if usage.is_some() {
            usage_count += 1;
        }
        if let Some(msg) = message {
            if let MessageContentBlock::ToolRequest(request) = &msg.content[0] {
                assert!(!msg.metadata.output_token_limit_reached);
                assert_eq!(msg.id.as_deref(), Some("test-id"));
                match &request.tool_call {
                    Err(ErrorData {
                        code: ErrorCode::INVALID_PARAMS,
                        message: m,
                        ..
                    }) => {
                        assert!(m.contains("must be a JSON object"));
                        assert!(
                            m.contains("test_tool"),
                            "error must name the original tool so the model can retry it: {m}"
                        );
                        found_tool_error = true;
                    }
                    _ => panic!("expected INVALID_PARAMS for non-object streamed args"),
                }
            }
        }
    }

    assert!(found_tool_error, "expected a tool request message");
    assert_eq!(usage_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_streaming_length_rejects_tool_calls_regardless_of_arguments() -> anyhow::Result<()>
{
    let response_lines = r#"data: {"model":"test-model","choices":[{"delta":{"role":"assistant","tool_calls":[{"id":"call_empty","function":{"name":"empty_tool","arguments":""},"type":"function","index":0},{"id":"call_valid","function":{"name":"valid_tool","arguments":"{\"value\":"},"type":"function","index":1}]},"index":0,"finish_reason":null}],"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: {"model":"test-model","choices":[{"delta":{"tool_calls":[{"function":{"arguments":""},"index":0},{"function":{"arguments":"true}"},"index":1}]},"index":0,"finish_reason":"length"}],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110},"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: [DONE]"#;

    let response_stream =
        tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
    let messages = response_to_streaming_message(response_stream);
    pin!(messages);

    let mut tool_error_ids = Vec::new();
    let mut usage_count = 0;
    while let Some(result) = messages.next().await {
        let (message, usage) = result?;
        if usage.is_some() {
            usage_count += 1;
        }
        if let Some(msg) = message {
            assert!(msg.metadata.output_token_limit_reached);
            assert_eq!(msg.id.as_deref(), Some("test-id"));
            for content in msg.content {
                if let MessageContentBlock::ToolRequest(request) = content {
                    let request_id = request.id.clone();
                    match request.tool_call {
                        Err(error) => {
                            assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
                            assert!(error.message.contains("output token limit"));
                            tool_error_ids.push(request_id);
                        }
                        Ok(_) => panic!("length-terminated tool call must not be executable"),
                    }
                }
            }
        }
    }

    assert_eq!(tool_error_ids, vec!["call_empty", "call_valid"]);
    assert_eq!(usage_count, 1);
    Ok(())
}

