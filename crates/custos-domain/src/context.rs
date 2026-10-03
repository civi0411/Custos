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
    #[serde(default)]
    pub provenance_hash: Option<String>,
    #[serde(default)]
    pub is_tainted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    pub id: String,
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
    #[serde(default)]
    pub context_digest: String,
}

impl ContextPack {
    pub fn new(
        id: String,
        items: Vec<ContextItem>,
        total_tokens: usize,
        context_digest: String,
    ) -> Self {
        Self {
            id,
            items,
            total_tokens,
            context_digest,
        }
    }
}
