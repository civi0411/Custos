//! Anchor Retrieval for Context Compilation
//!
//! Extracts relevant excerpt windows centered on search anchors/keywords (Custos.md §6.4 Step 3).

use std::collections::BTreeSet;

pub struct AnchorRetriever;

impl AnchorRetriever {
    /// Extracts excerpt windows around anchor matches in the file content.
    /// If content has <= 40 lines or query is empty, returns original content.
    pub fn retrieve_anchors(content: &str, query: &str, window_radius: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= 40 || query.trim().is_empty() {
            return content.to_string();
        }

        let query_terms: Vec<String> = query
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| w.len() >= 3)
            .collect();

        if query_terms.is_empty() {
            return content.to_string();
        }

        // Collect matching line indices
        let mut matched_indices = BTreeSet::new();
        for (idx, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            if query_terms.iter().any(|term| line_lower.contains(term)) {
                let start = idx.saturating_sub(window_radius);
                let end = (idx + window_radius + 1).min(lines.len());
                for i in start..end {
                    matched_indices.insert(i);
                }
            }
        }

        if matched_indices.is_empty() {
            // If no exact term matched, return first 40 lines as preview
            return lines.iter().take(40).cloned().collect::<Vec<_>>().join("\n");
        }

        // Render matched lines with continuity breaks
        let mut result = Vec::new();
        let mut prev_idx: Option<usize> = None;

        for &idx in &matched_indices {
            if let Some(prev) = prev_idx {
                if idx > prev + 1 {
                    result.push("... [gap] ...");
                }
            }
            result.push(lines[idx]);
            prev_idx = Some(idx);
        }

        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieve_anchors_with_gaps() {
        let mut doc = String::new();
        for i in 0..100 {
            if i == 20 {
                doc.push_str("pub fn target_anchor_function() {\n");
            } else if i == 80 {
                doc.push_str("pub fn another_anchor_match() {\n");
            } else {
                doc.push_str(&format!("// line {i} boilerplate\n"));
            }
        }

        let excerpts = AnchorRetriever::retrieve_anchors(&doc, "anchor target", 2);
        assert!(excerpts.contains("pub fn target_anchor_function()"));
        assert!(excerpts.contains("pub fn another_anchor_match()"));
        assert!(excerpts.contains("... [gap] ..."));
        assert!(!excerpts.contains("// line 50 boilerplate"));
    }
}
