//! Decoupled Runtime Orchestration Blocks for Custos Workbenches
//!
//! Aligns agent execution loops, coordinators, and schedulers directly with the 4 Workbenches:
//! - Engineering: Workspace & Git coordinators, terminal session lifecycle, worktree leasing
//! - Research: Jupyter kernel coordinators, model training/evaluation runners, checkpoint relays
//! - Quant: Algorithmic trading execution conduits and institutional risk gate verifiers
//! - Copilot: Governed agent loops, cognitive deliberation, S1/S2 routing, and chat session management
//! - Foundation: Distributed task DAG runtime, monotonic clocks, pub/sub event bus, integration registry

pub mod copilot;
pub mod engineering;
pub mod foundation;
pub mod quant;
pub mod research;
