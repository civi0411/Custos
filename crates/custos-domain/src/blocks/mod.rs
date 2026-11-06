//! Standard Architectural Blocks (Kim Tự Tháp - Domain Layer)
//!
//! Decoupled domain blocks that directly define and empower each Workbench:
//! - `engineering`: Workspace, files, git spans, terminal sessions
//! - `research`: Notebooks, cells, evidence, claims, facts, synthesis, review, experiments
//! - `quant`: Alpha factors, backtests, risk metrics, orders, market bars
//! - `copilot`: Sessions, turns, assistant drafts, outbox items, permits, approvals
//! - `foundation`: Canonical IDs, errors, budgets, spans, packets

pub mod copilot;
pub mod engineering;
pub mod foundation;
pub mod quant;
pub mod research;
