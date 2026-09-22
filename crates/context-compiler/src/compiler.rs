//! Token-Aware Context Compiler Implementation
//!
//! Provides relevance scoring, context ranking, and token budget enforcement.

use crate::traits::{ContextBuilder, ContextSlice};
use async_trait::async_trait;
use custos_core_domain::{new_id, ContextItem, DomainError};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SourceDocument {
    pub path: String,
    pub content: String,
}

impl SourceDocument {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p)?;
        Ok(Self {
            path: p.to_string_lossy().to_string(),
            content,
        })
    }
}

/// Token-aware compiler that packs relevant context items under a strict token budget.
#[derive(Debug, Clone, Default)]
pub struct TokenAwareContextCompiler {
    documents: Vec<SourceDocument>,
}

impl TokenAwareContextCompiler {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    pub fn with_documents(documents: Vec<SourceDocument>) -> Self {
        Self { documents }
    }

    pub fn add_document(&mut self, doc: SourceDocument) {
        self.documents.push(doc);
    }

    /// Fast, deterministic token estimation (~4 characters per token average for code/prose).
    pub fn estimate_tokens(text: &str) -> usize {
        let chars = text.chars().count();
        if chars == 0 {
            0
        } else {
            (chars + 3) / 4
        }
    }

    /// Calculates a lexical relevance score for a document against a search query.
    pub fn score_relevance(query: &str, doc: &SourceDocument) -> f32 {
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .collect();

        if terms.is_empty() {
            return 1.0;
        }

        let path_lower = doc.path.to_lowercase();
        let content_lower = doc.content.to_lowercase();

        let mut score = 0.0f32;

        // Path / filename bonus
        for term in &terms {
            if path_lower.contains(term) {
                score += 5.0;
            }
        }

        // Exact phrase match in content bonus
        if content_lower.contains(&query_lower) {
            score += 10.0;
        }

        // Term frequency match in content
        for term in &terms {
            let count = content_lower.matches(term).count();
            if count > 0 {
                score += (count as f32).min(10.0) * 1.5;
            }
        }

        score
    }
}

#[async_trait]
impl ContextBuilder for TokenAwareContextCompiler {
    async fn compile(&self, query: &str, max_tokens: usize) -> Result<ContextSlice, DomainError> {
        let mut scored: Vec<(f32, &SourceDocument)> = self
            .documents
            .iter()
            .map(|doc| (Self::score_relevance(query, doc), doc))
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // Sort descending by relevance score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut items = Vec::new();
        let mut total_tokens = 0usize;

        for (score, doc) in scored {
            let doc_tokens = Self::estimate_tokens(&doc.content);

            // Check if document fits into remaining budget
            if total_tokens + doc_tokens <= max_tokens {
                items.push(ContextItem {
                    id: new_id("ctx"),
                    source: doc.path.clone(),
                    content: doc.content.clone(),
                    score,
                    tokens: doc_tokens,
                });
                total_tokens += doc_tokens;
            } else {
                // If it doesn't fit entirely, slice what can fit if remaining budget is at least 5 tokens
                let remaining_tokens = max_tokens.saturating_sub(total_tokens);
                if remaining_tokens >= 5 {
                    let approx_chars = remaining_tokens * 4;
                    let sliced_content: String = doc.content.chars().take(approx_chars).collect();
                    let actual_tokens = Self::estimate_tokens(&sliced_content);
                    if actual_tokens > 0 {
                        items.push(ContextItem {
                            id: new_id("ctx"),
                            source: format!("{} (truncated)", doc.path),
                            content: sliced_content,
                            score,
                            tokens: actual_tokens,
                        });
                        total_tokens += actual_tokens;
                    }
                }
                break;
            }
        }

        Ok(ContextSlice {
            items,
            total_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_budget_enforcement() {
        let mut compiler = TokenAwareContextCompiler::new();
        compiler.add_document(SourceDocument::new(
            "crates/task-kernel/src/lib.rs",
            "task kernel manages state transitions and reduces events",
        ));
        compiler.add_document(SourceDocument::new(
            "crates/persistence-sqlite/src/lib.rs",
            "persistence sqlite handles task storage and database persistence",
        ));

        // Very small budget: 15 tokens
        let slice = compiler
            .compile("persistence", 15)
            .await
            .expect("Compile must succeed");

        assert!(slice.total_tokens <= 15);
        assert!(!slice.items.is_empty());
        assert!(slice.items[0].source.starts_with("crates/persistence-sqlite/src/lib.rs"));
    }

    #[tokio::test]
    async fn test_relevance_ranking() {
        let mut compiler = TokenAwareContextCompiler::new();
        compiler.add_document(SourceDocument::new(
            "other.md",
            "completely unrelated document about flowers and trees",
        ));
        compiler.add_document(SourceDocument::new(
            "storage.md",
            "This document explains the database persistence layer and SQLite store.",
        ));

        let slice = compiler
            .compile("database persistence", 1000)
            .await
            .expect("Compile must succeed");

        assert_eq!(slice.items.len(), 1);
        assert_eq!(slice.items[0].source, "storage.md");
        assert!(slice.items[0].score > 5.0);
    }
}
