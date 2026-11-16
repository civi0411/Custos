//! Source Code Span
//!
//! Pinpoints an exact range of source code lines/bytes with cryptographic digest.

use serde::{Deserialize, Serialize};

/// Precise, verifiable location of a code snippet within a repository snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    /// Path relative to the repository worktree root.
    pub path: String,
    /// 0-indexed byte offset start.
    pub start_byte: usize,
    /// 0-indexed byte offset end.
    pub end_byte: usize,
    /// 1-indexed starting line number.
    pub start_line: usize,
    /// 1-indexed ending line number.
    pub end_line: usize,
    /// SHA-256 digest of the exact source text inside this span.
    pub source_digest: String,
}

impl SourceSpan {
    pub fn new(
        path: impl Into<String>,
        start_byte: usize,
        end_byte: usize,
        start_line: usize,
        end_line: usize,
        source_digest: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            start_byte,
            end_byte,
            start_line,
            end_line,
            source_digest: source_digest.into(),
        }
    }

    pub fn line_count(&self) -> usize {
        if self.end_line >= self.start_line {
            self.end_line - self.start_line + 1
        } else {
            0
        }
    }
}
