//! Decoupled Adapter Building Blocks for Custos Workbenches
//!
//! Aligns physical I/O and external adapters directly with the 4 Workbenches:
//! - Engineering: Local Git worktrees, native PTY terminal driver, OS sandbox containment
//! - Research: Jupyter/Python kernel IPC driver, remote cloud/GPU compute targets
//! - Quant: Market data streaming feeds and subscription coordinators
//! - Copilot: Model providers (cloud & local inference), CLI harnesses, MCP federation, and A2A protocol
//! - Foundation: Download managers, packages distribution, and model bridge descriptors

pub mod copilot;
pub mod engineering;
pub mod foundation;
pub mod quant;
pub mod research;
