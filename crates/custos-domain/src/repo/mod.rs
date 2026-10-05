//! Repository Intelligence Domain Model
//!
//! Provides core abstractions for worktree snapshots, source spans,
//! query results, and provenance metrics.

pub mod query;
pub mod snapshot;
pub mod span;

pub use query::{ConfidenceLevel, CoverageMetrics, MissingReason, QueryResult, SymbolMatch};
pub use snapshot::RepoSnapshotRef;
pub use span::SourceSpan;
