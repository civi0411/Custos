//! Canonical Port Registry (Interface Freeze — Sprint 0)
//!
//! Single place that names the 6 frozen ports. Traits are DEFINED here (or re-exported
//! from where they already live) and IMPLEMENTED by outer crates, so the dependency
//! arrow always points inward: `adapters/persistence/runtime -> core -> domain`.
//!
//! | # | Port         | Defined in                         | Implemented by                  | Owner  |
//! |---|--------------|------------------------------------|---------------------------------|--------|
//! | 1 | KernelPort   | `contracts::kernel`                | `TrustedKernel` (this crate)    | Vĩ     |
//! | 2 | StoragePort  | `contracts::storage` (+Cas,Outbox) | `custos-persistence`            | Trường |
//! | 3 | SandboxPort  | `contracts::sandbox`               | `custos-adapters/sandbox`       | Trường |
//! | 4 | ModelPort    | `custos-provider::port`            | `custos-adapters/providers`     | Vĩ     |
//! | 5 | WorkflowPort | `contracts::workflow`              | `custos-runtime/workflow`       | Vinh   |
//! | 6 | MemoryPort   | `contracts::memory`                | `custos-persistence` + runtime  | Vĩ     |
//!
//! NOTE: module is `contracts`, not `ports`, because `kernel::ports` is glob re-exported
//! at the crate root and a top-level `ports` module would shadow `crate::ports::TaskStore`.

pub mod kernel;
pub mod memory;
pub mod sandbox;
pub mod storage;
pub mod workflow;

pub use kernel::{KernelPort, TrustedKernel};
pub use memory::MemoryPort;
pub use sandbox::{SandboxCommand, SandboxPort};
pub use storage::{CasPort, OutboxEntry, OutboxPort, OutboxStatus, StoragePort};
pub use workflow::WorkflowPort;
