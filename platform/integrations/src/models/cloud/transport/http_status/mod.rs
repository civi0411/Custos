//! Format-agnostic HTTP status → `ProviderError` mapping.
//!
//! Used by providers regardless of their wire format (OpenAI, Anthropic,
//! Google, etc.). Parses both `{"error":{"message":"..."}}` and
//! `{"message":"..."}` error shapes.

pub mod error_mapper;
pub mod response_handler;
pub mod retry;

#[cfg(test)]
mod tests;

pub use error_mapper::{
    is_context_length_exceeded, is_context_length_exceeded_message,
    map_http_error_to_provider_error, sanitize_url,
};
pub use response_handler::{
    handle_response, handle_response_with_limit, handle_status, handle_status_with_limit,
    read_error_body, read_json_response, read_json_response_with_limit,
    read_response_body_with_limit, send_bounded, set_response_deadline, ResponseDeadline,
    MAX_PROVIDER_JSON_RESPONSE_BYTES,
};
pub use retry::{
    duration_from_finite_secs, extract_retry_after, parse_http_date, parse_retry_after_header,
    MAX_RETRY_AFTER_SECS,
};
