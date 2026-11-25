pub mod builder;
pub mod compat;
pub mod context_limit;
pub mod provider;

pub use builder::{
    derive_base_path, ends_with_version_segment, ensure_url_scheme, from_declarative_config,
    parse_custom_headers, parse_openai_base_url, OpenAiBaseUrlParts, OpenAiProviderBuilder,
    DEFAULT_TIMEOUT_SECONDS, OPEN_AI_DEFAULT_BASE_PATH, OPEN_AI_DEFAULT_MODEL, OPEN_AI_DOC_URL,
    OPEN_AI_PROVIDER_NAME, OPEN_AI_VERSIONLESS_BASE_PATH,
};
pub use provider::OpenAiProvider;

#[allow(unused_imports)]
pub(crate) use compat::{
    apply_declared_request_params, meta_reasoning_effort, sanitize_request_for_compat,
    PROVIDERS_NEEDING_MAX_TOKENS_REMAP, PROVIDERS_NEEDING_REASONING_EFFORT_MAPPING,
    PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS, PROVIDERS_NEEDING_TOOL_STREAM,
};
#[allow(unused_imports)]
pub(crate) use context_limit::{
    parse_model_ids, parse_n_ctx_from_models, CachedContextLimit, N_CTX_FAILURE_TTL,
    N_CTX_PROBE_TIMEOUT,
};
#[allow(unused_imports)]
pub(crate) use provider::{OPEN_AI_DEFAULT_MODELS_PATH, OPEN_AI_DEFAULT_RESPONSES_PATH};

#[cfg(test)]
mod tests;
