//! Artifact References & Storage Metadata
//!
//! Immutable references to content-addressed or file-system artifacts
//! generated throughout the task execution lifecycle.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    FileDiff,
    DiagnosticLog,
    VerificationReport,
    ExecutionTrace,
    Snapshot,
    ContinuationState,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub kind: ArtifactKind,
    pub hash: String,
    pub path: String,
    pub size_bytes: u64,
    pub mime: Option<String>,
}

impl ArtifactRef {
    pub fn new(
        id: String,
        kind: ArtifactKind,
        hash: String,
        path: String,
        size_bytes: u64,
    ) -> Self {
        Self {
            id,
            kind,
            hash,
            path,
            size_bytes,
            mime: None,
        }
    }
}
