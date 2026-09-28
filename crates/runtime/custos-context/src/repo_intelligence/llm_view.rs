//! LLM-Optimized Code Graph Views & Prompt Slicers
//!
//! Transforms the deep relational CodeGraph into token-efficient, highly structured
//! prompt contexts tailored for LLM reasoning (Claude, GPT, Gemini, local models).

use crate::graph::{CodeGraph, NodeKind};
use std::fmt::Write;

/// Formatting modes for LLM prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmViewFormat {
    /// Dense Markdown outline designed for top-level system prompts (< 1,500 tokens).
    CompactSkeleton,
    /// XML-tagged structured graph slice (<code_graph>...</code_graph>) for RAG queries.
    XmlNeighborhood,
    /// Detailed Markdown contract sheet highlighting traits, signatures, and invariants.
    ContractSpecification,
}

pub struct LlmContextBuilder<'a> {
    graph: &'a CodeGraph,
}

impl<'a> LlmContextBuilder<'a> {
    pub fn new(graph: &'a CodeGraph) -> Self {
        Self { graph }
    }

    /// Generates a complete, ultra-compact architectural overview of the repository.
    /// Perfectly suited for the System Prompt / System One fast context.
    pub fn build_repo_skeleton(&self) -> String {
        let mut out = String::new();
        out.push_str("```codebase-architecture\n");
        out.push_str("// CUSTOS SYSTEM ARCHITECTURE SKELETON (Auto-generated)\n\n");

        // Group nodes by crate
        let mut crates = Vec::new();
        for node in self.graph.nodes.values() {
            if let NodeKind::Crate { name, tier, .. } = &node.kind {
                crates.push((name.clone(), tier.clone(), node.id.clone()));
            }
        }
        crates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        for (crate_name, tier, crate_id) in crates {
            let _ = writeln!(out, "Crate: {} [{}]", crate_name, tier);

            // Find contained symbols via outgoing Contains edges
            if let Some(edges) = self.graph.outgoing.get(&crate_id) {
                let mut traits = Vec::new();
                let mut structs = Vec::new();

                for edge in edges {
                    if let Some(child_node) = self.graph.nodes.get(&edge.to) {
                        match &child_node.kind {
                            NodeKind::Trait { name, .. } => traits.push(name.as_str()),
                            NodeKind::Struct { name, .. } => structs.push(name.as_str()),
                            _ => {}
                        }
                    }
                }

                if !traits.is_empty() {
                    let _ = writeln!(out, "  Contracts (traits): {}", traits.join(", "));
                }
                if !structs.is_empty() {
                    let preview = if structs.len() > 6 {
                        format!(
                            "{}, ... (+{} more)",
                            structs[..6].join(", "),
                            structs.len() - 6
                        )
                    } else {
                        structs.join(", ")
                    };
                    let _ = writeln!(out, "  Entities (structs): {}", preview);
                }
            }
            let _ = writeln!(out);
        }

        out.push_str("```\n");
        out
    }

    /// Extracts a high-fidelity XML context block around a specific symbol.
    /// Provides the exact structural neighborhood needed for coding and verification.
    pub fn build_symbol_neighborhood(&self, symbol_name: &str, max_hops: usize) -> Option<String> {
        let matching_nodes = self.graph.find_by_name(symbol_name);
        if matching_nodes.is_empty() {
            return None;
        }

        let focal_node = matching_nodes[0];
        let neighborhood = self.graph.extract_neighborhood(&focal_node.id, max_hops);

        let mut out = String::new();
        let _ = writeln!(
            out,
            "<code_graph focal_symbol=\"{}\">",
            focal_node.id.as_str()
        );

        // 1. Focal Node details
        let _ = writeln!(
            out,
            "  <target_node id=\"{}\" type=\"{}\">",
            focal_node.id.as_str(),
            focal_node.kind.type_name()
        );
        if let Some(fp) = &focal_node.file_path {
            let line = focal_node.line_number.unwrap_or(1);
            let _ = writeln!(out, "    <location file=\"{}\" line=\"{}\" />", fp, line);
        }
        let _ = writeln!(out, "  </target_node>");

        // 2. Relational Edges
        let _ = writeln!(out, "  <relationships>");
        if let Some(outgoing) = self.graph.outgoing.get(&focal_node.id) {
            for edge in outgoing {
                let _ = writeln!(
                    out,
                    "    <edge kind=\"{:?}\" direction=\"outgoing\" target=\"{}\" />",
                    edge.kind,
                    edge.to.as_str()
                );
            }
        }
        if let Some(incoming) = self.graph.incoming.get(&focal_node.id) {
            for edge in incoming {
                let _ = writeln!(
                    out,
                    "    <edge kind=\"{:?}\" direction=\"incoming\" source=\"{}\" />",
                    edge.kind,
                    edge.from.as_str()
                );
            }
        }
        let _ = writeln!(out, "  </relationships>");

        // 3. Connected Neighbor Summary
        let _ = writeln!(
            out,
            "  <connected_symbols count=\"{}\">",
            neighborhood.len()
        );
        for node in neighborhood {
            if node.id != focal_node.id {
                let _ = writeln!(
                    out,
                    "    <symbol id=\"{}\" type=\"{}\" file=\"{}\" />",
                    node.id.as_str(),
                    node.kind.type_name(),
                    node.file_path.as_deref().unwrap_or("unknown")
                );
            }
        }
        let _ = writeln!(out, "  </connected_symbols>");

        out.push_str("</code_graph>\n");
        Some(out)
    }

    /// Packs a set of symbols within a strict token budget.
    /// Uses a heuristic of ~4 characters per token.
    pub fn pack_within_token_budget(&self, symbol_names: &[&str], max_tokens: usize) -> String {
        let max_chars = max_tokens * 4;
        let mut accumulated = String::new();

        for name in symbol_names {
            if let Some(slice) = self.build_symbol_neighborhood(name, 1) {
                if accumulated.len() + slice.len() > max_chars {
                    break;
                }
                accumulated.push_str(&slice);
                accumulated.push('\n');
            }
        }

        accumulated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{CodeNode, EdgeKind, NodeKind, SymbolId};

    #[test]
    fn test_llm_view_and_skeleton() {
        let mut graph = CodeGraph::new();

        let crate_id = SymbolId::crate_node("custos-core");
        graph.add_node(CodeNode {
            id: crate_id.clone(),
            kind: NodeKind::Crate {
                name: "custos-core".into(),
                version: "0.1.0".into(),
                tier: "Core Domain".into(),
            },
            file_path: None,
            line_number: None,
            importance_score: 1.0,
        });

        let trait_id = SymbolId::symbol_node("custos-core", "DomainService");
        graph.add_node(CodeNode {
            id: trait_id.clone(),
            kind: NodeKind::Trait {
                name: "DomainService".into(),
                visibility: "pub".into(),
                doc_comment: None,
            },
            file_path: Some("crates/core/src/lib.rs".into()),
            line_number: Some(15),
            importance_score: 1.0,
        });

        graph.add_edge(crate_id, trait_id.clone(), EdgeKind::Contains, None);

        let builder = LlmContextBuilder::new(&graph);
        let skeleton = builder.build_repo_skeleton();
        assert!(skeleton.contains("custos-core"));
        assert!(skeleton.contains("DomainService"));

        let neighborhood = builder.build_symbol_neighborhood("DomainService", 1);
        assert!(neighborhood.is_some());
        let xml = neighborhood.unwrap();
        assert!(xml.contains("<code_graph"));
        assert!(xml.contains("DomainService"));
    }
}
