use std::time::Duration;

use futures::TryStreamExt;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error_mapper::{map_http_error_to_provider_error, sanitize_url};
use super::retry::extract_retry_after;
use crate::errors::ProviderError;

pub const MAX_PROVIDER_JSON_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
pub struct ResponseDeadline(tokio::time::Instant);

pub fn set_response_deadline(response: &mut Response, deadline: tokio::time::Instant) {
    response.extensions_mut().insert(ResponseDeadline(deadline));
}

pub async fn send_bounded(
    request: reqwest::RequestBuilder,
    timeout: Duration,
) -> Result<Response, ProviderError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut response = tokio::time::timeout_at(deadline, request.send())
        .await
        .map_err(|_| {
            ProviderError::NetworkError(
                "Request timed out — check your network connection and try again.".to_string(),
            )
        })??;
    set_response_deadline(&mut response, deadline);
    Ok(response)
}

pub async fn read_response_body_with_limit(
    response: Response,
    limit: usize,
) -> Result<Vec<u8>, ProviderError> {
    let deadline = response.extensions().get::<ResponseDeadline>().copied();
    let read = async move {
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();

        while let Some(chunk) = stream.try_next().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to read response body: {e}"))
        })? {
            if chunk.len() > limit.saturating_sub(body.len()) {
                return Err(ProviderError::RequestFailed(format!(
                    "Provider response body exceeds the {limit} byte limit"
                )));
            }
            body.try_reserve(chunk.len()).map_err(|_| {
                ProviderError::RequestFailed("Failed to allocate response body".to_string())
            })?;
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    };

    match deadline {
        Some(ResponseDeadline(deadline)) => {
            tokio::time::timeout_at(deadline, read).await.map_err(|_| {
                ProviderError::NetworkError(
                    "Response body timed out — check your network connection and try again."
                        .to_string(),
                )
            })?
        }
        None => read.await,
    }
}

pub async fn read_error_body(response: Response) -> Option<String> {
    read_response_body_with_limit(response, MAX_PROVIDER_JSON_RESPONSE_BYTES)
        .await
        .ok()
        .map(|body| String::from_utf8_lossy(&body).into_owned())
}

pub async fn read_json_response<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ProviderError> {
    read_json_response_with_limit(response, MAX_PROVIDER_JSON_RESPONSE_BYTES).await
}

pub async fn read_json_response_with_limit<T: DeserializeOwned>(
    response: Response,
    limit: usize,
) -> Result<T, ProviderError> {
    let body = read_response_body_with_limit(response, limit).await?;
    serde_json::from_slice(&body)
        .map_err(|e| ProviderError::RequestFailed(format!("Response body is not valid JSON: {e}")))
}

pub async fn handle_status(response: Response) -> Result<Response, ProviderError> {
    handle_status_with_limit(response, MAX_PROVIDER_JSON_RESPONSE_BYTES).await
}

pub async fn handle_status_with_limit(
    response: Response,
    limit: usize,
) -> Result<Response, ProviderError> {
    let status = response.status();
    if !status.is_success() {
        let url = sanitize_url(response.url().as_str());
        let headers = response.headers().clone();
        let body = read_response_body_with_limit(response, limit)
            .await
            .unwrap_or_default();
        let body = String::from_utf8_lossy(&body);
        let payload = serde_json::from_str::<Value>(&body).ok();
        let mut err = map_http_error_to_provider_error(status, payload.clone(), &url);
        if let ProviderError::RateLimitExceeded { details, .. } = &err {
            err = ProviderError::RateLimitExceeded {
                details: details.clone(),
                retry_delay: extract_retry_after(&headers, payload.as_ref()),
            };
        }
        return Err(err);
    }
    Ok(response)
}

pub async fn handle_response(response: Response) -> Result<Value, ProviderError> {
    handle_response_with_limit(response, MAX_PROVIDER_JSON_RESPONSE_BYTES).await
}

pub async fn handle_response_with_limit(
    response: Response,
    limit: usize,
) -> Result<Value, ProviderError> {
    let response = handle_status_with_limit(response, limit).await?;
    read_json_response_with_limit(response, limit).await
}
