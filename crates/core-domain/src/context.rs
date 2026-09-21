//! Context Items & Evidence Anchors
//!
//! Sliced, ranked context elements provided to language models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub source: String,
    pub content: String,
    pub score: f32,
    pub tokens: usize,
}
