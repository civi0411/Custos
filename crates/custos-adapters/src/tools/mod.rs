//! Custos Native Built-in Tools
//!
//! Provides native tooling capabilities for Custos Domain Packs:
//! - Data Visualizer: HTML charts, Sankey, Treemap, Mermaid diagrams
//! - Persistent Memory: Project & global memory management under `.custos/memory`
//! - Document Processing: Word (.docx), PDF (.pdf), Excel (.xlsx)

pub mod checkpoint;
pub mod source_fetch;

pub use source_fetch::*;

#[cfg(all(feature = "mcp-builtins", feature = "autovisualiser"))]
pub mod visualizer {
    pub use crate::mcp::builtins::autovisualiser::*;
}

#[cfg(all(feature = "mcp-builtins", feature = "memory-server"))]
pub mod memory {
    pub use crate::mcp::builtins::MemoryServer;
}
