//! Repository Query & Intelligence Outcomes
//!
//! Enforces honest confidence levels, coverage metrics, and missing item reasons.

use super::snapshot::RepoSnapshotRef;
use super::span::SourceSpan;
use serde::{Deserialize, Serialize};

/// Confidence level indicating how a code symbol or relationship was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    /// Pure regex, lexical, or FTS text search (unverified candidate).
    TextMatch,
    /// Tree-sitter / AST syntax grammar extraction without full symbol resolution.
    SyntacticCandidate,
    /// Compiler, semantic analyzer, or LSP authoritative resolution.
    ResolvedCompiler,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextMatch => write!(f, "text_match"),
            Self::SyntacticCandidate => write!(f, "syntactic_candidate"),
            Self::ResolvedCompiler => write!(f, "resolved_compiler"),
        }
    }
}

/// Explicit reason explaining why a file or symbol was excluded from results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    FileIgnored { path: String, pattern: String },
    ParserError { path: String, error: String },
    TokenBudgetExceeded { max_tokens: usize },
    StaleSource { path: String },
    AccessDenied { path: String, reason: String },
}

/// Quantitative measure of indexing completeness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub files_indexed: usize,
    pub total_files: usize,
    pub coverage_ratio: f32,
    pub has_unindexed_changes: bool,
}

/// Generic query outcome bundling results with provenance, confidence, and coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    pub data: T,
    pub snapshot: RepoSnapshotRef,
    pub confidence: ConfidenceLevel,
    pub missing_reasons: Vec<MissingReason>,
    pub coverage: CoverageMetrics,
}

/// Standard symbol locator in query responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMatch {
    pub name: String,
    pub kind: String,
    pub span: SourceSpan,
    pub confidence: ConfidenceLevel,
    pub container: Option<String>,
    pub signature: Option<String>,
    pub docstring: Option<String>,
}
