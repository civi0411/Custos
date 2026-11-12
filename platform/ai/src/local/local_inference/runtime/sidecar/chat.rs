use custos_provider::errors::ProviderError;
use futures::StreamExt;

use super::types::get_sidecar_port;

/// Streams chat completions directly from the managed sidecar via its OpenAI-compatible endpoint.
#[allow(clippy::type_complexity)]
pub async fn stream_sidecar_chat(
    model_name: &str,
    system: &str,
    messages: &[custos_provider::conversation::message::Message],
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    tx: tokio::sync::mpsc::Sender<
        Result<
            (
                Option<custos_provider::conversation::message::Message>,
                Option<custos_provider::conversation::token_usage::ProviderUsage>,
            ),
            ProviderError,
        >,
    >,
) -> Result<(), ProviderError> {
    let port = get_sidecar_port();
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");

    let mut msgs: Vec<serde_json::Value> = Vec::new();
    if !system.trim().is_empty() {
        msgs.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    for m in messages {
        let role = match m.role {
            rmcp::model::Role::User => "user",
            rmcp::model::Role::Assistant => "assistant",
        };
        let text = crate::local::local_inference::extract_text_content(m);
        if !text.trim().is_empty() {
            msgs.push(serde_json::json!({
                "role": role,
                "content": text
            }));
        }
    }

    let mut body = serde_json::json!({
        "model": model_name,
        "messages": msgs,
        "stream": true,
    });
    if let Some(t) = temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(m) = max_tokens {
        body["max_tokens"] = serde_json::json!(m);
    }

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Sidecar HTTP request failed: {e}")))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ProviderError::ExecutionError(format!(
            "Sidecar returned error: {err_text}"
        )));
    }

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_res) = stream.next().await {
        let chunk = match chunk_res {
            Ok(c) => c,
            Err(e) => return Err(ProviderError::ExecutionError(e.to_string())),
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer.drain(..=pos);

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    break;
                }

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                        if let Some(choice) = choices.first() {
                            if let Some(delta) = choice.get("delta") {
                                let text = delta
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| {
                                        delta.get("reasoning_content").and_then(|v| v.as_str())
                                    })
                                    .unwrap_or("");
                                if !text.is_empty() {
                                    let msg =
                                        custos_provider::conversation::message::Message::assistant(
                                        )
                                        .with_text(text);
                                    let _ = tx.send(Ok((Some(msg), None))).await;
                                }
                            }
                        }
                    }
                    if let Some(usage_val) = val.get("usage") {
                        let prompt_tokens = usage_val
                            .get("prompt_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let completion_tokens = usage_val
                            .get("completion_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let total_tokens = usage_val
                            .get("total_tokens")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let usage = custos_provider::conversation::token_usage::Usage::new(
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                        );
                        let provider_usage =
                            custos_provider::conversation::token_usage::ProviderUsage::new(
                                model_name.to_string(),
                                usage,
                            );
                        let _ = tx.send(Ok((None, Some(provider_usage)))).await;
                    }
                }
            }
        }
    }

    Ok(())
}
