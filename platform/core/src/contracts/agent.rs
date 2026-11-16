//! Agent Driver / Autonomous Agent Port Contract (Port 7)
//!
//! Standardized contract governing both native CLI coding agents (Claude Code,
//! OpenAI Codex, Goose, OpenCode) and internal governed agent runtimes.
//!
//! Enforces Gate 4 (Native Bypass Assurance): External/native agents that execute
//! their own tools cannot fraudulently claim `custos-mediated` assurance.

pub use super::harness::{
    AgentRuntimePort, CostVisibility, HarnessDescriptor, HarnessExecutionResult, HarnessMaturity,
    HarnessProfile, RunObserver, ToolMediationLevel, WorktreeOwnership,
};

/// Canonical type alias for any autonomous agent driver conforming to Port 7.
pub type AgentDriver = dyn AgentRuntimePort;
