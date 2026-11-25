use super::response::{get_usage, process_response_part_impl};
use crate::conversation::message::Message;
use crate::conversation::token_usage::ProviderUsage;
use crate::errors::ProviderError;
use rmcp::model::Role;
use serde_json::Value;
use uuid::Uuid;

pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl futures::Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: futures::Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    use async_stream::try_stream;
    use futures::StreamExt;

    try_stream! {
        let mut final_usage: Option<ProviderUsage> = None;
        let mut last_signature: Option<String> = None;
        let stream_id = Uuid::new_v4().to_string();
        let mut incomplete_data: Option<String> = None;
        let mut last_finish_reason: Option<String> = None;
        let mut last_response_id: Option<String> = None;

        while let Some(line_result) = stream.next().await {
            let line = line_result?;

            if line.trim().is_empty() {
                continue;
            }

            let data_part = if line.starts_with("data: ") {
                line.strip_prefix("data: ").unwrap()
            } else if line.starts_with("event:") || line.starts_with("id:") || line.starts_with("retry:") {
                continue;
            } else if incomplete_data.is_some() {
                &line
            } else {
                continue;
            };

            if data_part.trim() == "[DONE]" {
                break;
            }

            let chunk: Value = if let Some(ref mut incomplete) = incomplete_data {
                incomplete.push_str(data_part);
                match serde_json::from_str(incomplete) {
                    Ok(v) => {
                        incomplete_data = None;
                        v
                    }
                    Err(e) => {
                        if e.is_eof() {
                            continue;
                        }
                        tracing::warn!("Failed to parse streaming chunk: {}", e);
                        incomplete_data = None;
                        continue;
                    }
                }
            } else {
                match serde_json::from_str(data_part) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.is_eof() {
                            incomplete_data = Some(data_part.to_string());
                            continue;
                        }
                        tracing::warn!("Failed to parse streaming chunk: {}", e);
                        continue;
                    }
                }
            };

            if let Some(error) = chunk.get("error") {
                let message = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                let status = error
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("UNKNOWN");
                Err::<(), ProviderError>(ProviderError::RequestFailed(format!(
                    "Google API error ({status}): {message}"
                )))?;
            }

            if let Ok(usage) = get_usage(&chunk) {
                if usage.input_tokens.is_some() || usage.output_tokens.is_some() {
                    let model = chunk.get("modelVersion")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    final_usage = Some(ProviderUsage::new(model, usage));
                }
            }

            if let Some(response_id) = chunk.get("responseId").and_then(|v| v.as_str()) {
                last_response_id = Some(response_id.to_string());
            }

            let candidate = chunk
                .get("candidates")
                .and_then(|v| v.as_array())
                .and_then(|c| c.first());
            if let Some(reason) = candidate
                .and_then(|c| c.get("finishReason"))
                .and_then(|v| v.as_str())
            {
                last_finish_reason = Some(reason.to_string());
            }

            let parts = candidate
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array());

            if let Some(parts) = parts {
                for part in parts {
                    if let Some(content) = process_response_part_impl(part, &mut last_signature) {
                        let message = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![content],
                        ).with_id(stream_id.clone());
                        yield (Some(message), None);
                    }
                }
            }
        }

        if let Some(mut usage) = final_usage {
            if let Some(reason) = last_finish_reason {
                usage.finish_reasons = Some(vec![reason]);
            }
            usage.response_id = last_response_id;
            yield (None, Some(usage));
        }
    }
}
