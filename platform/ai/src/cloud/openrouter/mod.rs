pub mod gemini_compat;
pub mod provider;

pub use provider::{
    OpenRouterProvider, OpenRouterSessionIdProvider, OPENROUTER_DECISION_DEFAULT_MODEL,
    OPENROUTER_DEFAULT_MODEL, OPENROUTER_DOC_URL, OPENROUTER_KNOWN_MODELS, OPENROUTER_PROVIDER_NAME,
};

#[allow(unused_imports)]
pub(crate) use gemini_compat::{
    apply_gemini_compatibility, collision_free_gemini_schema_ref_key, decode_unit,
    escape_gemini_schema_ref_keys_in_tool_responses, is_gemini_model, match_schema_ref_at,
    replace_spans, scan_schema_ref_tokens,
};
#[allow(unused_imports)]
pub(crate) use provider::{
    is_mandatory_reasoning_error, merge_openrouter_parameters, merge_request_params,
    OPENROUTER_PARAMETERS_CONFIG_KEY,
};

#[cfg(test)]
mod tests;
