//! Runtime Topology Coordinators (RFC 004 §2A)
//!
//! Provides execution coordinators for topologies T0 through T6.

pub mod t0_direct;
pub mod t1_single_worker;
pub mod t2_staged_pipeline;
pub mod t3_read_fanout;
pub mod t4_worktree_integrator;
pub mod t6_repair_loop;

pub use t0_direct::T0DirectCoordinator;
pub use t1_single_worker::T1SingleWorkerCoordinator;
pub use t2_staged_pipeline::T2StagedPipelineCoordinator;
pub use t3_read_fanout::T3ReadFanOutCoordinator;
pub use t4_worktree_integrator::T4WorktreeCoordinator;
pub use t6_repair_loop::T6RepairLoopCoordinator;
