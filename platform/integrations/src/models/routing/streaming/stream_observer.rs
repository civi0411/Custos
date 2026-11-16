//! Response stream observation, error detection, and frame dispatch.

use super::legacy_decoder::{unary_completion, Decoder};
use super::turn_decoder::ModelTurnStreamDecoder;
use super::types::Wire;
use custos_domain::DomainError;
use custos_provider::turn::{ModelTurnEvent, TurnDelta};
use custos_provider::ProviderEvent;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

pub(crate) type ResponseObserver = (
    std::sync::Arc<dyn super::super::router::CredentialResolver>,
    custos_provider::ProviderRequest,
);

async fn observe_frame(
    observer: &Option<ResponseObserver>,
    frame: &str,
) -> Result<(), DomainError> {
    let Some((resolver, req)) = observer else {
        return Ok(());
    };
    let Some(account) = req.provider_account_id.as_deref() else {
        return Ok(());
    };
    if let Ok(value) = serde_json::from_str::<Value>(frame) {
        let error = value
            .get("error")
            .or_else(|| value.pointer("/response/error"));
        if let Some(error) = error.filter(|error| !error.is_null()) {
            let code = error
                .get("code")
                .filter(|value| value.is_string())
                .or_else(|| error.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let status = match code {
                "insufficient_quota"
                | "usage_limit_reached"
                | "quota_exhausted"
                | "rate_limit_exceeded" => 429,
                "model_not_found" | "model_not_supported" | "model_access_denied" => 404,
                "authentication_error" | "invalid_api_key" => 401,
                _ => error
                    .get("code")
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(200),
            };
            resolver
                .observe_response(
                    account,
                    &req.model,
                    status,
                    &serde_json::json!({"error":error}).to_string(),
                    None,
                )
                .await?;
        }
    }
    Ok(())
}

fn stream_response_observed_inner(
    mut response: reqwest::Response,
    id: String,
    wire: Wire,
    idle: Duration,
    observer: Option<ResponseObserver>,
) -> mpsc::Receiver<ProviderEvent> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let result = async {
            if response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|value| value.split(';').next() == Some("application/json"))
            {
                let mut bytes = Vec::new();
                loop {
                    let chunk = tokio::select! {
                        _ = tx.closed() => return Ok::<(), DomainError>(()),
                        result = tokio::time::timeout(idle, response.chunk()) => {
                            result.map_err(|_| DomainError::Validation("Provider response timed out".into()))?
                                .map_err(|_| DomainError::Validation("Provider response disconnected".into()))?
                        }
                    };
                    let Some(chunk) = chunk else { break; };
                    if bytes.len().saturating_add(chunk.len()) > 2 * 1024 * 1024 {
                        return Err(DomainError::Validation(
                            "Provider response exceeded limit".into(),
                        ));
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let value: Value = serde_json::from_slice(&bytes)
                    .map_err(|_| DomainError::Validation("Malformed provider response".into()))?;
                observe_frame(&observer, &value.to_string()).await?;
                let completion = unary_completion(&id, wire, &value)?;
                let _ = tx.send(completion).await;
                return Ok(());
            }
            let mut pending = Vec::new();
            let mut data = String::new();
            let mut decoder = Decoder::default();
            loop {
                let chunk = tokio::select! {
                    _ = tx.closed() => return Ok::<(), DomainError>(()),
                    result = tokio::time::timeout(idle, response.chunk()) => {
                        result.map_err(|_| DomainError::Validation("Provider stream timed out".into()))?
                            .map_err(|_| DomainError::Validation("Provider stream disconnected".into()))?
                    }
                };
                let eof = chunk.is_none();
                if let Some(chunk) = chunk {
                    pending.extend_from_slice(&chunk);
                }
                if pending.len() > 1024 * 1024 {
                    return Err(DomainError::Validation(
                        "Provider frame exceeded limit".into(),
                    ));
                }
                if eof && !pending.is_empty() {
                    pending.push(b'\n');
                }
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    let bytes: Vec<_> = pending.drain(..=end).collect();
                    let line = std::str::from_utf8(&bytes)
                        .map_err(|_| DomainError::Validation("Invalid provider UTF-8".into()))?
                        .trim_end_matches(['\r', '\n']);
                    let frame = if matches!(wire, Wire::Ollama) {
                        if line.is_empty() {
                            None
                        } else {
                            Some(line.to_string())
                        }
                    } else if line.is_empty() {
                        if data.is_empty() {
                            None
                        } else {
                            Some(std::mem::take(&mut data))
                        }
                    } else {
                        if let Some(part) = line.strip_prefix("data:") {
                            if !data.is_empty() {
                                data.push('\n');
                            }
                            data.push_str(part.trim_start());
                        }
                        None
                    };
                    if data.len() > 1024 * 1024 {
                        return Err(DomainError::Validation(
                            "Provider frame exceeded limit".into(),
                        ));
                    }
                    if let Some(frame) = frame {
                        observe_frame(&observer, &frame).await?;
                        for event in decoder.decode(&id, wire, &frame)? {
                            if tx.send(event).await.is_err() {
                                return Ok(());
                            }
                        }
                        if decoder.completed
                            && (!matches!(wire, Wire::OpenAi) || frame.trim() == "[DONE]")
                        {
                            let _ = tx.send(decoder.completion(&id)).await;
                            return Ok(());
                        }
                    }
                }
                if eof {
                    if !data.is_empty() {
                        observe_frame(&observer, &data).await?;
                        for event in decoder.decode(&id, wire, &data)? {
                            if tx.send(event).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    if !decoder.completed {
                        return Err(DomainError::Validation(
                            "Provider stream ended without completion".into(),
                        ));
                    }
                    let _ = tx.send(decoder.completion(&id)).await;
                    return Ok(());
                }
            }
        }
        .await;
        if let Err(error) = result {
            if error.to_string().contains("disconnected") || error.to_string().contains("timed out")
            {
                if let Some((resolver, req)) = &observer {
                    if let Some(account) = &req.provider_account_id {
                        if let Err(failure) = resolver
                            .observe_response(account, &req.model, 0, "", None)
                            .await
                        {
                            tracing::debug!(%failure, "Stream availability projection failed");
                        }
                    }
                }
            }
            let _ = tx.send(ProviderEvent::failed(&id, error.to_string())).await;
        }
    });
    rx
}

pub fn stream_response_observed(
    response: reqwest::Response,
    req: custos_provider::ProviderRequest,
    wire: Wire,
    resolver: std::sync::Arc<dyn super::super::router::CredentialResolver>,
) -> mpsc::Receiver<ProviderEvent> {
    stream_response_observed_inner(
        response,
        req.request_id.clone(),
        wire,
        Duration::from_secs(45),
        Some((resolver, req)),
    )
}

#[cfg(test)]
pub fn stream_response(
    response: reqwest::Response,
    id: String,
    wire: Wire,
) -> mpsc::Receiver<ProviderEvent> {
    stream_response_with_idle_timeout(response, id, wire, Duration::from_secs(45))
}

#[cfg(test)]
pub fn stream_response_with_idle_timeout(
    response: reqwest::Response,
    id: String,
    wire: Wire,
    idle: Duration,
) -> mpsc::Receiver<ProviderEvent> {
    stream_response_observed_inner(response, id, wire, idle, None)
}

pub fn stream_turn_response_observed(
    mut response: reqwest::Response,
    attempt_id: Uuid,
    actual_provider: String,
    actual_model: String,
    wire: Wire,
    idle: Duration,
) -> mpsc::Receiver<ModelTurnEvent> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let mut decoder = ModelTurnStreamDecoder::default();
        let prov = actual_provider.clone();
        let modl = actual_model.clone();
        let result = async {
            let mut pending = Vec::new();
            let mut data = String::new();
            loop {
                let chunk = tokio::select! {
                    _ = tx.closed() => return Ok::<(), DomainError>(()),
                    res = tokio::time::timeout(idle, response.chunk()) => {
                        res.map_err(|_| DomainError::Validation("Provider stream timed out".into()))?
                           .map_err(|_| DomainError::Validation("Provider stream disconnected".into()))?
                    }
                };
                let eof = chunk.is_none();
                if let Some(chunk) = chunk {
                    pending.extend_from_slice(&chunk);
                }
                if pending.len() > 1024 * 1024 {
                    return Err(DomainError::Validation(
                        "Provider frame exceeded limit".into(),
                    ));
                }
                if eof && !pending.is_empty() {
                    pending.push(b'\n');
                }
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    let bytes: Vec<_> = pending.drain(..=end).collect();
                    let line = std::str::from_utf8(&bytes)
                        .map_err(|_| DomainError::Validation("Invalid provider UTF-8".into()))?
                        .trim_end_matches(['\r', '\n']);
                    let frame = if matches!(wire, Wire::Ollama) {
                        if line.is_empty() {
                            None
                        } else {
                            Some(line.to_string())
                        }
                    } else if line.is_empty() {
                        if data.is_empty() {
                            None
                        } else {
                            Some(std::mem::take(&mut data))
                        }
                    } else {
                        if let Some(part) = line.strip_prefix("data:") {
                            if !data.is_empty() {
                                data.push('\n');
                            }
                            data.push_str(part.trim_start());
                        }
                        None
                    };
                    if data.len() > 1024 * 1024 {
                        return Err(DomainError::Validation(
                            "Provider frame exceeded limit".into(),
                        ));
                    }
                    if let Some(frame) = frame {
                        let evts = decoder.decode(attempt_id, &prov, &modl, wire, &frame)?;
                        for evt in evts {
                            if tx.send(evt).await.is_err() {
                                return Ok(());
                            }
                        }
                        if decoder.completed
                            && (!matches!(wire, Wire::OpenAi) || frame.trim() == "[DONE]")
                        {
                            return Ok(());
                        }
                    }
                }
                if eof {
                    if !data.is_empty() {
                        let evts = decoder.decode(attempt_id, &prov, &modl, wire, &data)?;
                        for evt in evts {
                            if tx.send(evt).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                    if !decoder.completed {
                        decoder.sequence += 1;
                        let _ = tx
                            .send(ModelTurnEvent {
                                attempt_id,
                                sequence: decoder.sequence,
                                actual_provider: Some(prov.clone()),
                                actual_model: Some(modl.clone()),
                                actual_transport: Some("turn_sse".to_string()),
                                delta: TurnDelta::Completed {
                                    finish_reason: Some("stop".to_string()),
                                },
                                usage_certainty: Some("estimated".to_string()),
                                provider_response_id: None,
                            })
                            .await;
                    }
                    return Ok(());
                }
            }
        }
        .await;
        if let Err(err) = result {
            decoder.sequence += 1;
            let _ = tx
                .send(ModelTurnEvent {
                    attempt_id,
                    sequence: decoder.sequence,
                    actual_provider: Some(prov),
                    actual_model: Some(modl),
                    actual_transport: Some("turn_sse".to_string()),
                    delta: TurnDelta::Failed {
                        error_class: "stream_error".to_string(),
                        message: err.to_string(),
                    },
                    usage_certainty: None,
                    provider_response_id: None,
                })
                .await;
        }
    });
    rx
}

