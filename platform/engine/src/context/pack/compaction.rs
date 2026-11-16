//! Progressive Compaction for Context Compilation
//!
//! Compacts text when context exceeds token budget (Custos.md §6.4 Step 6):
//! 1. Remove consecutive blank lines
//! 2. Strip single-line comments
//! 3. Prune lowest-ranked items until within token budget

use custos_domain::ContextItem;

pub struct ProgressiveCompactor;

impl ProgressiveCompactor {
    /// Estimates token count of a given string (approx 4 chars per token).
    pub fn estimate_tokens(text: &str) -> usize {
        text.len().div_ceil(4)
    }

    /// Progressively compacts context items to strictly fit within `max_tokens`.
    pub fn compact(mut items: Vec<ContextItem>, max_tokens: usize) -> Vec<ContextItem> {
        let current_tokens: usize = items.iter().map(|it| it.tokens).sum();
        if current_tokens <= max_tokens {
            return items;
        }

        // Pass 1: Strip extra blank lines
        for item in &mut items {
            item.content = Self::strip_blank_lines(&item.content);
            item.tokens = Self::estimate_tokens(&item.content);
        }

        let current_tokens: usize = items.iter().map(|it| it.tokens).sum();
        if current_tokens <= max_tokens {
            return items;
        }

        // Pass 2: Strip comments
        for item in &mut items {
            item.content = Self::strip_comments(&item.content);
            item.tokens = Self::estimate_tokens(&item.content);
        }

        let current_tokens: usize = items.iter().map(|it| it.tokens).sum();
        if current_tokens <= max_tokens {
            return items;
        }

        // Pass 3: Sort by score descending and prune lowest scoring items
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut compacted = Vec::new();
        let mut accumulated_tokens = 0;

        for item in items {
            if accumulated_tokens + item.tokens <= max_tokens {
                accumulated_tokens += item.tokens;
                compacted.push(item);
            }
        }

        compacted
    }

    fn strip_blank_lines(text: &str) -> String {
        let mut lines = Vec::new();
        let mut prev_blank = false;

        for line in text.lines() {
            let is_blank = line.trim().is_empty();
            if is_blank {
                if !prev_blank {
                    lines.push("");
                    prev_blank = true;
                }
            } else {
                lines.push(line);
                prev_blank = false;
            }
        }

        lines.join("\n")
    }

    fn strip_comments(text: &str) -> String {
        let mut lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") && !trimmed.starts_with("///") {
                // Skip non-doc comments
                continue;
            }
            lines.push(line);
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_within_budget_untouched() {
        let items = vec![ContextItem {
            id: "1".into(),
            source: "src.rs".into(),
            content: "fn test() {}".into(),
            score: 1.0,
            tokens: 5,
            provenance_hash: None,
            is_tainted: false,
        }];

        let result = ProgressiveCompactor::compact(items, 100);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_compaction_strips_comments_and_blank_lines() {
        let original = "fn test() {\n\n\n    // regular comment\n    println!(\"hi\");\n}";
        let items = vec![ContextItem {
            id: "1".into(),
            source: "src.rs".into(),
            content: original.into(),
            score: 1.0,
            tokens: 30,
            provenance_hash: None,
            is_tainted: false,
        }];

        // Budget of 10 forces Pass 2 (comment stripping)
        let result = ProgressiveCompactor::compact(items, 10);
        assert_eq!(result.len(), 1);
        assert!(!result[0].content.contains("// regular comment"));
    }

    #[test]
    fn test_compaction_prunes_lowest_score_when_over_budget() {
        let items = vec![
            ContextItem {
                id: "high_score".into(),
                source: "src1.rs".into(),
                content: "fn high_precision_calculation_operation() { 42 }".into(),
                score: 0.95,
                tokens: 15,
                provenance_hash: None,
                is_tainted: false,
            },
            ContextItem {
                id: "low_score".into(),
                source: "src2.rs".into(),
                content: "fn low_priority_logging_debug_helper() { 0 }".into(),
                score: 0.2,
                tokens: 15,
                provenance_hash: None,
                is_tainted: false,
            },
        ];

        // Total tokens is ~26. Budget only allows 1 item (15 tokens)
        let result = ProgressiveCompactor::compact(items, 15);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "high_score");
    }
}
