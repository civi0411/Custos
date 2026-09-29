pub mod base;
pub mod cache_semantics;
pub mod canonical;
pub mod context_limit;
pub mod conversation;
pub mod custos_mode;
pub mod documents;
pub mod errors;
pub mod formats;
/// Compatibility module retained for older clients; new code should use `custos_mode`.
#[deprecated(note = "use custos_mode")]
pub mod goose_mode {
    #[allow(deprecated)]
    pub use crate::custos_mode::GooseMode;
}
pub mod images;
pub mod json;
pub(crate) mod mcp_utils;
pub mod model;
pub mod permission;
pub mod request_log;
pub mod retry;
pub mod thinking;
pub mod utils;
