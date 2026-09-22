//! Core Primitives & Utility Re-exports
//!
//! Preserves backwards compatibility while delegating to specialized modules:
//! - `error.rs` for `DomainError`
//! - `ids.rs` for `new_id`, `digest`, `canonical_json`
//! - `artifact.rs` for `ArtifactRef`

pub use crate::artifact::ArtifactRef;
pub use crate::error::DomainError;
pub use crate::ids::{canonical_json, digest, new_id};
