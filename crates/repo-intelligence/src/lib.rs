//! Codebase Symbol & Graph Intelligence
//!
//! Provides symbol search, references, and dependency analysis across repositories.

pub mod scanner;

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
