//! Orchestration Intelligence (OI) Core Gateways (RFC 003 / RFC 004)
//!
//! Enforces static hard filters, admissibility gate, and deterministic plan compilation.

pub mod admissibility;
pub mod compiler;
pub mod hard_filters;
pub mod join_policy;
pub mod write_set;

pub use admissibility::{AdmissibilityEvaluator, AdmissibilityResult};
pub use compiler::PlanCompiler;
pub use hard_filters::HardFilters;
pub use join_policy::{JoinEvaluation, JoinEvaluator, JoinPolicy};
pub use write_set::WriteSetConflictChecker;
