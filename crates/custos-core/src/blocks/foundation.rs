//! Foundation Shared Block (Core Layer)
//!
//! Cross-cutting ports, composite storage traits, skill contracts, and orchestration intelligence:
//! persistent stores (RunStore, SessionStore, WorkflowStore), process sandboxing, skill execution,
//! and immutable decision snapshot extraction.

pub use crate::contracts::sandbox::{SandboxCommand, SandboxPort};
pub use crate::contracts::skill::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, SkillRegistry,
};
pub use crate::contracts::storage::{
    OutboxEntry, OutboxPort, OutboxStatus, RunStore, SessionStore, StoragePort, TaskStore,
    WorkflowStore,
};
pub use crate::decision::DecisionSnapshotExtractor;
pub use crate::oi::{
    AdmissibilityEvaluator, AdmissibilityResult, HardFilters, JoinEvaluation, JoinEvaluator,
    JoinPolicy, PlanCompiler, WriteSetConflictChecker,
};
