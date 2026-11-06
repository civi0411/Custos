//! Engineering Block (Domain Layer)
//!
//! Provides the core domain aggregates and value objects for the Engineering Workbench:
//! workspace lifecycle, file trees, diff hunks, terminal sessions, and repository spans.

pub use crate::workspace::artifact::{
    ArtifactHandoff, ArtifactKind, ArtifactLineageGraph, ArtifactRef, ArtifactSummary,
    LineageGraphEdge, LineageGraphNode,
};
pub use crate::workspace::repo::SourceSpan;
pub use crate::workspace::terminal::{
    TerminalOutputChunk, TerminalSession, TerminalSessionId, TerminalSessionStatus,
};
pub use crate::workspace::workspace::{
    DirtyManifest, ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceLineage,
    WorkspaceStatus,
};
pub use crate::workspace::workspace_files::{
    WorkspaceDiffEntry, WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff,
    WorkspaceFileEntry, WorkspaceFileTree, WriteWorkspaceFileParams,
};
