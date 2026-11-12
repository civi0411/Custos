//! Research Verifiers
//!
//! Independent oracles for verifying research papers, citations, and reproducibility.

pub mod alce;
pub mod citation_coverage_oracle;

pub use alce::AlceEntailmentEngine;
pub use citation_coverage_oracle::CitationCoverageOracle;
