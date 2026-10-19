//! Custos Native Built-in Tools
//!
//! Provides native tooling capabilities for Custos Domain Packs:
//! - Data Visualizer: HTML charts, Sankey, Treemap, Mermaid diagrams
//! - Persistent Memory: Project & global memory management under `.custos/memory`
//! - Document Processing: Word (.docx), PDF (.pdf), Excel (.xlsx)

pub mod checkpoint;

#[cfg(feature = "autovisualiser")]
pub mod visualizer {
    pub use crate::mcp::core::autovisualiser::*;
}

#[cfg(feature = "memory-server")]
pub mod memory {
    pub use crate::mcp::core::MemoryServer;
}
