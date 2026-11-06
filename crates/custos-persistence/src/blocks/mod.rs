//! Decoupled Persistence Building Blocks for Custos Workbenches
//!
//! Aligns SQLite persistence repositories directly with the 4 Workbenches:
//! - Engineering: Workspace & worktree persistence
//! - Research: Scientific claims, evidence, facts, experiments, and synthesis
//! - Quant: Alpha factors, tearsheets, risk audits, and execution blotters
//! - Copilot: Personal assistant, sessions, turns, and mutating outbox
//! - Foundation: Canonical tasks, runs, DAG workflows, decision snapshots, and credentials

pub mod copilot;
pub mod engineering;
pub mod foundation;
pub mod quant;
pub mod research;
