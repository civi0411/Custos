//! Codebase Symbol & Graph Intelligence
//!
//! Provides symbol search, references, and dependency analysis across repositories.

pub mod coordinator;
pub mod graph;
pub mod llm_view;
pub mod scanner;

pub use coordinator::*;
pub use graph::*;
pub use llm_view::*;
pub use scanner::*;

pub struct RepoAnalyzer;
impl RepoAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for RepoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
