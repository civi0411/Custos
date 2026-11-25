use crate::model::ModelConfig;
use serde_json::{json, Value};

pub const OLLAMA_PROVIDER_NAME: &str = "ollama";
pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_TIMEOUT: u64 = 600;
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen3";
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[
    OLLAMA_DEFAULT_MODEL,
    "qwen3-vl",
    "qwen3-coder:30b",
    "qwen3-coder:480b-cloud",
];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

pub const OLLAMA_MAX_RETRIES: usize = 10;
pub const OLLAMA_INITIAL_RETRY_INTERVAL_MS: u64 = 2000;
pub const OLLAMA_BACKOFF_MULTIPLIER: f64 = 1.5;
pub const OLLAMA_MAX_RETRY_INTERVAL_MS: u64 = 15_000;

pub const OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS: u64 = 120;

/// Provider settings resolved from `config::Config` at construction time.
///
/// All values that the Ollama provider reads out of the global config are
/// resolved once, up front, and carried here. This keeps the streaming hot
/// path free of config lookups and makes the provider's config dependencies
/// explicit at the construction boundary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaOptions {
    /// Explicit context window override from `GOOSE_INPUT_LIMIT`.
    /// `None` when unset, zero, or invalid; `num_ctx` is then omitted so Ollama
    /// uses its model default.
    pub input_limit: Option<usize>,
    /// Whether to keep `stream_options` in the request (`OLLAMA_STREAM_USAGE`,
    /// default `true`).
    pub stream_usage: bool,
    /// Per-chunk stream timeout in seconds, resolved from
    /// `OLLAMA_STREAM_TIMEOUT` > `GOOSE_STREAM_TIMEOUT` > `OLLAMA_TIMEOUT` >
    /// default (120s).
    pub chunk_timeout_secs: u64,
}

impl Default for OllamaOptions {
    fn default() -> Self {
        Self {
            input_limit: None,
            stream_usage: true,
            chunk_timeout_secs: OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS,
        }
    }
}

pub(crate) fn resolve_ollama_num_ctx(options: &OllamaOptions) -> Option<usize> {
    options.input_limit
}

pub(crate) fn apply_ollama_options(
    payload: &mut Value,
    options: &OllamaOptions,
    _model_config: &ModelConfig,
) {
    if let Some(obj) = payload.as_object_mut() {
        // Gate stream_options behind OLLAMA_STREAM_USAGE (default: true).
        // Older Ollama builds that don't support stream_options may stall before
        // emitting any SSE data, blocking until the client timeout (600s).
        // with_line_timeout() only protects after the first line arrives, so
        // users on older builds should set OLLAMA_STREAM_USAGE=false.
        if !options.stream_usage {
            obj.remove("stream_options");
        }

        // Convert max_completion_tokens / max_tokens to Ollama's options.num_predict.
        // Reasoning models emit max_completion_tokens; non-reasoning models emit max_tokens.
        let max_tokens = obj
            .remove("max_completion_tokens")
            .or_else(|| obj.remove("max_tokens"));
        if let Some(max_tokens) = max_tokens {
            let options_value = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options_value.as_object_mut() {
                options_obj.entry("num_predict").or_insert(max_tokens);
            }
        }

        // Only an explicit GOOSE_INPUT_LIMIT overrides Ollama's num_ctx.
        if let Some(limit) = resolve_ollama_num_ctx(options) {
            let options_value = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options_value.as_object_mut() {
                options_obj.insert("num_ctx".to_string(), json!(limit));
            }
        }
    }
}
