use super::super::*;
use crate::conversation::message::MessageContentBlock;
use crate::conversation::token_usage::ProviderUsage;
use crate::model::ModelConfig;
use tokio::pin;
use tokio_stream::{self, StreamExt};

pub(crate) fn test_model_config(model_name: &str) -> ModelConfig {
    ModelConfig::new(model_name)
}

pub(crate) const OPENAI_TOOL_USE_RESPONSE: &str = r#"{
    "choices": [{
        "role": "assistant",
        "message": {
            "tool_calls": [{
                "id": "1",
                "function": {
                    "name": "example_fn",
                    "arguments": "{\"param\": \"value\"}"
                }
            }]
        }
    }],
    "usage": {
        "input_tokens": 10,
        "output_tokens": 25,
        "total_tokens": 35
    }
}"#;

pub(crate) const METADATA_FRAME: &str = concat!(
    r#"data: {"hook_results":{"before_request_hooks":[{"verdict":true,"#,
    r#""id":"guardrail-1","type":"guardrail","deny":false}]}}"#
);

pub(crate) struct StreamingUsageTestResult {
    pub(crate) usage_count: usize,
    pub(crate) usage: Option<ProviderUsage>,
    pub(crate) tool_calls: Vec<String>,
    pub(crate) has_text_content: bool,
    pub(crate) text: String,
    pub(crate) output_token_limit_message_ids: Vec<Option<String>>,
}

pub(crate) async fn run_streaming_test(response_lines: &str) -> anyhow::Result<StreamingUsageTestResult> {
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
                        if let Ok(tc) = &req.tool_call {
                            result.tool_calls.push(tc.name.to_string());
                        }
                    }
                    MessageContentBlock::Text(text) => {
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
