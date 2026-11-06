//! Research Workbench Runtime Block
//!
//! Provides execution coordinators for interactive Python notebook kernels,
//! training and evaluation runners, checkpoint CAS relays, and remote workload admission.

pub use crate::compute::admission::WorkloadAdmissionCoordinator;
pub use crate::compute::evaluation::ModelEvaluationRunner;
pub use crate::compute::relay::CheckpointRelayCoordinator;
pub use crate::compute::remote::RemoteExecutionCoordinator;
pub use crate::compute::training::LocalTrainingRunner;
pub use crate::notebook::coordinator::PythonKernelCoordinator;
