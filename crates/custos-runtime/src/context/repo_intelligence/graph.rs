//! Standardized Code Graph Representation for LLM Intelligence & Memory
//!
//! Provides a multi-layer property graph representing codebases at crate, file,
//! contract (trait), entity (struct/enum), and functional (call) granularity.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Canonical identifier for any node in the codebase.
/// Examples:
/// - `crate::custos_task_kernel`
/// - `file::crates/task-kernel/src/lib.rs`
/// - `symbol::custos_task_kernel::TaskKernel`
/// - `trait::custos_context_compiler::ContextBuilder`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn crate_node(name: &str) -> Self {
        Self(format!("crate::{}", name))
    }

    pub fn file_node(rel_path: &str) -> Self {
        Self(format!("file::{}", rel_path))
    }

    pub fn symbol_node(parent_path: &str, name: &str) -> Self {
        Self(format!("symbol::{}::{}", parent_path, name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Category of the code node in the architectural hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Architectural boundaries (e.g., Cargo package / workspace crate)
    Crate {
        name: String,
        version: String,
        tier: String,
    },
    /// Source file node
    File {
        relative_path: String,
        line_count: usize,
        size_bytes: u64,
        sha256_hash: Option<String>,
    },
    /// Abstract contract / interface (Rust trait, TS interface)
    Trait {
        name: String,
        visibility: String,
        doc_comment: Option<String>,
    },
    /// Concrete state entity (Rust struct, TS class)
    Struct {
        name: String,
        visibility: String,
        doc_comment: Option<String>,
    },
    /// State machine or algebraic data type (Rust enum)
    Enum {
        name: String,
        visibility: String,
        doc_comment: Option<String>,
    },
    /// Callable routine / method
    Function {
        name: String,
        visibility: String,
        is_async: bool,
        signature: String,
        doc_comment: Option<String>,
    },
}

impl NodeKind {
    pub fn display_name(&self) -> &str {
        match self {
            NodeKind::Crate { name, .. } => name,
            NodeKind::File { relative_path, .. } => relative_path,
            NodeKind::Trait { name, .. } => name,
            NodeKind::Struct { name, .. } => name,
            NodeKind::Enum { name, .. } => name,
            NodeKind::Function { name, .. } => name,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            NodeKind::Crate { .. } => "crate",
            NodeKind::File { .. } => "file",
            NodeKind::Trait { .. } => "trait",
            NodeKind::Struct { .. } => "struct",
            NodeKind::Enum { .. } => "enum",
            NodeKind::Function { .. } => "fn",
        }
    }
}

/// Rich metadata attached to every node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    pub id: SymbolId,
    pub kind: NodeKind,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub importance_score: f32, // Centrality / priority for LLM context packing
}

/// Semantic relationship edge between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Structural containment (e.g. Crate contains File, File contains Struct)
    Contains,
    /// Behavioral contract realization (e.g. TaskKernelService implements TaskKernel)
    Implements,
    /// Function execution invocation (e.g. A calls B)
    Calls,
    /// Type referencing (e.g. function returns Fact, struct field contains DomainError)
    References,
    /// Module or crate-level architectural dependency
    DependsOn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: SymbolId,
    pub to: SymbolId,
    pub kind: EdgeKind,
    pub label: Option<String>,
}

/// In-memory, high-performance Code Graph.
/// Serves as the authoritative semantic index for both LLM context compilation
/// and long-term agent memory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeGraph {
    pub nodes: HashMap<SymbolId, CodeNode>,
    pub outgoing: HashMap<SymbolId, Vec<GraphEdge>>,
    pub incoming: HashMap<SymbolId, Vec<GraphEdge>>,
    pub symbol_index: BTreeMap<String, Vec<SymbolId>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a node.
    pub fn add_node(&mut self, node: CodeNode) {
        let name = node.kind.display_name().to_string();
        self.symbol_index
            .entry(name)
            .or_default()
            .push(node.id.clone());
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add a directed semantic edge between two nodes.
    pub fn add_edge(
        &mut self,
        from: SymbolId,
        to: SymbolId,
        kind: EdgeKind,
        label: Option<String>,
    ) {
        let edge = GraphEdge {
            from: from.clone(),
            to: to.clone(),
            kind,
            label,
        };
        self.outgoing
            .entry(from.clone())
            .or_default()
            .push(edge.clone());
        self.incoming.entry(to).or_default().push(edge);
    }

    /// Find nodes matching a symbol name (e.g., "ContextBuilder", "TaskKernel").
    pub fn find_by_name(&self, name: &str) -> Vec<&CodeNode> {
        self.symbol_index
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Extract the k-hop neighborhood around a focal symbol.
    /// This is the foundational algorithm for Graph-RAG and LLM Subgraph extraction.
    pub fn extract_neighborhood(&self, center_id: &SymbolId, max_hops: usize) -> Vec<&CodeNode> {
        let mut visited = HashSet::new();
        let mut queue = vec![(center_id.clone(), 0)];
        visited.insert(center_id.clone());

        let mut result = Vec::new();

        while let Some((curr_id, hops)) = queue.pop() {
            if let Some(node) = self.nodes.get(&curr_id) {
                result.push(node);
            }

            if hops < max_hops {
                // Traverse outgoing edges
                if let Some(edges) = self.outgoing.get(&curr_id) {
                    for edge in edges {
                        if visited.insert(edge.to.clone()) {
                            queue.push((edge.to.clone(), hops + 1));
                        }
                    }
                }
                // Traverse incoming edges
                if let Some(edges) = self.incoming.get(&curr_id) {
                    for edge in edges {
                        if visited.insert(edge.from.clone()) {
                            queue.push((edge.from.clone(), hops + 1));
                        }
                    }
                }
            }
        }

        result
    }

    /// Returns list of traits implemented by a given struct, or structs implementing a trait.
    pub fn find_implementations(&self, trait_or_struct_id: &SymbolId) -> Vec<&CodeNode> {
        let mut impls = Vec::new();
        // Look up incoming Implements edges (e.g., Struct -(Implements)-> Trait)
        if let Some(edges) = self.incoming.get(trait_or_struct_id) {
            for edge in edges {
                if edge.kind == EdgeKind::Implements {
                    if let Some(node) = self.nodes.get(&edge.from) {
                        impls.push(node);
                    }
                }
            }
        }
        // Look up outgoing Implements edges
        if let Some(edges) = self.outgoing.get(trait_or_struct_id) {
            for edge in edges {
                if edge.kind == EdgeKind::Implements {
                    if let Some(node) = self.nodes.get(&edge.to) {
                        impls.push(node);
                    }
                }
            }
        }
        impls
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_graph_and_neighborhood() {
        let mut graph = CodeGraph::new();

        let trait_id = SymbolId::symbol_node("context_compiler", "ContextBuilder");
        let struct_id = SymbolId::symbol_node("task_kernel", "TaskKernelService");

        graph.add_node(CodeNode {
            id: trait_id.clone(),
            kind: NodeKind::Trait {
                name: "ContextBuilder".into(),
                visibility: "pub".into(),
                doc_comment: Some("Compiles context slice for LLM".into()),
            },
            file_path: Some("crates/context-compiler/src/traits.rs".into()),
            line_number: Some(10),
            importance_score: 1.0,
        });

        graph.add_node(CodeNode {
            id: struct_id.clone(),
            kind: NodeKind::Struct {
                name: "TaskKernelService".into(),
                visibility: "pub".into(),
                doc_comment: None,
            },
            file_path: Some("crates/task-kernel/src/service.rs".into()),
            line_number: Some(25),
            importance_score: 0.9,
        });

        // Add Implements edge
        graph.add_edge(
            struct_id.clone(),
            trait_id.clone(),
            EdgeKind::Implements,
            None,
        );

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // Test search
        let found = graph.find_by_name("ContextBuilder");
        assert_eq!(found.len(), 1);

        // Test implementation lookup
        let impls = graph.find_implementations(&trait_id);
        assert_eq!(impls.len(), 1);
        assert_eq!(impls[0].id, struct_id);

        // Test neighborhood extraction
        let neighborhood = graph.extract_neighborhood(&trait_id, 1);
        assert_eq!(neighborhood.len(), 2);
    }
}
