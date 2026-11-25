//! Custos Daemon Bootstrap Architecture (SADE R8 §6.8)
//!
//! Decomposes the monolithic `bootstrap_with_store_and_model` into
//! isolated, verifiable, single-responsibility lifecycle phases:
//! - `storage`: Startup crash recovery and ledger reconciliation
//! - `background`: Background OAuth sweeper and uncertain runs reconciler
//! - `kernel`: TrustedKernel and SovereignDeveloperAdapter initialization
//! - `workbenches`: Domain pack inventory discovery and GovernedAgentRuntime
//! - `coordinators`: Coordinators, TaskRuntime, and LocalApiDispatcher assembly

pub mod background;
pub mod coordinators;
pub mod kernel;
pub mod storage;
pub mod workbenches;

pub use background::spawn_background_sweepers;
pub use coordinators::{init_coordinators_and_api, CoordinatorServices};
pub use kernel::{init_kernel_services, KernelServices};
pub use storage::reconcile_storage_on_startup;
pub use workbenches::{init_workbenches_and_harness, WorkbenchHarnessSetup};
