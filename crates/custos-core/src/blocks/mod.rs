//! Standard Architectural Blocks (Kim Tự Tháp - Core Contracts & Policies Layer)
//!
//! Decoupled contracts, ports, state machines, and policies that directly empower each Workbench:
//! - `engineering`: Workspace, file systems, terminal sessions, sandbox policies
//! - `research`: Notebook kernels, experiment execution, research draft ingress, evidence pipelines
//! - `quant`: Risk evaluation and pre-trade decision gates
//! - `copilot`: Kernel state machines, authority policies, human-in-the-loop approvals, harness contracts
//! - `foundation`: Composite persistence stores, skill discovery, sandbox execution, orchestration intelligence

pub mod copilot;
pub mod engineering;
pub mod foundation;
pub mod quant;
pub mod research;
