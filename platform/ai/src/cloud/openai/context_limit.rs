use crate::errors::ProviderError;
use std::time::{Duration, Instant};

pub const N_CTX_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
pub const N_CTX_FAILURE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
pub enum CachedContextLimit {
    Success(Option<usize>),
    Failure(Instant),
}

impl CachedContextLimit {
    pub fn value(self) -> Option<Option<usize>> {
        match self {
            Self::Success(limit) => Some(limit),
            Self::Failure(fetched_at) if fetched_at.elapsed() < N_CTX_FAILURE_TTL => Some(None),
            Self::Failure(_) => None,
        }
    }
}

pub fn parse_model_ids(json: &serde_json::Value) -> Result<Vec<String>, ProviderError> {
    let models = json
        .get("data")
        .and_then(|value| value.as_array())
        .or_else(|| json.as_array())
        .ok_or_else(|| {
            ProviderError::RequestFailed("Missing models array in JSON response".into())
        })?;
    let mut model_ids: Vec<String> = models
        .iter()
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    model_ids.sort();
    Ok(model_ids)
}

/// Extract `meta.n_ctx` for `model_name` from a `/v1/models` response body.
pub fn parse_n_ctx_from_models(json: &serde_json::Value, model_name: &str) -> Option<usize> {
    let data = json.get("data")?.as_array()?;

    let n_ctx = |entry: &serde_json::Value| -> Option<usize> {
        entry
            .get("meta")?
            .get("n_ctx")?
            .as_u64()
            .map(|v| v as usize)
    };

    if let Some(entry) = data
        .iter()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(model_name))
    {
        return n_ctx(entry);
    }

    // For single-model servers without --alias, llama.cpp reports the loaded model
    // file path as id rather than the client's alias, so no entry matches above.
    // Fall back to the sole entry's n_ctx.
    match data.as_slice() {
        [only] => n_ctx(only),
        _ => None,
    }
}
